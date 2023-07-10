use super::{FixedTaskScheduler, TaskScheduler, TaskType};
use arpa_core::{SchedulerError, SchedulerResult};
use async_trait::async_trait;
use futures::Future;
use std::collections::HashMap;
use tokio::task::JoinHandle;

#[derive(Debug, Default)]
pub struct SimpleFixedTaskScheduler {
    fixed_tasks: HashMap<TaskType, JoinHandle<()>>,
}

impl SimpleFixedTaskScheduler {
    pub fn new() -> Self {
        SimpleFixedTaskScheduler {
            fixed_tasks: HashMap::new(),
        }
    }
}

impl TaskScheduler for SimpleFixedTaskScheduler {
    fn add_task<T>(&mut self, task_type: TaskType, future: T) -> SchedulerResult<()>
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
    {
        if self.fixed_tasks.contains_key(&task_type) {
            return Err(SchedulerError::TaskAlreadyExisted);
        }

        let mut mdc = vec![];
        log_mdc::iter(|k, v| mdc.push((k.to_owned(), v.to_owned())));

        let handle = tokio::spawn(async move {
            log_mdc::extend(mdc);
            future.await;
        });
        self.fixed_tasks.insert(task_type, handle);
        Ok(())
    }
}

#[async_trait]
impl FixedTaskScheduler for SimpleFixedTaskScheduler {
    async fn join(mut self) {
        for (_, fixed_task) in self.fixed_tasks.iter_mut() {
            let _ = fixed_task.await;
        }
    }

    async fn abort(&mut self, task_type: &TaskType) -> SchedulerResult<()> {
        if !self.fixed_tasks.contains_key(task_type) {
            return Err(SchedulerError::TaskNotFound);
        }
        let handle = self.fixed_tasks.remove(task_type).unwrap();
        handle.abort();
        Ok(())
    }

    fn get_tasks(&self) -> Vec<&TaskType> {
        self.fixed_tasks.keys().collect::<Vec<&TaskType>>()
    }
}

#[cfg(test)]
pub mod tests {

    use tokio::time;

    #[tokio::test]
    async fn test() {
        let mut handles = vec![];
        handles.push(tokio::spawn(async {
            time::sleep(time::Duration::from_secs(10)).await;
            println!("finished");
            true
        }));

        handles.push(tokio::spawn(async {
            time::sleep(time::Duration::from_secs(10)).await;
            println!("finished");
            false
        }));

        for handle in &handles {
            handle.abort();
        }

        println!("main finished");
    }
}
