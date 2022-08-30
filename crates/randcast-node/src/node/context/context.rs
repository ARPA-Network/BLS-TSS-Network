use super::chain::{Chain, ChainFetcher, GeneralAdapterChain, GeneralMainChain, MainChainFetcher};
use crate::node::{
    committer::committer_server,
    dal::{
        api::{
            BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher,
        },
        types::RandomnessTask,
    },
    error::errors::{NodeError, NodeResult},
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler,
        fixed::{FixedTaskScheduler, SimpleFixedTaskScheduler},
    },
};
use async_trait::async_trait;
use log::error;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

pub trait Context {
    type MainChain;

    type AdapterChain;

    fn deploy(self) -> ContextHandle;
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
    > Context for GeneralContext<N, G, T>
{
    type MainChain = GeneralMainChain<N, G, T>;

    type AdapterChain = GeneralAdapterChain<N, G, T>;

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

pub struct GeneralContext<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
> {
    main_chain: GeneralMainChain<N, G, T>,
    adapter_chains: HashMap<usize, GeneralAdapterChain<N, G, T>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
    > GeneralContext<N, G, T>
{
    pub fn new(main_chain: GeneralMainChain<N, G, T>) -> Self {
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
        adapter_chain: GeneralAdapterChain<N, G, T>,
    ) -> NodeResult<()> {
        if self.adapter_chains.contains_key(&adapter_chain.id()) {
            return Err(NodeError::RepeatedChainId);
        }

        self.adapter_chains
            .insert(adapter_chain.id(), adapter_chain);

        Ok(())
    }
}

pub trait ContextFetcher<T: Context> {
    fn contains_chain(&self, index: usize) -> bool;

    fn get_adapter_chain(&self, index: usize) -> Option<&T::AdapterChain>;

    fn get_main_chain(&self) -> &T::MainChain;

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>>;

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>>;

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>>;
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
    > ContextFetcher<GeneralContext<N, G, T>> for GeneralContext<N, G, T>
{
    fn contains_chain(&self, index: usize) -> bool {
        self.adapter_chains.contains_key(&index)
    }

    fn get_adapter_chain(
        &self,
        index: usize,
    ) -> Option<&<GeneralContext<N, G, T> as Context>::AdapterChain> {
        self.adapter_chains.get(&index)
    }

    fn get_main_chain(&self) -> &<GeneralContext<N, G, T> as Context>::MainChain {
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

#[async_trait]
pub trait TaskWaiter {
    async fn wait_task(&self);
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

trait CommitterServerStarter<T: Context> {
    fn start_committer_server(&mut self, rpc_endpoint: String, context: Arc<RwLock<T>>);
}

impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
    > CommitterServerStarter<GeneralContext<N, G, T>> for SimpleFixedTaskScheduler
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T>>>,
    ) {
        self.add_task(async move {
            if let Err(e) = committer_server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        });
    }
}
