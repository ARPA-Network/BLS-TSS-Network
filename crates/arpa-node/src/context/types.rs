use super::{
    chain::{types::GeneralMainChain, Chain, RelayedChain},
    BLSTasksHandler, BlockInfoHandler, ChainIdentityHandler, ChainIdentityHandlerType,
    CommitterServerStarter, Context, ContextFetcher, GroupInfoHandler, ManagementServerStarter,
    NodeInfoHandler, RelayedChainType, SignatureResultCacheHandler, TaskWaiter,
};
use crate::{
    committer::server as committer_server,
    error::{NodeError, NodeResult},
    management::server as management_server,
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler, TaskScheduler,
    },
};
use arpa_core::{
    Config, GeneralMainChainIdentity, GeneralRelayedChainIdentity, RandomnessTask, RpcServerType,
    SchedulerResult, TaskType, DEFAULT_DYNAMIC_TASK_CLEANER_INTERVAL_MILLIS,
};
use arpa_dal::cache::RandomnessResultCache;
use async_trait::async_trait;
use log::error;
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct GeneralContext<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    main_chain: GeneralMainChain<PC, S>,
    relayed_chains: HashMap<usize, RelayedChainType<PC, S>>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
    config: Config,
}

impl<
        PC: Curve,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > GeneralContext<PC, S>
{
    pub fn new(main_chain: GeneralMainChain<PC, S>, config: Config) -> Self {
        GeneralContext {
            main_chain,
            relayed_chains: HashMap::new(),
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
            config,
        }
    }
}

impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > Context<PC, S> for GeneralContext<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    type MainChain = GeneralMainChain<PC, S>;

    fn get_main_chain(&self) -> &<GeneralContext<PC, S> as Context<PC, S>>::MainChain {
        &self.main_chain
    }

    fn contains_relayed_chain(&self, index: usize) -> bool {
        self.relayed_chains.contains_key(&index)
    }

    fn get_relayed_chain(
        &self,
        index: usize,
    ) -> Option<
        &Box<
            dyn RelayedChain<
                    PC,
                    S,
                    NodeInfoCache = Box<dyn NodeInfoHandler<PC>>,
                    GroupInfoCache = Box<dyn GroupInfoHandler<PC>>,
                    BlockInfoCache = Box<dyn BlockInfoHandler>,
                    RandomnessTasksQueue = Box<dyn BLSTasksHandler<RandomnessTask>>,
                    RandomnessResultCaches = Box<
                        dyn SignatureResultCacheHandler<RandomnessResultCache>,
                    >,
                    ChainIdentity = ChainIdentityHandlerType<PC>,
                > + Sync
                + Send,
        >,
    > {
        self.relayed_chains.get(&index)
    }

    fn add_relayed_chain(
        &mut self,
        relayed_chain: Box<
            dyn RelayedChain<
                    PC,
                    S,
                    NodeInfoCache = Box<dyn NodeInfoHandler<PC>>,
                    GroupInfoCache = Box<dyn GroupInfoHandler<PC>>,
                    BlockInfoCache = Box<dyn BlockInfoHandler>,
                    RandomnessTasksQueue = Box<dyn BLSTasksHandler<RandomnessTask>>,
                    RandomnessResultCaches = Box<
                        dyn SignatureResultCacheHandler<RandomnessResultCache>,
                    >,
                    ChainIdentity = ChainIdentityHandlerType<PC>,
                > + Sync
                + Send,
        >,
    ) -> NodeResult<()> {
        let index = relayed_chain.id();

        if self.relayed_chains.contains_key(&index) {
            return Err(NodeError::RepeatedChainId);
        }

        self.relayed_chains.insert(index, relayed_chain);

        Ok(())
    }

    async fn deploy(self) -> SchedulerResult<ContextHandle> {
        self.get_main_chain().init_components(&self).await?;
        for relayed_chain in self.relayed_chains.values() {
            relayed_chain.init_components(&self).await?;
        }

        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self.config.get_node_committer_rpc_endpoint().to_string();

        let node_management_rpc_endpoint =
            self.config.get_node_management_rpc_endpoint().to_string();

        let context = Arc::new(RwLock::new(self));

        f_ts.write()
            .await
            .start_committer_server(rpc_endpoint, context.clone())?;

        f_ts.write()
            .await
            .start_management_server(node_management_rpc_endpoint, context.clone())?;

        let ts = context.read().await.get_dynamic_task_handler();

        Ok(ContextHandle { ts })
    }
}

impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > ContextFetcher for GeneralContext<PC, S>
{
    fn get_supported_relayed_chains(&self) -> Vec<usize> {
        self.relayed_chains.keys().cloned().collect()
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

    fn get_config(&self) -> &Config {
        &self.config
    }
}
pub struct ContextHandle {
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
}

#[async_trait]
impl TaskWaiter for ContextHandle {
    async fn wait_task(&self) {
        loop {
            while !self.ts.read().await.dynamic_tasks.is_empty() {
                let (task_recv, task_monitor) = self.ts.write().await.dynamic_tasks.pop().unwrap();

                let _ = task_recv.await;

                if let Some(monitor) = task_monitor {
                    monitor.abort();
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(
                DEFAULT_DYNAMIC_TASK_CLEANER_INTERVAL_MILLIS,
            ))
            .await;
        }
    }
}

impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > CommitterServerStarter<GeneralContext<PC, S>, PC, S> for SimpleFixedTaskScheduler
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<PC, S>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Committer), async move {
            if let Err(e) = committer_server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        })
    }
}

impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > ManagementServerStarter<GeneralContext<PC, S>, PC, S> for SimpleFixedTaskScheduler
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    fn start_management_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<PC, S>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Management), async move {
            if let Err(e) = management_server::start_management_server(rpc_endpoint, context).await
            {
                error!("{:?}", e);
            };
        })
    }
}

impl<PC: Curve + 'static> ChainIdentityHandler<PC> for GeneralMainChainIdentity {}
impl<PC: Curve + 'static> ChainIdentityHandler<PC> for GeneralRelayedChainIdentity {}
