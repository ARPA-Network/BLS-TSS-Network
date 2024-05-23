use super::{Chain, MainChain, RelayedChain};
use crate::{
    context::{ChainIdentityHandlerType, ContextFetcher},
    listener::{
        block::BlockListener, new_randomness_task::NewRandomnessTaskListener,
        post_commit_grouping::PostCommitGroupingListener, post_grouping::PostGroupingListener,
        pre_grouping::PreGroupingListener,
        randomness_signature_aggregation::RandomnessSignatureAggregationListener,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskListener,
        schedule_node_activation::NodeActivationListener, Listener,
    },
    queue::event_queue::EventQueue,
    scheduler::{fixed::SimpleFixedTaskScheduler, TaskScheduler},
    subscriber::{
        block::BlockSubscriber, in_grouping::InGroupingSubscriber,
        post_grouping::PostGroupingSubscriber,
        post_success_grouping::PostSuccessGroupingSubscriber, pre_grouping::PreGroupingSubscriber,
        randomness_signature_aggregation::RandomnessSignatureAggregationSubscriber,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskSubscriber,
        schedule_node_activation::NodeActivationSubscriber, Subscriber,
    },
};
use arpa_core::{
    ChainIdentity, ComponentTaskType, GeneralMainChainIdentity, GeneralRelayedChainIdentity,
    ListenerDescriptor, ListenerType, RandomnessTask, SchedulerError, SchedulerResult,
    TimeLimitDescriptor, DEFAULT_NODE_ACTIVATION_INTERVAL_MILLIS,
};
use arpa_dal::cache::{InMemoryBlockInfoCache, RandomnessResultCache};
use arpa_dal::{
    BLSTasksHandler, BlockInfoHandler, GroupInfoHandler, NodeInfoHandler,
    SignatureResultCacheHandler,
};
use async_trait::async_trait;
use log::error;
use std::{fmt::Display, marker::PhantomData, sync::Arc};
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

async fn add_listener_task(
    chain_id: usize,
    mut listener: impl Listener + Send + Sync + Display + 'static,
    listener_descriptor: ListenerDescriptor,
    fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
) -> SchedulerResult<()> {
    if let Err(e) = listener.initialize().await {
        return Err(SchedulerError::ListenerInitializationError(e.to_string()));
    };
    fs.write().await.add_task(
        ComponentTaskType::Listener(chain_id, listener_descriptor.l_type),
        async move {
            if let Err(e) = listener
                .start(
                    listener_descriptor.interval_millis,
                    listener_descriptor.use_jitter,
                    listener_descriptor.reset_descriptor,
                )
                .await
            {
                error!("{:?}", e);
            };
        },
    )
}

#[derive(Debug)]
pub struct GeneralMainChain<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    id: usize,
    description: String,
    is_eigenlayer: bool,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    committer_randomness_result_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    time_limits: TimeLimitDescriptor,
    listener_descriptors: Vec<ListenerDescriptor>,
}

