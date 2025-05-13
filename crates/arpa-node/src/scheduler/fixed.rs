use crate::listener::Listener;

use super::{ComponentTaskType, FixedTaskScheduler, TaskScheduler};
use arpa_core::{SchedulerError, SchedulerResult};
use async_trait::async_trait;
use futures::Future;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct TaskHandle {
    listener: Option<Arc<dyn Listener + Send + Sync + 'static>>,
    handle: JoinHandle<()>,
}

#[derive(Debug, Default)]
pub struct SimpleFixedTaskScheduler {
    fixed_tasks: HashMap<ComponentTaskType, TaskHandle>,
}

impl SimpleFixedTaskScheduler {
    pub fn new() -> Self {
        SimpleFixedTaskScheduler {
            fixed_tasks: HashMap::new(),
        }
    }
}

#[async_trait]
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
        let task_handle = TaskHandle {
            listener: None,
            handle,
        };
        self.fixed_tasks.insert(task_type, task_handle);
        Ok(())
    }

    async fn shutdown(&mut self) {
        info!("stop fixed tasks...");
        for (task_type, task_handle) in self.fixed_tasks.iter_mut() {
            info!("stop task: {:?}", task_type);
            task_handle.handle.abort();
        }

        // clear task list
        self.fixed_tasks.clear();
        info!("fixed tasks stopped");
    }
}

#[async_trait]
impl FixedTaskScheduler for SimpleFixedTaskScheduler {
    async fn join(self) {
        for (_, fixed_task) in self.fixed_tasks.into_iter() {
            let _ = fixed_task.handle.await;
        }
    }

    async fn abort(&mut self, task_type: &ComponentTaskType) -> SchedulerResult<()> {
        if !self.fixed_tasks.contains_key(task_type) {
            return Err(SchedulerError::TaskNotFound);
        }
        let handle = self.fixed_tasks.remove(task_type).unwrap();
        handle.handle.abort();
        Ok(())
    }

    fn get_tasks(&self) -> Vec<&ComponentTaskType> {
        self.fixed_tasks.keys().collect::<Vec<&ComponentTaskType>>()
    }

    fn add_listener_task(
        &mut self,
        listener: impl Listener + Send + Sync + 'static,
    ) -> SchedulerResult<()> {
        let task_type = ComponentTaskType::Listener(
            listener.listener_descriptor().chain_id,
            listener.listener_descriptor().l_type,
        );

        if self.fixed_tasks.contains_key(&task_type) {
            return Err(SchedulerError::TaskAlreadyExisted);
        }

        let mut mdc = vec![];
        log_mdc::iter(|k, v| mdc.push((k.to_owned(), v.to_owned())));

        let listener = Arc::new(listener);

        let listener_clone = listener.clone();

        let handle = tokio::spawn(async move {
            log_mdc::extend(mdc);
            if let Err(e) = listener_clone.start().await {
                error!("listener start error: {:?}", e);
            };
        });
        let task_handle = TaskHandle {
            listener: Some(listener),
            handle,
        };
        self.fixed_tasks.insert(task_type, task_handle);
        Ok(())
    }

    fn restart_listener(&mut self, task_type: &ComponentTaskType) -> SchedulerResult<()> {
        if !self.fixed_tasks.contains_key(task_type) {
            return Err(SchedulerError::TaskNotFound);
        }
        match task_type {
            ComponentTaskType::Listener(_, _) => {
                let task_handle = self.fixed_tasks.get_mut(task_type).unwrap();
                task_handle.handle.abort();

                let listener = task_handle.listener.take().unwrap();

                let listener_clone = listener.clone();

                let mut mdc = vec![];
                log_mdc::iter(|k, v| mdc.push((k.to_owned(), v.to_owned())));

                let handle = tokio::spawn(async move {
                    log_mdc::extend(mdc);
                    let _ = listener_clone.start().await;
                });

                task_handle.handle = handle;
                task_handle.listener = Some(listener);

                Ok(())
            }
            _ => Err(SchedulerError::TaskNotFound),
        }
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
