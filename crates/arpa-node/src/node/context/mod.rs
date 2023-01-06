pub mod chain;
pub mod types;

use self::types::ContextHandle;

use crate::node::{
    queue::event_queue::EventQueue,
    scheduler::{dynamic::SimpleDynamicTaskScheduler, fixed::SimpleFixedTaskScheduler},
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

pub trait Context {
    type MainChain;

    type AdapterChain;

    async fn deploy(self) -> ContextHandle;
}

#[async_trait]
pub trait TaskWaiter {
    async fn wait_task(&self);
}

pub(crate) trait ContextFetcher<T: Context> {
    fn contains_chain(&self, index: usize) -> bool;

    fn get_adapter_chain(&self, index: usize) -> Option<&T::AdapterChain>;

    fn get_main_chain(&self) -> &T::MainChain;

    fn get_fixed_task_handler(&self) -> Arc<RwLock<SimpleFixedTaskScheduler>>;

    fn get_dynamic_task_handler(&self) -> Arc<RwLock<SimpleDynamicTaskScheduler>>;

    fn get_event_queue(&self) -> Arc<RwLock<EventQueue>>;
}

pub(crate) trait CommitterServerStarter<T: Context> {
    fn start_committer_server(&mut self, rpc_endpoint: String, context: Arc<RwLock<T>>);
}
