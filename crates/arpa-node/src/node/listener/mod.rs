pub mod block;
pub mod new_randomness_task;
pub mod post_commit_grouping;
pub mod post_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;

use crate::node::error::NodeResult;
use async_trait::async_trait;

#[async_trait]
pub trait Listener {
    async fn start(self) -> NodeResult<()>;
}
