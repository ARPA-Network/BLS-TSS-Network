use crate::node::error::errors::NodeResult;
use async_trait::async_trait;

#[async_trait]
pub trait Listener {
    async fn start(self) -> NodeResult<()>;
}
