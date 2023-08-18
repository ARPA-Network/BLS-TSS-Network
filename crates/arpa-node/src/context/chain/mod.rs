pub mod types;
use crate::{queue::event_queue::EventQueue, scheduler::fixed::SimpleFixedTaskScheduler};

use arpa_core::{ListenerDescriptor, SchedulerResult};
use async_trait::async_trait;
use std::sync::Arc;
use threshold_bls::{
    group::Curve,
    sig::{SignatureScheme, ThresholdScheme},
};
use tokio::sync::RwLock;

use super::ContextFetcher;

#[async_trait]
pub trait Chain<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>: std::fmt::Debug
{
    type NodeInfoCache;
    type GroupInfoCache;
    type BlockInfoCache;
    type RandomnessTasksQueue;
    type RandomnessResultCaches;
    type ChainIdentity;

    fn id(&self) -> usize;

    fn description(&self) -> &str;

    fn get_chain_identity(&self) -> Arc<RwLock<Self::ChainIdentity>>;

    fn get_block_cache(&self) -> Arc<RwLock<Self::BlockInfoCache>>;

    fn get_node_cache(&self) -> Arc<RwLock<Self::NodeInfoCache>>;

    fn get_group_cache(&self) -> Arc<RwLock<Self::GroupInfoCache>>;

    fn get_randomness_tasks_cache(&self) -> Arc<RwLock<Self::RandomnessTasksQueue>>;

    fn get_randomness_result_cache(&self) -> Arc<RwLock<Self::RandomnessResultCaches>>;

    async fn init_components(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_listener(
        &self,
        eq: Arc<RwLock<EventQueue>>,
        fs: Arc<RwLock<SimpleFixedTaskScheduler>>,
        listener: ListenerDescriptor,
    ) -> SchedulerResult<()>;

    async fn init_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));
}

#[async_trait]
pub trait MainChain<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>: Chain<PC, S>
{
    async fn init_block_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_dkg_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_randomness_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_block_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));

    async fn init_dkg_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));

    async fn init_randomness_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));
}

#[async_trait]
pub trait RelayedChain<
    PC: Curve,
    S: SignatureScheme + ThresholdScheme<Public = PC::Point, Private = PC::Scalar>,
>: Chain<PC, S>
{
    async fn init_block_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_randomness_listeners(
        &self,
        context: &(dyn ContextFetcher + Sync + Send),
    ) -> SchedulerResult<()>;

    async fn init_block_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));

    async fn init_randomness_subscribers(&self, context: &(dyn ContextFetcher + Sync + Send));
}
