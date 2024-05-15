use super::{ComponentTaskType, FixedTaskScheduler, TaskScheduler};
use arpa_core::{SchedulerError, SchedulerResult};
use async_trait::async_trait;
use futures::Future;
use std::collections::HashMap;
use tokio::task::JoinHandle;

#[derive(Debug, Default)]
pub struct SimpleFixedTaskScheduler {
    fixed_tasks: HashMap<ComponentTaskType, JoinHandle<()>>,
}

impl SimpleFixedTaskScheduler {
    pub fn new() -> Self {
        SimpleFixedTaskScheduler {
            fixed_tasks: HashMap::new(),
        }
    }
}

impl TaskScheduler for SimpleFixedTaskScheduler {
    fn add_task(
        &mut self,
        task_type: ComponentTaskType,
        future: impl Future + Send + 'static,
    ) -> SchedulerResult<()> {
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

    async fn abort(&mut self, task_type: &ComponentTaskType) -> SchedulerResult<()> {
        if !self.fixed_tasks.contains_key(task_type) {
            return Err(SchedulerError::TaskNotFound);
        }
        let handle = self.fixed_tasks.remove(task_type).unwrap();
        handle.abort();
        Ok(())
    }

    fn get_tasks(&self) -> Vec<&ComponentTaskType> {
        self.fixed_tasks.keys().collect::<Vec<&ComponentTaskType>>()
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
