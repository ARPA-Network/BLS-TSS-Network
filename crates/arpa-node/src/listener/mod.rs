pub mod block;
pub mod new_randomness_task;
pub mod post_commit_grouping;
pub mod post_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;
pub mod schedule_node_activation;
pub mod schedule_provider_reconnection;
use crate::error::NodeResult;
use arpa_core::jitter;
use arpa_core::log::{build_general_payload, LogType};
use arpa_core::ListenerDescriptor;
use async_trait::async_trait;
use log::{debug, error};
use std::fmt::Debug;
use std::fmt::Display;
use std::time::Duration;
use tokio::time::sleep;
use tokio_retry::{strategy::FixedInterval, Retry};

#[async_trait]
pub trait Listener: Debug + Display {
    async fn start(&self) -> NodeResult<()> {
        let interval_millis = self.listener_descriptor().interval_millis;
        let use_jitter = self.listener_descriptor().use_jitter;
        let reset_descriptor = self.listener_descriptor().reset_descriptor;
        let jitter_fn = if let Some(jitter_fn) = self.jitter_fn() {
            jitter_fn
        } else {
            Box::new(jitter)
        };

        let mut next_polling_strategy = FixedInterval::from_millis(interval_millis).map(|e| {
            if use_jitter {
                jitter_fn(e)
            } else {
                e
            }
        });

        loop {
            if let Err(err) = self.listen().await {
                error!(
                    "{}",
                    build_general_payload(
                        LogType::ListenerInterrupted,
                        &format!("{} is interrupted. Retry... Error: {:?}.", self, err),
                        Some(self.chain_id())
                    )
                );

                let reset_strategy = FixedInterval::from_millis(reset_descriptor.interval_millis)
                    .map(|e| {
                        if reset_descriptor.use_jitter {
                            jitter_fn(e)
                        } else {
                            e
                        }
                    })
                    .take(reset_descriptor.max_attempts);

                Retry::spawn(reset_strategy, || async {
                    self.handle_interruption().await
                })
                .await?;
            }
            debug!(
                "{} chain {} is sleeping for {:?}.",
                self,
                self.chain_id(),
                next_polling_strategy.next().unwrap()
            );
            sleep(next_polling_strategy.next().unwrap()).await;
        }
    }

    async fn initialize(&mut self) -> NodeResult<()> {
        Ok(())
    }

    async fn listen(&self) -> NodeResult<()>;

    async fn handle_interruption(&self) -> NodeResult<()> {
        Ok(())
    }

    fn chain_id(&self) -> usize;

    fn listener_descriptor(&self) -> ListenerDescriptor;

    fn jitter_fn(&self) -> Option<Box<dyn Fn(Duration) -> Duration + Send + Sync>> {
        None
    }
}

#[cfg(test)]
pub mod tests {
    use arpa_core::jitter;
    use tokio_retry::strategy::FixedInterval;

    #[tokio::test]
    async fn test() {
        let mut s = FixedInterval::from_millis(1000).map(jitter);
        for _ in 0..10 {
            println!("{:?}", s.next().unwrap());
        }
    }
}
