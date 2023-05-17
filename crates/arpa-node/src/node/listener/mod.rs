pub mod block;
pub mod new_randomness_task;
pub mod post_commit_grouping;
pub mod post_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;

use crate::node::error::NodeResult;
use arpa_node_core::jitter;
use async_trait::async_trait;
use log::error;
use tokio::time::sleep;
use tokio_retry::strategy::FixedInterval;

#[async_trait]
pub trait Listener {
    async fn start(&self, interval_millis: u64, use_jitter: bool) -> NodeResult<()> {
        let mut retry_strategy =
            FixedInterval::from_millis(interval_millis)
                .map(|e| if use_jitter { jitter(e) } else { e });

        loop {
            if let Err(err) = self.listen().await {
                error!("listener is interrupted. Retry... Error: {:?}, ", err);
            }
            sleep(retry_strategy.next().unwrap()).await;
        }
    }

    async fn listen(&self) -> NodeResult<()>;
}
