use super::{
    chain::{types::GeneralMainChain, Chain},
    CommitterServerStarter, Context, ContextFetcher, ManagementServerStarter, TaskWaiter,
};
use crate::{
    committer::server as committer_server,
    management::server as management_server,
    queue::event_queue::EventQueue,
    scheduler::{
        dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler, TaskScheduler,
    },
};
use arpa_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_core::{
    ChainIdentity, Config, RandomnessTask, RpcServerType, SchedulerResult, TaskType,
    DEFAULT_DYNAMIC_TASK_CLEANER_INTERVAL_MILLIS,
};
use arpa_dal::{
    cache::RandomnessResultCache, BLSTasksFetcher, BLSTasksUpdater, ContextInfoUpdater,
    GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher, NodeInfoUpdater,
    SignatureResultCacheFetcher, SignatureResultCacheUpdater,
};
use async_trait::async_trait;
use log::error;
use std::sync::Arc;
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct GeneralContext<
    N: NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater,
    G: GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    C: SignatureResultCacheFetcher<RandomnessResultCache>
        + SignatureResultCacheUpdater<RandomnessResultCache>,
    I: ChainIdentity + ControllerClientBuilder<PC> + CoordinatorClientBuilder + AdapterClientBuilder,
    PC: PairingCurve,
> {
    main_chain: GeneralMainChain<N, G, T, C, I, PC>,
    eq: Arc<RwLock<EventQueue>>,
    ts: Arc<RwLock<SimpleDynamicTaskScheduler>>,
    f_ts: Arc<RwLock<SimpleFixedTaskScheduler>>,
    config: Config,
}

impl<
        N: NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater + Sync + Send + 'static,
        G: GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
        PC: PairingCurve,
    > GeneralContext<N, G, T, C, I, PC>
{
    pub fn new(main_chain: GeneralMainChain<N, G, T, C, I, PC>, config: Config) -> Self {
        GeneralContext {
            main_chain,
            eq: Arc::new(RwLock::new(EventQueue::new())),
            ts: Arc::new(RwLock::new(SimpleDynamicTaskScheduler::new())),
            f_ts: Arc::new(RwLock::new(SimpleFixedTaskScheduler::new())),
            config,
        }
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > Context for GeneralContext<N, G, T, C, I, PC>
{
    type MainChain = GeneralMainChain<N, G, T, C, I, PC>;

    async fn deploy(self) -> SchedulerResult<ContextHandle> {
        self.get_main_chain().init_components(&self).await?;

        let f_ts = self.get_fixed_task_handler();

        let rpc_endpoint = self.config.node_committer_rpc_endpoint.clone();

        let node_management_rpc_endpoint = self.config.node_management_rpc_endpoint.clone();

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
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ContextFetcher<GeneralContext<N, G, T, C, I, PC>> for GeneralContext<N, G, T, C, I, PC>
{
    fn get_main_chain(&self) -> &<GeneralContext<N, G, T, C, I, PC> as Context>::MainChain {
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
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > CommitterServerStarter<GeneralContext<N, G, T, C, I, PC>> for SimpleFixedTaskScheduler
{
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, C, I, PC>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Committer), async move {
            if let Err(e) = committer_server::start_committer_server(rpc_endpoint, context).await {
                error!("{:?}", e);
            };
        })
    }
}

impl<
        N: NodeInfoFetcher<PC>
            + NodeInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        G: GroupInfoFetcher<PC>
            + GroupInfoUpdater<PC>
            + ContextInfoUpdater
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        T: BLSTasksFetcher<RandomnessTask>
            + BLSTasksUpdater<RandomnessTask>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        C: SignatureResultCacheFetcher<RandomnessResultCache>
            + SignatureResultCacheUpdater<RandomnessResultCache>
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        I: ChainIdentity
            + ControllerClientBuilder<PC>
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ManagementServerStarter<GeneralContext<N, G, T, C, I, PC>> for SimpleFixedTaskScheduler
{
    fn start_management_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<GeneralContext<N, G, T, C, I, PC>>>,
    ) -> SchedulerResult<()> {
        self.add_task(TaskType::RpcServer(RpcServerType::Management), async move {
            if let Err(e) = management_server::start_management_server(rpc_endpoint, context).await
            {
                error!("{:?}", e);
            };
        })
    }
}
