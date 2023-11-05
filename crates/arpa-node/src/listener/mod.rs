pub mod block;
pub mod new_randomness_task;
pub mod post_commit_grouping;
pub mod post_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;

use crate::error::NodeResult;
use arpa_core::{jitter, FixedIntervalRetryDescriptor};
use async_trait::async_trait;
use log::error;
use tokio::time::sleep;
use tokio_retry::{strategy::FixedInterval, Retry};

#[async_trait]
pub trait Listener {
    async fn start(
        &self,
        interval_millis: u64,
        use_jitter: bool,
        reset_descriptor: FixedIntervalRetryDescriptor,
    ) -> NodeResult<()> {
        let mut next_polling_strategy =
            FixedInterval::from_millis(interval_millis)
                .map(|e| if use_jitter { jitter(e) } else { e });

        loop {
            if let Err(err) = self.listen().await {
                error!("Listener is interrupted. Retry... Error: {:?}.", err);

                let reset_strategy = FixedInterval::from_millis(reset_descriptor.interval_millis)
                    .map(|e| {
                        if reset_descriptor.use_jitter {
                            jitter(e)
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
            sleep(next_polling_strategy.next().unwrap()).await;
        }
    }

    async fn listen(&self) -> NodeResult<()>;

    async fn handle_interruption(&self) -> NodeResult<()> {
        Ok(())
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
