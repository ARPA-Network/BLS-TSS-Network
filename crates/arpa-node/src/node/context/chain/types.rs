use crate::node::{
    context::types::GeneralContext,
    listener::{
        block::BlockListener, new_randomness_task::NewRandomnessTaskListener,
        post_commit_grouping::PostCommitGroupingListener, post_grouping::PostGroupingListener,
        pre_grouping::PreGroupingListener,
        randomness_signature_aggregation::RandomnessSignatureAggregationListener,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskListener, Listener,
    },
    queue::event_queue::EventQueue,
    scheduler::{fixed::SimpleFixedTaskScheduler, ListenerType, TaskScheduler, TaskType},
    subscriber::{
        block::BlockSubscriber, in_grouping::InGroupingSubscriber,
        post_grouping::PostGroupingSubscriber,
        post_success_grouping::PostSuccessGroupingSubscriber, pre_grouping::PreGroupingSubscriber,
        randomness_signature_aggregation::RandomnessSignatureAggregationSubscriber,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskSubscriber, Subscriber,
    },
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{
    ChainIdentity, GeneralChainIdentity, MockChainIdentity, RandomnessTask, SchedulerError,
    SchedulerResult,
};
use arpa_node_dal::{
    cache::{
        InMemoryBLSTasksQueue, InMemoryBlockInfoCache, InMemoryGroupInfoCache,
        InMemoryNodeInfoCache, InMemorySignatureResultCache, RandomnessResultCache,
    },
    ContextInfoUpdater, NodeInfoUpdater,
    {BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher},
};
use arpa_node_sqlite_db::{BLSTasksDBClient, GroupInfoDBClient, NodeInfoDBClient};
use async_trait::async_trait;
use log::error;
use std::{marker::PhantomData, sync::Arc};
use threshold_bls::group::PairingCurve;
use tokio::sync::RwLock;

use super::{Chain, ChainFetcher, ContextFetcher, MainChain, MainChainFetcher};

#[derive(Debug)]
pub struct GeneralMainChain<
    N: NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater,
    G: GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder<PC>,
    PC: PairingCurve,
> {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<I>>,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    committer_randomness_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
    c: PhantomData<PC>,
}

impl<PC: PairingCurve + Send + Sync + 'static>
    GeneralMainChain<
        InMemoryNodeInfoCache<PC>,
        InMemoryGroupInfoCache<PC>,
        InMemoryBLSTasksQueue<RandomnessTask>,
        MockChainIdentity,
        PC,
    >
{
    pub fn new(
        id: usize,
        description: String,
        chain_identity: MockChainIdentity,
        node_cache: InMemoryNodeInfoCache<PC>,
        group_cache: InMemoryGroupInfoCache<PC>,
        randomness_tasks_cache: InMemoryBLSTasksQueue<RandomnessTask>,
    ) -> Self {
        GeneralMainChain {
            id,
            description,
            chain_identity: Arc::new(RwLock::new(chain_identity)),
            block_cache: Arc::new(RwLock::new(InMemoryBlockInfoCache::new())),
            randomness_tasks_cache: Arc::new(RwLock::new(randomness_tasks_cache)),
            committer_randomness_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<RandomnessResultCache>::new(),
            )),
            node_cache: Arc::new(RwLock::new(node_cache)),
            group_cache: Arc::new(RwLock::new(group_cache)),
            c: PhantomData,
        }
    }
}

impl<PC: PairingCurve + Send + Sync + 'static>
    GeneralMainChain<
        NodeInfoDBClient<PC>,
        GroupInfoDBClient<PC>,
        BLSTasksDBClient<RandomnessTask, PC>,
        GeneralChainIdentity,
        PC,
    >
{
    pub fn new(
        id: usize,
        description: String,
        chain_identity: GeneralChainIdentity,
        node_cache: NodeInfoDBClient<PC>,
        group_cache: GroupInfoDBClient<PC>,
        randomness_tasks_cache: BLSTasksDBClient<RandomnessTask, PC>,
    ) -> Self {
        GeneralMainChain {
            id,
            description,
            chain_identity: Arc::new(RwLock::new(chain_identity)),
            block_cache: Arc::new(RwLock::new(InMemoryBlockInfoCache::new())),
            randomness_tasks_cache: Arc::new(RwLock::new(randomness_tasks_cache)),
            committer_randomness_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<RandomnessResultCache>::new(),
            )),
            node_cache: Arc::new(RwLock::new(node_cache)),
            group_cache: Arc::new(RwLock::new(group_cache)),
            c: PhantomData,
        }
    }
}

