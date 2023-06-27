pub mod dynamic;
pub mod fixed;
use arpa_core::{SchedulerResult, TaskType};
use async_trait::async_trait;
use futures::Future;

pub trait TaskScheduler {
    fn add_task<T>(&mut self, task_type: TaskType, future: T) -> SchedulerResult<()>
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static;
}

#[async_trait]
pub trait FixedTaskScheduler: TaskScheduler {
    async fn join(mut self);

    async fn abort(&mut self, task_type: &TaskType) -> SchedulerResult<()>;

    fn get_tasks(&self) -> Vec<&TaskType>;
}

pub trait DynamicTaskScheduler: TaskScheduler {
    fn add_task_with_shutdown_signal<T, P, F>(
        &mut self,
        future: T,
        shutdown_predicate: P,
        shutdown_check_frequency: u64,
    ) where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
        P: Fn() -> F + Sync + Send + 'static,
        F: Future<Output = bool> + Send + 'static;
}
