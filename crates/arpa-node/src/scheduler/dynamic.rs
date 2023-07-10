use arpa_core::SchedulerResult;
use futures::Future;
use tokio::{
    sync::{oneshot::channel, oneshot::Receiver},
    task::JoinHandle,
};

use super::{DynamicTaskScheduler, TaskScheduler, TaskType};

#[derive(Debug, Default)]
pub struct SimpleDynamicTaskScheduler {
    // TODO access control
    pub dynamic_tasks: Vec<(Receiver<()>, Option<JoinHandle<()>>)>,
}

impl SimpleDynamicTaskScheduler {
    pub fn new() -> Self {
        SimpleDynamicTaskScheduler {
            dynamic_tasks: vec![],
        }
    }
}

impl TaskScheduler for SimpleDynamicTaskScheduler {
    fn add_task<T>(&mut self, _: TaskType, future: T) -> SchedulerResult<()>
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
    {
        let (send, recv) = channel::<()>();

        let mut mdc = vec![];
        log_mdc::iter(|k, v| mdc.push((k.to_owned(), v.to_owned())));

        tokio::spawn(async move {
            log_mdc::extend(mdc);
            future.await;
            drop(send);
        });

        self.dynamic_tasks.push((recv, None));

        Ok(())
    }
}

impl DynamicTaskScheduler for SimpleDynamicTaskScheduler {
    fn add_task_with_shutdown_signal<T, P, F>(
        &mut self,
        future: T,
        shutdown_predicate: P,
        shutdown_check_frequency: u64,
    ) where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
        P: Fn() -> F + Sync + Send + 'static,
        F: Future<Output = bool> + Send + 'static,
    {
        let (send, recv) = channel::<()>();

        let task = tokio::spawn(async move {
            future.await;
            drop(send);
        });

        let task_monitor = tokio::spawn(async move {
            loop {
                if shutdown_predicate().await {
                    task.abort();
                    return;
                }

                tokio::time::sleep(std::time::Duration::from_millis(shutdown_check_frequency))
                    .await;
            }
        });

        self.dynamic_tasks.push((recv, Some(task_monitor)));
    }
}

#[cfg(test)]
pub mod tests {

    use std::time::Duration;
    use tokio::task;
    use tokio::time;

    #[tokio::test]
    async fn test() {
        let original_task = task::spawn(async {
            let _detached_task = task::spawn(async {
                // Here we sleep to make sure that the first task returns before.
                time::sleep(Duration::from_millis(10)).await;
                // This will be called, even though the JoinHandle is dropped.
                println!("♫ Still alive ♫");
            });
        });

        original_task
            .await
            .expect("The task being joined has panicked");
        println!("Original task is joined.");

        // We make sure that the new task has time to run, before the main
        // task returns.

        time::sleep(Duration::from_millis(1000)).await;
    }

    #[tokio::test]
    async fn test_abort() {
        let task_1 = tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                println!("1");
            }
        });

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            task_1.abort();
        });

        tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
    }
}