#[async_trait]
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > Chain for GeneralMainChain<N, G, T, I, PC>
{
    type BlockInfoCache = InMemoryBlockInfoCache;

    type RandomnessTasksQueue = T;

    type RandomnessResultCaches = InMemorySignatureResultCache<RandomnessResultCache>;

    type Context = GeneralContext<N, G, T, I, PC>;

    type ChainIdentity = I;

    async fn init_listener(
        &self,
        eq: Arc<RwLock<EventQueue>>,
        fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
        task_type: TaskType,
    ) -> SchedulerResult<()> {
        match task_type {
            TaskType::Listener(e) => match e {
                ListenerType::Block => {
                    let p_block = BlockListener::new(self.id(), self.get_chain_identity(), eq);

                    fs.write()
                        .await
                        .add_task(TaskType::Listener(ListenerType::Block), async move {
                            if let Err(e) = p_block.start().await {
                                error!("{:?}", e);
                            };
                        })
                }
                ListenerType::PreGrouping => {
                    let p_pre_grouping = PreGroupingListener::new(
                        self.get_chain_identity(),
                        self.get_group_cache(),
                        eq,
                    );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::PreGrouping),
                        async move {
                            if let Err(e) = p_pre_grouping.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
                ListenerType::PostCommitGrouping => {
                    let p_post_commit_grouping = PostCommitGroupingListener::new(
                        self.get_chain_identity(),
                        self.get_group_cache(),
                        eq,
                    );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::PostCommitGrouping),
                        async move {
                            if let Err(e) = p_post_commit_grouping.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
                ListenerType::PostGrouping => {
                    let p_post_grouping = PostGroupingListener::new(
                        self.get_block_cache(),
                        self.get_group_cache(),
                        eq,
                    );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::PostGrouping),
                        async move {
                            if let Err(e) = p_post_grouping.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
                ListenerType::NewRandomnessTask => {
                    let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

                    let p_new_randomness_task = NewRandomnessTaskListener::new(
                        self.id(),
                        id_address,
                        self.get_chain_identity(),
                        self.get_randomness_tasks_cache(),
                        eq,
                    );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::NewRandomnessTask),
                        async move {
                            if let Err(e) = p_new_randomness_task.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
                ListenerType::ReadyToHandleRandomnessTask => {
                    let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

                    let p_ready_to_handle_randomness_task =
                        ReadyToHandleRandomnessTaskListener::new(
                            self.id(),
                            id_address,
                            self.get_chain_identity(),
                            self.get_block_cache(),
                            self.get_group_cache(),
                            self.get_randomness_tasks_cache(),
                            eq,
                        );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::ReadyToHandleRandomnessTask),
                        async move {
                            if let Err(e) = p_ready_to_handle_randomness_task.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
                ListenerType::RandomnessSignatureAggregation => {
                    let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

                    let p_randomness_signature_aggregation =
                        RandomnessSignatureAggregationListener::new(
                            self.id(),
                            id_address,
                            self.get_block_cache(),
                            self.get_group_cache(),
                            self.get_randomness_result_cache(),
                            eq,
                        );

                    fs.write().await.add_task(
                        TaskType::Listener(ListenerType::RandomnessSignatureAggregation),
                        async move {
                            if let Err(e) = p_randomness_signature_aggregation.start().await {
                                error!("{:?}", e);
                            };
                        },
                    )
                }
            },
            _ => Err(SchedulerError::TaskNotFound),
        }
    }

    async fn init_listeners(
        &self,
        context: &GeneralContext<N, G, T, I, PC>,
    ) -> SchedulerResult<()> {
        match &context.get_config().listeners {
            Some(listeners) => {
                for listener in listeners {
                    self.init_listener(
                        context.get_event_queue(),
                        context.get_fixed_task_handler(),
                        TaskType::Listener(listener.clone()),
                    )
                    .await?;
                }
            }
            None => {
                self.init_block_listeners(context).await?;

                self.init_dkg_listeners(context).await?;

                self.init_randomness_listeners(context).await?;
            }
        }

        Ok(())
    }

    async fn init_subscribers(&self, context: &GeneralContext<N, G, T, I, PC>) {
        self.init_block_subscribers(context).await;

        self.init_dkg_subscribers(context).await;

        self.init_randomness_subscribers(context).await;
    }
}

#[async_trait]
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > MainChain for GeneralMainChain<N, G, T, I, PC>
{
    type NodeInfoCache = N;

    type GroupInfoCache = G;

    async fn init_block_listeners(&self, context: &Self::Context) -> SchedulerResult<()> {
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::Block),
        )
        .await?;

        Ok(())
    }

    async fn init_dkg_listeners(&self, context: &Self::Context) -> SchedulerResult<()> {
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::PreGrouping),
        )
        .await?;
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::PostCommitGrouping),
        )
        .await?;
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::PostGrouping),
        )
        .await?;

        Ok(())
    }

    async fn init_randomness_listeners(&self, context: &Self::Context) -> SchedulerResult<()> {
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::NewRandomnessTask),
        )
        .await?;
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::ReadyToHandleRandomnessTask),
        )
        .await?;
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            TaskType::Listener(ListenerType::RandomnessSignatureAggregation),
        )
        .await?;

        Ok(())
    }

    async fn init_block_subscribers(&self, context: &Self::Context) {
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe().await;
    }

    async fn init_dkg_subscribers(&self, context: &Self::Context) {
        let s_pre_grouping =
            PreGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_pre_grouping.subscribe().await;

        let s_in_grouping = InGroupingSubscriber::new(
            self.get_chain_identity(),
            self.get_node_cache(),
            self.get_group_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_in_grouping.subscribe().await;

        let s_post_success_grouping =
            PostSuccessGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_post_success_grouping.subscribe().await;

        let s_post_grouping = PostGroupingSubscriber::new(
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_post_grouping.subscribe().await;
    }

    async fn init_randomness_subscribers(&self, context: &Self::Context) {
        let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::<
            G,
            T,
            InMemorySignatureResultCache<RandomnessResultCache>,
            PC,
        >::new(
            self.id(),
            id_address,
            self.get_group_cache(),
            self.get_randomness_tasks_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_randomness_task.subscribe().await;

        let s_randomness_signature_aggregation =
            RandomnessSignatureAggregationSubscriber::<I, PC>::new(
                self.id(),
                id_address,
                self.get_chain_identity(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_randomness_signature_aggregation.subscribe().await;
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > ChainFetcher<GeneralMainChain<N, G, T, I, PC>> for GeneralMainChain<N, G, T, I, PC>
{
    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as Chain>::ChainIdentity>> {
        self.chain_identity.clone()
    }

    fn get_block_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as Chain>::BlockInfoCache>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as Chain>::RandomnessTasksQueue>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as Chain>::RandomnessResultCaches>> {
        self.committer_randomness_result_cache.clone()
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
            + Clone
            + std::fmt::Debug
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
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder<PC>
            + ChainProviderBuilder
            + std::fmt::Debug
            + Clone
            + Sync
            + Send
            + 'static,
        PC: PairingCurve + std::fmt::Debug + Clone + Sync + Send + 'static,
    > MainChainFetcher<GeneralMainChain<N, G, T, I, PC>> for GeneralMainChain<N, G, T, I, PC>
{
    fn get_node_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as MainChain>::NodeInfoCache>> {
        self.node_cache.clone()
    }

    fn get_group_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I, PC> as MainChain>::GroupInfoCache>> {
        self.group_cache.clone()
    }
}
