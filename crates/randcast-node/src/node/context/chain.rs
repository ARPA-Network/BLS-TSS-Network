use super::context::{ContextFetcher, InMemoryContext};
use crate::node::{
    dao::{
        api::NodeInfoFetcher,
        cache::{
            GroupRelayConfirmationResultCache, GroupRelayResultCache, InMemoryBLSTasksQueue,
            InMemoryBlockInfoCache, InMemoryGroupInfoCache, InMemoryNodeInfoCache,
            InMemorySignatureResultCache, RandomnessResultCache,
        },
        types::{ChainIdentity, GroupRelayConfirmationTask, GroupRelayTask, RandomnessTask},
    },
    listener::{
        block::MockBlockListener,
        group_relay_confirmation_signature_aggregation::MockGroupRelayConfirmationSignatureAggregationListener,
        group_relay_signature_aggregation::MockGroupRelaySignatureAggregationListener,
        new_group_relay_confirmation_task::MockNewGroupRelayConfirmationTaskListener,
        new_group_relay_task::MockNewGroupRelayTaskListener,
        new_randomness_task::MockNewRandomnessTaskListener,
        post_commit_grouping::MockPostCommitGroupingListener,
        post_grouping::MockPostGroupingListener, pre_grouping::MockPreGroupingListener,
        randomness_signature_aggregation::MockRandomnessSignatureAggregationListener,
        ready_to_handle_group_relay_confirmation_task::MockReadyToHandleGroupRelayConfirmationTaskListener,
        ready_to_handle_group_relay_task::MockReadyToHandleGroupRelayTaskListener,
        ready_to_handle_randomness_task::MockReadyToHandleRandomnessTaskListener, types::Listener,
    },
    scheduler::fixed::FixedTaskScheduler,
    subscriber::{
        block::BlockSubscriber,
        group_relay_confirmation_signature_aggregation::GroupRelayConfirmationSignatureAggregationSubscriber,
        group_relay_signature_aggregation::GroupRelaySignatureAggregationSubscriber,
        in_grouping::InGroupingSubscriber, post_grouping::PostGroupingSubscriber,
        post_success_grouping::PostSuccessGroupingSubscriber, pre_grouping::PreGroupingSubscriber,
        randomness_signature_aggregation::RandomnessSignatureAggregationSubscriber,
        ready_to_handle_group_relay_confirmation_task::ReadyToHandleGroupRelayConfirmationTaskSubscriber,
        ready_to_handle_group_relay_task::ReadyToHandleGroupRelayTaskSubscriber,
        ready_to_handle_randomness_task::ReadyToHandleRandomnessTaskSubscriber, types::Subscriber,
    },
};
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use threshold_bls::curve::bls12381::{Scalar, G1};

pub trait Chain {
    type BlockInfoCache;
    type RandomnessTasksQueue;
    type RandomnessResultCaches;

    fn init_components(&self, context: &InMemoryContext) {
        self.init_listeners(context);

        self.init_subscribers(context);
    }

    fn init_listeners(&self, context: &InMemoryContext);

    fn init_subscribers(&self, context: &InMemoryContext);
}

pub trait AdapterChain: Chain {
    type GroupRelayConfirmationTasksQueue;
    type GroupRelayConfirmationResultCaches;
}

pub trait MainChain: Chain {
    type NodeInfoCache;
    type GroupInfoCache;
    type GroupRelayTasksQueue;
    type GroupRelayResultCaches;
}

pub trait ChainFetcher<T: Chain> {
    fn id(&self) -> usize;

    fn description(&self) -> &str;

    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentity>>;

    fn get_block_cache(&self) -> Arc<RwLock<T::BlockInfoCache>>;

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<T::RandomnessTasksQueue>>;

    fn get_randomness_result_cache(&self) -> Arc<RwLock<T::RandomnessResultCaches>>;
}

pub trait AdapterChainFetcher<T: AdapterChain> {
    fn get_group_relay_confirmation_tasks_cache(
        &self,
    ) -> Arc<RwLock<T::GroupRelayConfirmationTasksQueue>>;

    fn get_group_relay_confirmation_result_cache(
        &self,
    ) -> Arc<RwLock<T::GroupRelayConfirmationResultCaches>>;
}

pub trait MainChainFetcher<T: MainChain> {
    fn get_node_cache(&self) -> Arc<RwLock<T::NodeInfoCache>>;

    fn get_group_cache(&self) -> Arc<RwLock<T::GroupInfoCache>>;