impl<
        PC: Curve + Send + Sync + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > GeneralMainChain<PC, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        description: String,
        is_eigenlayer: bool,
        chain_identity: GeneralMainChainIdentity,
        node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        committer_randomness_result_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        time_limits: TimeLimitDescriptor,
        listener_descriptors: Vec<ListenerDescriptor>,
    ) -> Self {
        let chain_id = chain_identity.get_chain_id();
        GeneralMainChain {
            id: chain_id,
            description,
            is_eigenlayer,
            chain_identity: Arc::new(RwLock::new(Box::new(chain_identity))),
            block_cache: Arc::new(RwLock::new(Box::new(InMemoryBlockInfoCache::new(
                chain_id,
                time_limits.block_time,
            )))),
            randomness_tasks_cache,
            committer_randomness_result_cache,
            node_cache,
            group_cache,
            c: PhantomData,
            s: PhantomData,
            time_limits,
            listener_descriptors,
        }
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > Chain<PC, S> for GeneralMainChain<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    type NodeInfoCache = Box<dyn NodeInfoHandler<PC>>;

    type GroupInfoCache = Box<dyn GroupInfoHandler<PC>>;

    type BlockInfoCache = Box<dyn BlockInfoHandler>;

    type RandomnessTasksQueue = Box<dyn BLSTasksHandler<RandomnessTask>>;

    type RandomnessResultCaches = Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>;

    type ChainIdentity = ChainIdentityHandlerType<PC>;

    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentityHandlerType<PC>>> {
        self.chain_identity.clone()
    }

    fn get_node_cache(&self) -> Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>> {
        self.node_cache.clone()
    }

    fn get_group_cache(&self) -> Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>> {
        self.group_cache.clone()
    }

    fn get_block_cache(&self) -> Arc<RwLock<Box<dyn BlockInfoHandler>>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>> {
        self.committer_randomness_result_cache.clone()
    }

    async fn init_listener(
        &self,
        eq: Arc<RwLock<EventQueue>>,
        fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
        listener: ListenerDescriptor,
    ) -> SchedulerResult<()> {
        match listener.l_type {
            ListenerType::ScheduleNodeActivation => {
                let p_node_activation = NodeActivationListener::new(
                    self.id(),
                    self.is_eigenlayer,
                    self.get_chain_identity(),
                    eq,
                );

                add_listener_task(self.id(), p_node_activation, listener, fs).await
            }
            ListenerType::Block => {
                let p_block = BlockListener::new(self.id(), self.get_chain_identity(), eq);

                add_listener_task(self.id(), p_block, listener, fs).await
            }
            ListenerType::PreGrouping => {
                let p_pre_grouping =
                    PreGroupingListener::new(self.get_chain_identity(), self.get_group_cache(), eq);

                add_listener_task(self.id(), p_pre_grouping, listener, fs).await
            }
            ListenerType::PostCommitGrouping => {
                let p_post_commit_grouping = PostCommitGroupingListener::new(
                    self.get_chain_identity(),
                    self.get_group_cache(),
                    eq,
                );

                add_listener_task(self.id(), p_post_commit_grouping, listener, fs).await
            }
            ListenerType::PostGrouping => {
                let p_post_grouping = PostGroupingListener::new(
                    self.get_block_cache(),
                    self.get_group_cache(),
                    eq,
                    self.time_limits.dkg_timeout_duration,
                );

                add_listener_task(self.id(), p_post_grouping, listener, fs).await
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

                add_listener_task(self.id(), p_new_randomness_task, listener, fs).await
            }
            ListenerType::ReadyToHandleRandomnessTask => {
                let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

                let p_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskListener::new(
                    self.id(),
                    id_address,
                    self.get_chain_identity(),
                    self.get_block_cache(),
                    self.get_group_cache(),
                    self.get_randomness_tasks_cache(),
                    eq,
                    self.time_limits.randomness_task_exclusive_window,
                );

                add_listener_task(self.id(), p_ready_to_handle_randomness_task, listener, fs).await
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

                add_listener_task(self.id(), p_randomness_signature_aggregation, listener, fs).await
            }
        }
    }

    async fn init_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        self.init_schedule_listeners(context).await?;

        self.init_block_listeners(context).await?;

        self.init_dkg_listeners(context).await?;

        self.init_randomness_listeners(context).await?;

        Ok(())
    }

    async fn init_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        self.init_schedule_subscribers(context).await;

        self.init_block_subscribers(context).await;

        self.init_dkg_subscribers(context).await;

        self.init_randomness_subscribers(context).await;
    }

    async fn init_components(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        self.init_listeners(context).await?;

        self.init_subscribers(context).await;

        Ok(())
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > MainChain<PC, S> for GeneralMainChain<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn init_schedule_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        self.init_listener(
            context.get_event_queue(),
            context.get_fixed_task_handler(),
            ListenerDescriptor {
                l_type: ListenerType::ScheduleNodeActivation,
                interval_millis: DEFAULT_NODE_ACTIVATION_INTERVAL_MILLIS,
                use_jitter: false,
                reset_descriptor: self.time_limits.provider_reset_descriptor,
            },
        )
        .await?;

        Ok(())
    }

    async fn init_block_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        for listener in self.listener_descriptors.iter() {
            if listener.l_type == ListenerType::Block {
                self.init_listener(
                    context.get_event_queue(),
                    context.get_fixed_task_handler(),
                    *listener,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn init_dkg_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        for listener in self.listener_descriptors.iter() {
            if listener.l_type == ListenerType::PreGrouping
                || listener.l_type == ListenerType::PostCommitGrouping
                || listener.l_type == ListenerType::PostGrouping
            {
                self.init_listener(
                    context.get_event_queue(),
                    context.get_fixed_task_handler(),
                    *listener,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn init_randomness_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        for listener in self.listener_descriptors.iter() {
            if listener.l_type == ListenerType::NewRandomnessTask
                || listener.l_type == ListenerType::ReadyToHandleRandomnessTask
                || listener.l_type == ListenerType::RandomnessSignatureAggregation
            {
                self.init_listener(
                    context.get_event_queue(),
                    context.get_fixed_task_handler(),
                    *listener,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn init_schedule_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let s_node_activation = NodeActivationSubscriber::<PC>::new(
            self.get_chain_identity(),
            context.get_event_queue(),
        );

        s_node_activation.subscribe().await;
    }

    async fn init_block_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe().await;
    }

    async fn init_dkg_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let s_pre_grouping =
            PreGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_pre_grouping.subscribe().await;

        let s_in_grouping = InGroupingSubscriber::new(
            self.get_chain_identity(),
            self.get_node_cache(),
            self.get_group_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
            self.time_limits.dkg_wait_for_phase_interval_millis,
        );

        s_in_grouping.subscribe().await;

        let s_post_success_grouping =
            PostSuccessGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_post_success_grouping.subscribe().await;

        let s_post_grouping = PostGroupingSubscriber::new(
            self.get_chain_identity(),
            context.get_supported_relayed_chains(),
            self.get_group_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_post_grouping.subscribe().await;
    }

    async fn init_randomness_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::<PC, S>::new(
            self.id(),
            id_address,
            self.get_group_cache(),
            self.get_randomness_tasks_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
            self.time_limits.commit_partial_signature_retry_descriptor,
        );

        s_ready_to_handle_randomness_task.subscribe().await;

        let s_randomness_signature_aggregation =
            RandomnessSignatureAggregationSubscriber::<PC, S>::new(
                self.id(),
                id_address,
                self.get_chain_identity(),
                self.get_block_cache(),
                self.get_randomness_result_cache(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_randomness_signature_aggregation.subscribe().await;
    }
}

#[derive(Debug)]
pub struct GeneralRelayedChain<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
> {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<ChainIdentityHandlerType<PC>>>,
    node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
    group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
    committer_randomness_result_cache:
        Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>>,
    c: PhantomData<PC>,
    s: PhantomData<S>,
    time_limits: TimeLimitDescriptor,
    listener_descriptors: Vec<ListenerDescriptor>,
}

impl<
        PC: Curve + Send + Sync + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > GeneralRelayedChain<PC, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        description: String,
        chain_identity: GeneralRelayedChainIdentity,
        node_cache: Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>>,
        group_cache: Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>>,
        randomness_tasks_cache: Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>>,
        committer_randomness_result_cache: Arc<
            RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>,
        >,
        time_limits: TimeLimitDescriptor,
        listener_descriptors: Vec<ListenerDescriptor>,
    ) -> Self {
        let chain_id = chain_identity.get_chain_id();

        GeneralRelayedChain {
            id: chain_id,
            description,
            chain_identity: Arc::new(RwLock::new(Box::new(chain_identity))),
            block_cache: Arc::new(RwLock::new(Box::new(InMemoryBlockInfoCache::new(
                chain_id,
                time_limits.block_time,
            )))),
            randomness_tasks_cache,
            committer_randomness_result_cache,
            node_cache,
            group_cache,
            c: PhantomData,
            s: PhantomData,
            time_limits,
            listener_descriptors,
        }
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Sync
            + Send
            + 'static,
    > Chain<PC, S> for GeneralRelayedChain<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    type NodeInfoCache = Box<dyn NodeInfoHandler<PC>>;

    type GroupInfoCache = Box<dyn GroupInfoHandler<PC>>;

    type BlockInfoCache = Box<dyn BlockInfoHandler>;

    type RandomnessTasksQueue = Box<dyn BLSTasksHandler<RandomnessTask>>;

    type RandomnessResultCaches = Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>;

    type ChainIdentity = ChainIdentityHandlerType<PC>;

    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }
    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentityHandlerType<PC>>> {
        self.chain_identity.clone()
    }

    fn get_node_cache(&self) -> Arc<RwLock<Box<dyn NodeInfoHandler<PC>>>> {
        self.node_cache.clone()
    }

    fn get_group_cache(&self) -> Arc<RwLock<Box<dyn GroupInfoHandler<PC>>>> {
        self.group_cache.clone()
    }

    fn get_block_cache(&self) -> Arc<RwLock<Box<dyn BlockInfoHandler>>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<Box<dyn BLSTasksHandler<RandomnessTask>>>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<Box<dyn SignatureResultCacheHandler<RandomnessResultCache>>>> {
        self.committer_randomness_result_cache.clone()
    }

    async fn init_listener(
        &self,
        eq: Arc<RwLock<EventQueue>>,
        fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
        listener: ListenerDescriptor,
    ) -> SchedulerResult<()> {
        match listener.l_type {
            ListenerType::Block => {
                let p_block = BlockListener::new(self.id(), self.get_chain_identity(), eq);

                add_listener_task(self.id, p_block, listener, fs).await
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

                add_listener_task(self.id, p_new_randomness_task, listener, fs).await
            }
            ListenerType::ReadyToHandleRandomnessTask => {
                let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

                let p_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskListener::new(
                    self.id(),
                    id_address,
                    self.get_chain_identity(),
                    self.get_block_cache(),
                    self.get_group_cache(),
                    self.get_randomness_tasks_cache(),
                    eq,
                    self.time_limits.randomness_task_exclusive_window,
                );

                add_listener_task(self.id, p_ready_to_handle_randomness_task, listener, fs).await
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

                add_listener_task(self.id, p_randomness_signature_aggregation, listener, fs).await
            }
            _ => {
                return Err(SchedulerError::UnsupportedListenerType(
                    self.id(),
                    listener.l_type.to_string(),
                ))
            }
        }
    }

    async fn init_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        self.init_block_listeners(context).await?;

        self.init_randomness_listeners(context).await?;

        Ok(())
    }

    async fn init_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        self.init_block_subscribers(context).await;

        self.init_randomness_subscribers(context).await;
    }

    async fn init_components(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        self.init_listeners(context).await?;

        self.init_subscribers(context).await;

        Ok(())
    }
}

#[async_trait]
impl<
        PC: Curve + std::fmt::Debug + Clone + Sync + Send + 'static,
        S: SignatureScheme
            + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>
            + Clone
            + Send
            + Sync
            + 'static,
    > RelayedChain<PC, S> for GeneralRelayedChain<PC, S>
where
    <S as ThresholdScheme>::Error: Sync + Send,
    <S as SignatureScheme>::Error: Sync + Send,
{
    async fn init_block_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        for listener in self.listener_descriptors.iter() {
            if listener.l_type == ListenerType::Block {
                self.init_listener(
                    context.get_event_queue(),
                    context.get_fixed_task_handler(),
                    *listener,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn init_randomness_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()> {
        for listener in self.listener_descriptors.iter() {
            if listener.l_type == ListenerType::NewRandomnessTask
                || listener.l_type == ListenerType::ReadyToHandleRandomnessTask
                || listener.l_type == ListenerType::RandomnessSignatureAggregation
            {
                self.init_listener(
                    context.get_event_queue(),
                    context.get_fixed_task_handler(),
                    *listener,
                )
                .await?;
            }
        }

        Ok(())
    }

    async fn init_block_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe().await;
    }

    async fn init_randomness_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send)) {
        let id_address = self.get_node_cache().read().await.get_id_address().unwrap();

        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::<PC, S>::new(
            self.id(),
            id_address,
            self.get_group_cache(),
            self.get_randomness_tasks_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
            self.time_limits.commit_partial_signature_retry_descriptor,
        );

        s_ready_to_handle_randomness_task.subscribe().await;

        let s_randomness_signature_aggregation =
            RandomnessSignatureAggregationSubscriber::<PC, S>::new(
                self.id(),
                id_address,
                self.get_chain_identity(),
                self.get_block_cache(),
                self.get_randomness_result_cache(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_randomness_signature_aggregation.subscribe().await;
    }
}
