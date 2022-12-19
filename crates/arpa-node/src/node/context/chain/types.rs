use crate::node::{
    context::types::GeneralContext,
    listener::{
        block::BlockListener,
        group_relay_confirmation_signature_aggregation::GroupRelayConfirmationSignatureAggregationListener,
        group_relay_signature_aggregation::GroupRelaySignatureAggregationListener,
        new_group_relay_confirmation_task::NewGroupRelayConfirmationTaskListener,
        new_group_relay_task::NewGroupRelayTaskListener,
        new_randomness_task::NewRandomnessTaskListener,
        post_commit_grouping::PostCommitGroupingListener, post_grouping::PostGroupingListener,
        pre_grouping::PreGroupingListener,
        randomness_signature_aggregation::RandomnessSignatureAggregationListener,
        ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTaskListener,
        ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTaskListener,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskListener, Listener,
    },
    scheduler::TaskScheduler,
    subscriber::{
        block::BlockSubscriber,
        group_relay_confirmation_signature_aggregation::GroupRelayConfirmationSignatureAggregationSubscriber,
        group_relay_signature_aggregation::GroupRelaySignatureAggregationSubscriber,
        in_grouping::InGroupingSubscriber, post_grouping::PostGroupingSubscriber,
        post_success_grouping::PostSuccessGroupingSubscriber, pre_grouping::PreGroupingSubscriber,
        randomness_signature_aggregation::RandomnessSignatureAggregationSubscriber,
        ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTaskSubscriber,
        ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTaskSubscriber,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskSubscriber, Subscriber,
    },
};
use arpa_node_contract_client::{
    adapter::AdapterClientBuilder, controller::ControllerClientBuilder,
    coordinator::CoordinatorClientBuilder, provider::ChainProviderBuilder,
};
use arpa_node_core::{
    ChainIdentity, GeneralChainIdentity, GroupRelayConfirmationTask, GroupRelayTask,
    MockChainIdentity, RandomnessTask,
};
use arpa_node_dal::{
    cache::{
        GroupRelayConfirmationResultCache, GroupRelayResultCache, InMemoryBLSTasksQueue,
        InMemoryBlockInfoCache, InMemoryGroupInfoCache, InMemoryNodeInfoCache,
        InMemorySignatureResultCache, RandomnessResultCache,
    },
    {BLSTasksFetcher, BLSTasksUpdater, GroupInfoFetcher, GroupInfoUpdater, NodeInfoFetcher},
};
use arpa_node_sqlite_db::{BLSTasksDBClient, GroupInfoDBClient, NodeInfoDBClient};
use async_trait::async_trait;
use log::error;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::RwLock;

use super::{
    AdapterChain, AdapterChainFetcher, Chain, ChainFetcher, ContextFetcher, MainChain,
    MainChainFetcher,
};

pub struct GeneralAdapterChain<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder,
> {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<I>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    group_relay_confirmation_tasks_cache:
        Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>>,
    committer_randomness_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
    committer_group_relay_confirmation_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>>,
    n: PhantomData<N>,
    g: PhantomData<G>,
}

#[async_trait]
impl<
        N: NodeInfoFetcher + Sync + Send + 'static,
        G: GroupInfoFetcher + GroupInfoUpdater + Sync + Send + 'static,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask> + Sync + Send + 'static,
        I: ChainIdentity
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ControllerClientBuilder
            + CoordinatorClientBuilder
            + AdapterClientBuilder
            + ChainProviderBuilder
            + Sync
            + Send
            + 'static,
    > Chain for GeneralAdapterChain<N, G, T, I>
{
    type BlockInfoCache = InMemoryBlockInfoCache;

    type RandomnessTasksQueue = T;

    type RandomnessResultCaches = InMemorySignatureResultCache<RandomnessResultCache>;

    type Context = GeneralContext<N, G, T, I>;

    type ChainIdentity = I;

    async fn init_listeners(&self, context: &GeneralContext<N, G, T, I>) {
        self.init_block_listeners(context).await;

        self.init_randomness_listeners(context).await;

        self.init_group_relay_confirmation_listeners(context).await;
    }

    async fn init_subscribers(&self, context: &GeneralContext<N, G, T, I>) {
        self.init_block_subscribers(context).await;

        self.init_randomness_subscribers(context).await;

        self.init_group_relay_subscribers(context).await;

        self.init_group_relay_confirmation_subscribers(context)
            .await;
    }
}

