use super::{
    chain::{
        types::{GeneralAdapterChain, GeneralMainChain},
        Chain, ChainFetcher, MainChainFetcher,
    },
    CommitterServerStarter, Context, ContextFetcher, TaskWaiter,
};
use crate::node::{
    committer::server,
    contract_client::{
        adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
        coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
    },
    dal::{
        types::RandomnessTask,
        ChainIdentity,
        {BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher},
    },
    error::{NodeError, NodeResult},
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler, TaskScheduler,
    },
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub node_rpc_endpoint: String,
    pub provider_endpoint: String,
    pub controller_address: String,
    // Data file for persistence
    pub data_path: Option<String>,
    pub account: Account,
    pub adapters: Vec<Adapter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adapter {
    pub id: usize,
    pub name: String,
    pub endpoint: String,
    pub account: Account,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub hdwallet: Option<HDWallet>,
    pub keystore: Option<Keystore>,
    // not recommended
    pub private_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keystore {
    pub path: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HDWallet {
    pub mnemonic: String,
    pub path: Option<String>,
    pub index: u32,
    pub passphrase: Option<String>,
}

pub struct GeneralContext<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder,
> {
    main_chain: GeneralMainChain<N, G, T, I>,
    adapter_chains: HashMap<usize, GeneralAdapterChain<N, G, T, I>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > GeneralContext<N, G, T, I>
{
    pub fn new(main_chain: GeneralMainChain<N, G, T, I>) -> Self {
        GeneralContext {
            main_chain,
            adapter_chains: HashMap::new(),
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
        }
    }

    pub fn add_adapter_chain(
        &mut self,
        adapter_chain: GeneralAdapterChain<N, G, T, I>,
    ) -> NodeResult<()> {
        if self.adapter_chains.contains_key(&adapter_chain.id()) {
            return Err(NodeError::RepeatedChainId);
        }

        self.adapter_chains
            .insert(adapter_chain.id(), adapter_chain);

        Ok(())
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > Context for GeneralContext<N, G, T, I>
{
    type MainChain = GeneralMainChain<N, G, T, I>;

    type AdapterChain = GeneralAdapterChain<N, G, T, I>;

    fn deploy(self) -> ContextHandle {
        self.get_main_chain().init_components(&self);

        for adapter_chain in self.adapter_chains.values() {
            adapter_chain.init_components(&self);
        }

        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .get_node_rpc_endpoint()
            .unwrap()
            .to_string();

        let context = Arc::new(RwLock::new(self));

        f_ts.write()
            .start_committer_server(rpc_endpoint, context.clone());

        let ts = context.read().get_dynamic_task_handler();

        ContextHandle { ts }
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > ContextFetcher<GeneralContext<N, G, T, I>> for GeneralContext<N, G, T, I>
{
    fn contains_chain(&self, index: usize) -> bool {
        self.adapter_chains.contains_key(&index)
    }

    fn get_adapter_chain(
        &self,
        index: usize,
    ) -> Option<&<GeneralContext<N, G, T, I> as Context>::AdapterChain> {
        self.adapter_chains.get(&index)
    }

    fn get_main_chain(&self) -> &<GeneralContext<N, G, T, I> as Context>::MainChain {
        &self.main_chain
    }

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>> {
        self.f_ts.clone()
    }

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>> {
        self.ts.clone()
    }

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>> {
        self.eq.clone()
    }
}

pub struct ContextHandle {
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

#[async_trait]
impl TaskWaiter for ContextHandle {
    async fn wait_task(&self) {
        loop {
            while !self.ts.read().dynamic_tasks.is_empty() {
                let (task_recv, task_monitor) = self.ts.write().dynamic_tasks.pop().unwrap();

                let _ = task_recv.await;

                if let Some(monitor) = task_monitor {
                    monitor.abort();
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    }
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > CommitterServerStarter<GeneralContext<N, G, T, I>> for SimpleFixedTaskScheduler
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, I>>>,
    ) {
        self.add_task(async move {
            if let Err(e) = server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        });
    }
}