    fn get_group_relay_tasks_cache(&self) -> Arc<RwLock<T::GroupRelayTasksQueue>>;

    fn get_group_relay_result_cache(&self) -> Arc<RwLock<T::GroupRelayResultCaches>>;
}

pub struct InMemoryAdapterChain {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<RandomnessTask>>>,
    group_relay_confirmation_tasks_cache:
        Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayConfirmationTask>>>,
    committer_randomness_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,

    committer_group_relay_confirmation_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayConfirmationResultCache>>>,
}

impl Chain for InMemoryAdapterChain {
    type BlockInfoCache = InMemoryBlockInfoCache;

    type RandomnessTasksQueue = InMemoryBLSTasksQueue<RandomnessTask>;

    type RandomnessResultCaches = InMemorySignatureResultCache<RandomnessResultCache>;

    fn init_listeners(&self, context: &InMemoryContext) {
        // block
        let p_block = MockBlockListener::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_block.start().await {
                    error!("{:?}", e);
                };
            });

        // randomness
        let p_new_randomness_task = MockNewRandomnessTaskListener::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_new_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_randomness_task = MockReadyToHandleRandomnessTaskListener::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            self.get_block_cache(),
            context.get_main_chain().get_group_cache(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_randomness_signature_aggregation = MockRandomnessSignatureAggregationListener::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            context.get_main_chain().get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_randomness_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });

        // group_relay_confirmation
        let p_new_group_relay_confirmation_task = MockNewGroupRelayConfirmationTaskListener::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            self.get_group_relay_confirmation_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_new_group_relay_confirmation_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_group_relay_confirmation_task =
            MockReadyToHandleGroupRelayConfirmationTaskListener::new(
                self.id(),
                self.get_block_cache(),
                context.get_main_chain().get_group_cache(),
                self.get_group_relay_confirmation_tasks_cache(),
                context.get_event_queue(),
            );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_group_relay_confirmation_task
                    .start()
                    .await
                {
                    error!("{:?}", e);
                };
            });

        let p_group_relay_confirmation_signature_aggregation =
            MockGroupRelayConfirmationSignatureAggregationListener::new(
                self.id(),
                context
                    .get_main_chain()
                    .get_node_cache()
                    .read()
                    .get_id_address()
                    .to_string(),
                context.get_main_chain().get_group_cache(),
                self.get_group_relay_confirmation_result_cache(),
                context.get_event_queue(),
            );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_group_relay_confirmation_signature_aggregation
                    .start()
                    .await
                {
                    error!("{:?}", e);
                };
            });
    }

    fn init_subscribers(&self, context: &InMemoryContext) {
        // block
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe();

        // randomness
        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            context.get_main_chain().get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_randomness_task.subscribe();

        let s_randomness_signature_aggregation = RandomnessSignatureAggregationSubscriber::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_randomness_signature_aggregation.subscribe();

        // group_relay
        let s_group_relay_signature_aggregation = GroupRelaySignatureAggregationSubscriber::new(
            self.id(),
            context
                .get_main_chain()
                .get_node_cache()
                .read()
                .get_id_address()
                .to_string(),
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_group_relay_signature_aggregation.subscribe();

        // group_relay_confirmation
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

        s_ready_to_handle_group_relay_confirmation_task.subscribe();

        let s_group_relay_confirmation_signature_aggregation =
            GroupRelayConfirmationSignatureAggregationSubscriber::new(
                self.id(),
                context
                    .get_main_chain()
                    .get_node_cache()
                    .read()
                    .get_id_address()
                    .to_string(),
                self.get_chain_identity(),
                context.get_event_queue(),
                context.get_dynamic_task_handler(),
            );

        s_group_relay_confirmation_signature_aggregation.subscribe();
    }
}

impl AdapterChain for InMemoryAdapterChain {
    type GroupRelayConfirmationTasksQueue = InMemoryBLSTasksQueue<GroupRelayConfirmationTask>;

    type GroupRelayConfirmationResultCaches =
        InMemorySignatureResultCache<GroupRelayConfirmationResultCache>;
}

