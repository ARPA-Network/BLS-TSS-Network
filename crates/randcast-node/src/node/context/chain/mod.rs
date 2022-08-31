pub mod types;

use crate::node::dal::types::ChainIdentity;
use parking_lot::RwLock;
use std::sync::Arc;

use super::ContextFetcher;

pub(crate) trait Chain {
    type BlockInfoCache;
    type RandomnessTasksQueue;
    type RandomnessResultCaches;
    type Context;

    fn init_components(&self, context: &Self::Context) {
        self.init_listeners(context);

        self.init_subscribers(context);
    }

    fn init_listeners(&self, context: &Self::Context);

    fn init_subscribers(&self, context: &Self::Context);
}

pub(crate) trait AdapterChain: Chain {
    type GroupRelayConfirmationTasksQueue;
    type GroupRelayConfirmationResultCaches;
}

pub(crate) trait MainChain: Chain {
    type NodeInfoCache;
    type GroupInfoCache;
    type GroupRelayTasksQueue;
    type GroupRelayResultCaches;
}

pub(crate) trait ChainFetcher<T: Chain> {
    fn id(&self) -> usize;

    fn description(&self) -> &str;

    fn get_chain_identity(&self) -> Arc<RwLock<ChainIdentity>>;

    fn get_block_cache(&self) -> Arc<RwLock<T::BlockInfoCache>>;

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<T::RandomnessTasksQueue>>;

    fn get_randomness_result_cache(&self) -> Arc<RwLock<T::RandomnessResultCaches>>;
}

pub(crate) trait AdapterChainFetcher<T: AdapterChain> {
    fn get_group_relay_confirmation_tasks_cache(
        &self,
    ) -> Arc<RwLock<T::GroupRelayConfirmationTasksQueue>>;

    fn get_group_relay_confirmation_result_cache(
        &self,
    ) -> Arc<RwLock<T::GroupRelayConfirmationResultCaches>>;
}

pub(crate) trait MainChainFetcher<T: MainChain> {
    fn get_node_cache(&self) -> Arc<RwLock<T::NodeInfoCache>>;

    fn get_group_cache(&self) -> Arc<RwLock<T::GroupInfoCache>>;

    fn get_group_relay_tasks_cache(&self) -> Arc<RwLock<T::GroupRelayTasksQueue>>;

    fn get_group_relay_result_cache(&self) -> Arc<RwLock<T::GroupRelayResultCaches>>;
}