#[async_trait]
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
    > AdapterChain for GeneralAdapterChain<N, G, T, I>
{
    type GroupRelayConfirmationTasksQueue = InMemoryBLSTasksQueue<GroupRelayConfirmationTask>;

    type GroupRelayConfirmationResultCaches =
        InMemorySignatureResultCache<GroupRelayConfirmationResultCache>;

    async fn init_block_listeners(&self, context: &Self::Context) {
        let p_block = BlockListener::new(
            self.id(),
            self.get_chain_identity(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_block.start().await {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_randomness_listeners(&self, context: &Self::Context) {
        let id_address = context
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()
            .unwrap();

        let p_new_randomness_task = NewRandomnessTaskListener::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_new_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskListener::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            self.get_block_cache(),
            context.get_main_chain().get_group_cache(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_randomness_signature_aggregation = RandomnessSignatureAggregationListener::new(
            self.id(),
            id_address,
            context.get_main_chain().get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_randomness_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_group_relay_confirmation_listeners(&self, context: &Self::Context) {
        let id_address = context
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()
            .unwrap();

        let p_new_group_relay_confirmation_task = NewGroupRelayConfirmationTaskListener::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            self.get_group_relay_confirmation_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_new_group_relay_confirmation_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_group_relay_confirmation_task =
            ReadyToHandleGroupRelayConfirmationTaskListener::new(
                self.id(),
                self.get_block_cache(),
                context.get_main_chain().get_group_cache(),
                self.get_group_relay_confirmation_tasks_cache(),
                context.get_event_queue(),
            );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_group_relay_confirmation_task
                    .start()
                    .await
                {
                    error!("{:?}", e);
                };
            });

        let p_group_relay_confirmation_signature_aggregation =
            GroupRelayConfirmationSignatureAggregationListener::new(
                self.id(),
                id_address,
                context.get_main_chain().get_group_cache(),
                self.get_group_relay_confirmation_result_cache(),
                context.get_event_queue(),
            );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_group_relay_confirmation_signature_aggregation
                    .start()
                    .await
                {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_block_subscribers(&self, context: &Self::Context) {
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe().await;
    }

    async fn init_randomness_subscribers(&self, context: &Self::Context) {
        let id_address = context
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()
            .unwrap();

        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::new(
            self.id(),
            id_address,
            context.get_main_chain().get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_randomness_task.subscribe().await;

        let s_randomness_signature_aggregation = RandomnessSignatureAggregationSubscriber::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_randomness_signature_aggregation.subscribe().await;
    }

    async fn init_group_relay_subscribers(&self, context: &Self::Context) {
        let id_address = context
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()
            .unwrap();

        let s_group_relay_signature_aggregation = GroupRelaySignatureAggregationSubscriber::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_group_relay_signature_aggregation.subscribe().await;
    }

    async fn init_group_relay_confirmation_subscribers(&self, context: &Self::Context) {
        let id_address = context
            .get_main_chain()
            .get_node_cache()
            .read()
            .await
            .get_id_address()
            .unwrap();

        let s_ready_to_handle_group_relay_confirmation_task =
            ReadyToHandleGroupRelayConfirmationTaskSubscriber::new(
                self.id(),
                context.get_main_chain().get_chain_identity(),
                self.get_chain_identity(),
                context.get_main_chain().get_group_cache(),
                self.get_group_relay_confirmation_result_cache(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_ready_to_handle_group_relay_confirmation_task
            .subscribe()
            .await;

        let s_group_relay_confirmation_signature_aggregation =
            GroupRelayConfirmationSignatureAggregationSubscriber::new(
                self.id(),
                id_address,
                self.get_chain_identity(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_group_relay_confirmation_signature_aggregation
            .subscribe()
            .await;
    }
}

impl<
        N: NodeInfoFetcher,
        G: GroupInfoFetcher + GroupInfoUpdater,
        T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
        I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder,
    > GeneralAdapterChain<N, G, T, I>
{
    pub fn new(
        id: usize,
        description: String,
        chain_identity: I,
        randomness_tasks_cache: T,
    ) -> Self {
        let chain_identity = Arc::new(RwLock::new(chain_identity));

        GeneralAdapterChain {
            id,
            description,
            chain_identity,
            block_cache: Arc::new(RwLock::new(InMemoryBlockInfoCache::new())),
            randomness_tasks_cache: Arc::new(RwLock::new(randomness_tasks_cache)),
            committer_randomness_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<RandomnessResultCache>::new(),
            )),
            group_relay_confirmation_tasks_cache: Arc::new(RwLock::new(InMemoryBLSTasksQueue::<
                GroupRelayConfirmationTask,
            >::new())),
            committer_group_relay_confirmation_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<GroupRelayConfirmationResultCache>::new(),
            )),
            n: PhantomData,
            g: PhantomData,
        }
    }
}

pub struct GeneralMainChain<
    N: NodeInfoFetcher,
    G: GroupInfoFetcher + GroupInfoUpdater,
    T: BLSTasksFetcher<RandomnessTask> + BLSTasksUpdater<RandomnessTask>,
    I: ChainIdentity + ControllerClientBuilder + CoordinatorClientBuilder + AdapterClientBuilder,
> {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<I>>,
    node_cache: Arc<RwLock<N>>,
    group_cache: Arc<RwLock<G>>,
    group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
    committer_group_relay_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<T>>,
    committer_randomness_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
}

impl
    GeneralMainChain<
        InMemoryNodeInfoCache,
        InMemoryGroupInfoCache,
        InMemoryBLSTasksQueue<RandomnessTask>,
        MockChainIdentity,
    >
{
    pub fn new(
        id: usize,
        description: String,
        chain_identity: MockChainIdentity,
        node_cache: InMemoryNodeInfoCache,
        group_cache: InMemoryGroupInfoCache,
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
            group_relay_tasks_cache: Arc::new(RwLock::new(
                InMemoryBLSTasksQueue::<GroupRelayTask>::new(),
            )),
            committer_group_relay_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<GroupRelayResultCache>::new(),
            )),
        }
    }
}

impl
    GeneralMainChain<
        NodeInfoDBClient,
        GroupInfoDBClient,
        BLSTasksDBClient<RandomnessTask>,
        GeneralChainIdentity,
    >
{
    pub fn new(
        id: usize,
        description: String,
        chain_identity: GeneralChainIdentity,
        node_cache: NodeInfoDBClient,
        group_cache: GroupInfoDBClient,
        randomness_tasks_cache: BLSTasksDBClient<RandomnessTask>,
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
            group_relay_tasks_cache: Arc::new(RwLock::new(
                InMemoryBLSTasksQueue::<GroupRelayTask>::new(),
            )),
            committer_group_relay_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<GroupRelayResultCache>::new(),
            )),
        }
    }
}

#[async_trait]
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
    > Chain for GeneralMainChain<N, G, T, I>
{
    type BlockInfoCache = InMemoryBlockInfoCache;

    type RandomnessTasksQueue = T;

    type RandomnessResultCaches = InMemorySignatureResultCache<RandomnessResultCache>;

    type Context = GeneralContext<N, G, T, I>;

    type ChainIdentity = I;

    async fn init_listeners(&self, context: &GeneralContext<N, G, T, I>) {
        self.init_block_listeners(context).await;

        self.init_dkg_listeners(context).await;

        self.init_randomness_listeners(context).await;

        self.init_group_relay_listeners(context).await;
    }

    async fn init_subscribers(&self, context: &GeneralContext<N, G, T, I>) {
        self.init_block_subscribers(context).await;

        self.init_dkg_subscribers(context).await;

        self.init_randomness_subscribers(context).await;

        self.init_group_relay_subscribers(context).await;
    }
}

#[async_trait]
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
    > MainChain for GeneralMainChain<N, G, T, I>
{
    type NodeInfoCache = N;

    type GroupInfoCache = G;

    type GroupRelayTasksQueue = InMemoryBLSTasksQueue<GroupRelayTask>;

    type GroupRelayResultCaches = InMemorySignatureResultCache<GroupRelayResultCache>;

    async fn init_block_listeners(&self, context: &Self::Context) {
        let p_block = BlockListener::new(
            self.id(),
            self.get_chain_identity(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_block.start().await {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_dkg_listeners(&self, context: &Self::Context) {
        let p_pre_grouping = PreGroupingListener::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_pre_grouping.start().await {
                    error!("{:?}", e);
                };
            });

        let p_post_commit_grouping = PostCommitGroupingListener::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_post_commit_grouping.start().await {
                    error!("{:?}", e);
                };
            });

        let p_post_grouping = PostGroupingListener::new(
            self.get_block_cache(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_post_grouping.start().await {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_randomness_listeners(&self, context: &Self::Context) {
        let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

        let p_new_randomness_task = NewRandomnessTaskListener::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_new_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskListener::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            self.get_block_cache(),
            self.get_group_cache(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_randomness_signature_aggregation = RandomnessSignatureAggregationListener::new(
            self.id(),
            id_address,
            self.get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_randomness_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });
    }

    async fn init_group_relay_listeners(&self, context: &Self::Context) {
        let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

        let p_new_group_relay_task = NewGroupRelayTaskListener::new(
            self.get_chain_identity(),
            self.get_group_relay_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_new_group_relay_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_group_relay_task = ReadyToHandleGroupRelayTaskListener::new(
            self.get_block_cache(),
            self.get_group_cache(),
            self.get_group_relay_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_group_relay_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_group_relay_signature_aggregation = GroupRelaySignatureAggregationListener::new(
            id_address,
            self.get_group_cache(),
            self.get_group_relay_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .await
            .add_task(async move {
                if let Err(e) = p_group_relay_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });
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

        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::new(
            self.id(),
            id_address,
            self.get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_randomness_task.subscribe().await;

        let s_randomness_signature_aggregation = RandomnessSignatureAggregationSubscriber::new(
            self.id(),
            id_address,
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_randomness_signature_aggregation.subscribe().await;
    }

    async fn init_group_relay_subscribers(&self, context: &Self::Context) {
        let s_ready_to_handle_group_relay_task = ReadyToHandleGroupRelayTaskSubscriber::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            self.get_group_relay_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_group_relay_task.subscribe().await;
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
    > ChainFetcher<GeneralAdapterChain<N, G, T, I>> for GeneralAdapterChain<N, G, T, I>
{
    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(&self) -> Arc<RwLock<I>> {
        self.chain_identity.clone()
    }

    fn get_block_cache(
        &self,
    ) -> Arc<RwLock<<GeneralAdapterChain<N, G, T, I> as Chain>::BlockInfoCache>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(
        &self,
    ) -> Arc<RwLock<<GeneralAdapterChain<N, G, T, I> as Chain>::RandomnessTasksQueue>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<<GeneralAdapterChain<N, G, T, I> as Chain>::RandomnessResultCaches>> {
        self.committer_randomness_result_cache.clone()
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
    > AdapterChainFetcher<GeneralAdapterChain<N, G, T, I>> for GeneralAdapterChain<N, G, T, I>
{
    fn get_group_relay_confirmation_tasks_cache(
        &self,
    ) -> Arc<
        RwLock<<GeneralAdapterChain<N, G, T, I> as AdapterChain>::GroupRelayConfirmationTasksQueue>,
    > {
        self.group_relay_confirmation_tasks_cache.clone()
    }

    fn get_group_relay_confirmation_result_cache(
        &self,
    ) -> Arc<
        RwLock<
            <GeneralAdapterChain<N, G, T, I> as AdapterChain>::GroupRelayConfirmationResultCaches,
        >,
    > {
        self.committer_group_relay_confirmation_result_cache.clone()
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
    > ChainFetcher<GeneralMainChain<N, G, T, I>> for GeneralMainChain<N, G, T, I>
{
    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as Chain>::ChainIdentity>> {
        self.chain_identity.clone()
    }

    fn get_block_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as Chain>::BlockInfoCache>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as Chain>::RandomnessTasksQueue>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as Chain>::RandomnessResultCaches>> {
        self.committer_randomness_result_cache.clone()
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
    > MainChainFetcher<GeneralMainChain<N, G, T, I>> for GeneralMainChain<N, G, T, I>
{
    fn get_node_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as MainChain>::NodeInfoCache>> {
        self.node_cache.clone()
    }

    fn get_group_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as MainChain>::GroupInfoCache>> {
        self.group_cache.clone()
    }

    fn get_group_relay_tasks_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as MainChain>::GroupRelayTasksQueue>> {
        self.group_relay_tasks_cache.clone()
    }

    fn get_group_relay_result_cache(
        &self,
    ) -> Arc<RwLock<<GeneralMainChain<N, G, T, I> as MainChain>::GroupRelayResultCaches>> {
        self.committer_group_relay_result_cache.clone()
    }
}