impl InMemoryAdapterChain {
    pub fn new(id: usize, description: String, chain_identity: ChainIdentity) -> Self {
        let chain_identity = Arc::new(RwLock::new(chain_identity));

        InMemoryAdapterChain {
            id,
            description,
            chain_identity,
            block_cache: Arc::new(RwLock::new(InMemoryBlockInfoCache::new())),
            randomness_tasks_cache: Arc::new(RwLock::new(
                InMemoryBLSTasksQueue::<RandomnessTask>::new(),
            )),
            committer_randomness_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<RandomnessResultCache>::new(),
            )),
            group_relay_confirmation_tasks_cache: Arc::new(RwLock::new(InMemoryBLSTasksQueue::<
                GroupRelayConfirmationTask,
            >::new())),
            committer_group_relay_confirmation_result_cache: Arc::new(RwLock::new(
                InMemorySignatureResultCache::<GroupRelayConfirmationResultCache>::new(),
            )),
        }
    }
}

pub struct InMemoryMainChain {
    id: usize,
    description: String,
    chain_identity: Arc<RwLock<ChainIdentity>>,
    node_cache: Arc<RwLock<InMemoryNodeInfoCache>>,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    group_relay_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<GroupRelayTask>>>,
    committer_group_relay_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<GroupRelayResultCache>>>,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    randomness_tasks_cache: Arc<RwLock<InMemoryBLSTasksQueue<RandomnessTask>>>,
    committer_randomness_result_cache:
        Arc<RwLock<InMemorySignatureResultCache<RandomnessResultCache>>>,
}

impl InMemoryMainChain {
    pub fn new(
        id: usize,
        description: String,
        chain_identity: ChainIdentity,
        node_rpc_endpoint: String,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> Self {
        let id_address = chain_identity.get_id_address().to_string();

        let chain_identity = Arc::new(RwLock::new(chain_identity));

        let node_cache = InMemoryNodeInfoCache::new(
            id_address,
            node_rpc_endpoint,
            dkg_private_key,
            dkg_public_key,
        );

        let group_cache = InMemoryGroupInfoCache::new();

        InMemoryMainChain {
            id,
            description,
            chain_identity,
            block_cache: Arc::new(RwLock::new(InMemoryBlockInfoCache::new())),
            randomness_tasks_cache: Arc::new(RwLock::new(
                InMemoryBLSTasksQueue::<RandomnessTask>::new(),
            )),
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

impl Chain for InMemoryMainChain {
    type BlockInfoCache = InMemoryBlockInfoCache;

    type RandomnessTasksQueue = InMemoryBLSTasksQueue<RandomnessTask>;

    type RandomnessResultCaches = InMemorySignatureResultCache<RandomnessResultCache>;

    fn init_listeners(&self, context: &InMemoryContext) {
        // block
        let p_block = MockBlockListener::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_chain_identity(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_block.start().await {
                    error!("{:?}", e);
                };
            });

        // dkg
        let p_pre_grouping = MockPreGroupingListener::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_pre_grouping.start().await {
                    error!("{:?}", e);
                };
            });

        let p_post_commit_grouping = MockPostCommitGroupingListener::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_post_commit_grouping.start().await {
                    error!("{:?}", e);
                };
            });

