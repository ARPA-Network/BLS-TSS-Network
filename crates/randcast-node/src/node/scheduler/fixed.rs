use async_trait::async_trait;
use futures::Future;
use tokio::task::JoinHandle;

#[async_trait]
pub trait FixedTaskScheduler {
    fn add_task<T>(&mut self, future: T)
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static;

    async fn join(mut self);
}

#[derive(Default)]
pub struct SimpleFixedTaskScheduler {
    fixed_tasks: Vec<JoinHandle<()>>,
}

impl SimpleFixedTaskScheduler {
    pub fn new() -> Self {
        SimpleFixedTaskScheduler {
            fixed_tasks: vec![],
        }
    }
}

#[async_trait]
impl FixedTaskScheduler for SimpleFixedTaskScheduler {
    fn add_task<T>(&mut self, future: T)
    where
        T: Future<Output = ()> + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(future);
    }

    async fn join(mut self) {
        for fixed_task in self.fixed_tasks.iter_mut() {
            let _ = fixed_task.await;
        }
    }
}
