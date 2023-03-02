pub mod types;
use crate::node::{
    queue::event_queue::EventQueue,
    scheduler::{fixed::SimpleFixedTaskScheduler, TaskType},
};

use super::ContextFetcher;
use arpa_node_core::SchedulerResult;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub(crate) trait Chain {
    type BlockInfoCache;
    type RandomnessTasksQueue;
    type RandomnessResultCaches;
    type Context: Send + Sync;
    type ChainIdentity;

    async fn init_components(&self, context: &Self::Context) -> SchedulerResult<()> {
        self.init_listeners(context).await?;

        self.init_subscribers(context).await;

        Ok(())
    }

    async fn init_listener(
        &self,
        eq: Arc<RwLock<EventQueue>>,
        fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
        task_type: TaskType,
    ) -> SchedulerResult<()>;

    async fn init_listeners(&self, context: &Self::Context) -> SchedulerResult<()>;

    async fn init_subscribers(&self, context: &Self::Context);
}

#[async_trait]
pub(crate) trait MainChain: Chain {
    type NodeInfoCache;
    type GroupInfoCache;

    async fn init_block_listeners(&self, context: &Self::Context) -> SchedulerResult<()>;

    async fn init_dkg_listeners(&self, context: &Self::Context) -> SchedulerResult<()>;

    async fn init_randomness_listeners(&self, context: &Self::Context) -> SchedulerResult<()>;

    async fn init_block_subscribers(&self, context: &Self::Context);

    async fn init_dkg_subscribers(&self, context: &Self::Context);

    async fn init_randomness_subscribers(&self, context: &Self::Context);
}

pub(crate) trait ChainFetcher<T: Chain> {
    fn id(&self) -> usize;

    fn description(&self) -> &str;

    fn get_chain_identity(&self) -> Arc<RwLock<T::ChainIdentity>>;

    fn get_block_cache(&self) -> Arc<RwLock<T::BlockInfoCache>>;

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<T::RandomnessTasksQueue>>;

    fn get_randomness_result_cache(&self) -> Arc<RwLock<T::RandomnessResultCaches>>;
}

pub(crate) trait MainChainFetcher<T: MainChain> {
    fn get_node_cache(&self) -> Arc<RwLock<T::NodeInfoCache>>;

    fn get_group_cache(&self) -> Arc<RwLock<T::GroupInfoCache>>;
}