        let p_post_grouping = MockPostGroupingListener::new(
            self.get_block_cache(),
            self.get_group_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_post_grouping.start().await {
                    error!("{:?}", e);
                };
            });

        // randomness
        let p_new_randomness_task = MockNewRandomnessTaskListener::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_chain_identity(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_new_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_randomness_task = MockReadyToHandleRandomnessTaskListener::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_chain_identity(),
            self.get_block_cache(),
            self.get_group_cache(),
            self.get_randomness_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_randomness_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_randomness_signature_aggregation = MockRandomnessSignatureAggregationListener::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_randomness_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });

        // group_relay
        let p_new_group_relay_task = MockNewGroupRelayTaskListener::new(
            self.get_chain_identity(),
            self.get_group_relay_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_new_group_relay_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_ready_to_handle_group_relay_task = MockReadyToHandleGroupRelayTaskListener::new(
            self.get_block_cache(),
            self.get_group_cache(),
            self.get_group_relay_tasks_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_ready_to_handle_group_relay_task.start().await {
                    error!("{:?}", e);
                };
            });

        let p_group_relay_signature_aggregation = MockGroupRelaySignatureAggregationListener::new(
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_group_cache(),
            self.get_group_relay_result_cache(),
            context.get_event_queue(),
        );

        context
            .get_fixed_task_handler()
            .write()
            .add_task(async move {
                if let Err(e) = p_group_relay_signature_aggregation.start().await {
                    error!("{:?}", e);
                };
            });
    }

    fn init_subscribers(&self, context: &InMemoryContext) {
        // block
        let s_block =
            BlockSubscriber::new(self.id(), self.get_block_cache(), context.get_event_queue());

        s_block.subscribe();

        // dkg
        let s_pre_grouping =
            PreGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_pre_grouping.subscribe();

        let s_in_grouping = InGroupingSubscriber::new(
            self.get_chain_identity(),
            self.get_node_cache(),
            self.get_group_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_in_grouping.subscribe();

        let s_post_success_grouping =
            PostSuccessGroupingSubscriber::new(self.get_group_cache(), context.get_event_queue());

        s_post_success_grouping.subscribe();

        let s_post_grouping = PostGroupingSubscriber::new(
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_post_grouping.subscribe();

        // randomness
        let s_ready_to_handle_randomness_task = ReadyToHandleRandomnessTaskSubscriber::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_group_cache(),
            self.get_randomness_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_randomness_task.subscribe();

        let s_randomness_signature_aggregation = RandomnessSignatureAggregationSubscriber::new(
            self.id(),
            self.get_node_cache().read().get_id_address().to_string(),
            self.get_chain_identity(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_randomness_signature_aggregation.subscribe();

        // group_relay
        let s_ready_to_handle_group_relay_task = ReadyToHandleGroupRelayTaskSubscriber::new(
            self.get_chain_identity(),
            self.get_group_cache(),
            self.get_group_relay_result_cache(),
            context.get_event_queue(),
            context.get_dynamic_task_handler(),
        );

        s_ready_to_handle_group_relay_task.subscribe();
    }
}

impl MainChain for InMemoryMainChain {
    type NodeInfoCache = InMemoryNodeInfoCache;

    type GroupInfoCache = InMemoryGroupInfoCache;

    type GroupRelayTasksQueue = InMemoryBLSTasksQueue<GroupRelayTask>;

    type GroupRelayResultCaches = InMemorySignatureResultCache<GroupRelayResultCache>;
}

impl ChainFetcher<InMemoryAdapterChain> for InMemoryAdapterChain {
    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentity>> {
        self.chain_identity.clone()
    }

    fn get_block_cache(&self) -> Arc<RwLock<<InMemoryAdapterChain as Chain>::BlockInfoCache>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryAdapterChain as Chain>::RandomnessTasksQueue>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryAdapterChain as Chain>::RandomnessResultCaches>> {
        self.committer_randomness_result_cache.clone()
    }
}

impl AdapterChainFetcher<InMemoryAdapterChain> for InMemoryAdapterChain {
    fn get_group_relay_confirmation_tasks_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryAdapterChain as AdapterChain>::GroupRelayConfirmationTasksQueue>> {
        self.group_relay_confirmation_tasks_cache.clone()
    }

    fn get_group_relay_confirmation_result_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryAdapterChain as AdapterChain>::GroupRelayConfirmationResultCaches>>
    {
        self.committer_group_relay_confirmation_result_cache.clone()
    }
}

impl ChainFetcher<InMemoryMainChain> for InMemoryMainChain {
    fn id(&self) -> usize {
        self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentity>> {
        self.chain_identity.clone()
    }

    fn get_block_cache(&self) -> Arc<RwLock<<InMemoryMainChain as Chain>::BlockInfoCache>> {
        self.block_cache.clone()
    }

    fn get_randomness_tasks_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryMainChain as Chain>::RandomnessTasksQueue>> {
        self.randomness_tasks_cache.clone()
    }

    fn get_randomness_result_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryMainChain as Chain>::RandomnessResultCaches>> {
        self.committer_randomness_result_cache.clone()
    }
}

impl MainChainFetcher<InMemoryMainChain> for InMemoryMainChain {
    fn get_node_cache(&self) -> Arc<RwLock<<InMemoryMainChain as MainChain>::NodeInfoCache>> {
        self.node_cache.clone()
    }

    fn get_group_cache(&self) -> Arc<RwLock<<InMemoryMainChain as MainChain>::GroupInfoCache>> {
        self.group_cache.clone()
    }

    fn get_group_relay_tasks_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryMainChain as MainChain>::GroupRelayTasksQueue>> {
        self.group_relay_tasks_cache.clone()
    }

    fn get_group_relay_result_cache(
        &self,
    ) -> Arc<RwLock<<InMemoryMainChain as MainChain>::GroupRelayResultCaches>> {
        self.committer_group_relay_result_cache.clone()
    }
}
