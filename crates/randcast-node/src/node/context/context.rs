use super::chain::{
    Chain, ChainFetcher, InMemoryAdapterChain, InMemoryMainChain, MainChainFetcher,
};
use crate::node::{
    committer::committer_server,
    dao::{api::NodeInfoFetcher, types::ChainIdentity},
    error::errors::{NodeError, NodeResult},
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler,
        fixed::{FixedTaskScheduler, SimpleFixedTaskScheduler},
    },
};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use threshold_bls::curve::bls12381::{Scalar, G1};

pub trait Context {
    type MainChain;

    type AdapterChain;

    fn deploy(self) -> ContextHandle;
}

impl Context for InMemoryContext {
    type MainChain = InMemoryMainChain;

    type AdapterChain = InMemoryAdapterChain;

    fn deploy(self) -> ContextHandle {
        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self
            .get_main_chain()
            .get_node_cache()
            .read()
            .get_node_rpc_endpoint()
            .to_string();

        let context = Arc::new(RwLock::new(self));

        f_ts.write()
            .start_committer_server(rpc_endpoint, context.clone());

        let ts = context.read().get_dynamic_task_handler();

        ContextHandle { ts }
    }
}

pub struct InMemoryContext {
    main_chain: InMemoryMainChain,
    adapter_chains: HashMap<usize, InMemoryAdapterChain>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
}

impl InMemoryContext {
    pub fn new(
        main_chain_identity: ChainIdentity,
        node_rpc_endpoint: String,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> Self {
        let main_chain = InMemoryMainChain::new(
            0,
            "main".to_string(),
            main_chain_identity,
            node_rpc_endpoint,
            dkg_private_key,
            dkg_public_key,
        );

        let context = InMemoryContext {
            main_chain,
            adapter_chains: HashMap::new(),
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
        };

        context.get_main_chain().init_components(&context);

        context
    }

    pub fn add_adapter_chain(&mut self, adapter_chain: InMemoryAdapterChain) -> NodeResult<()> {
        if self.adapter_chains.contains_key(&adapter_chain.id()) {
            return Err(NodeError::RepeatedChainId);
        }

        adapter_chain.init_components(self);

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

impl ContextFetcher<InMemoryContext> for InMemoryContext {
    fn contains_chain(&self, index: usize) -> bool {
        self.adapter_chains.contains_key(&index)
    }

    fn get_adapter_chain(
        &self,
        index: usize,
    ) -> Option<&<InMemoryContext as Context>::AdapterChain> {
        self.adapter_chains.get(&index)
    }

    fn get_main_chain(&self) -> &<InMemoryContext as Context>::MainChain {
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

trait CommitterServerStarter {
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<InMemoryContext>>,
    );
}

impl CommitterServerStarter for SimpleFixedTaskScheduler {
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<InMemoryContext>>,
    ) {
        self.add_task(async move {
            if let Err(e) = committer_server::start_committer_server(rpc_endpoint, context).await {
                println!("{:?}", e);
            };
        });
    }
}
