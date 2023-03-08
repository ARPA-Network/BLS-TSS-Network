pub mod chain;
pub mod types;

use self::types::{Config, ContextHandle};

use crate::node::{
    queue::event_queue::EventQueue,
    scheduler::{dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler},
};
use arpa_node_core::SchedulerResult;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

pub trait Context {
    type MainChain;

    async fn deploy(self) -> SchedulerResult<ContextHandle>;
}

#[async_trait]
pub trait TaskWaiter {
    async fn wait_task(&self);
}

pub(crate) trait ContextFetcher<T: Context> {
    fn get_config(&self) -> &Config;

    fn get_main_chain(&self) -> &T::MainChain;

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>>;

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>>;

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>>;
}

pub(crate) trait CommitterServerStarter<T: Context> {
    fn start_committer_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<T>>,
    ) -> SchedulerResult<()>;
}

pub(crate) trait ManagementServerStarter<T: Context> {
    fn start_management_server(
        &mut self,
        rpc_endpoint: String,
        context: Arc<RwLock<T>>,
    ) -> SchedulerResult<()>;
}
