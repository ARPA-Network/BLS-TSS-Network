pub mod block;
pub mod in_grouping;
pub mod post_grouping;
pub mod post_success_grouping;
pub mod pre_grouping;
pub mod randomness_signature_aggregation;
pub mod ready_to_handle_randomness_task;

use crate::{
    error::NodeResult,
    event::{types::Topic, Event},
};
use async_trait::async_trait;

pub trait DebuggableEvent: Event + std::fmt::Debug + Send + Sync {}

pub trait DebuggableSubscriber: Subscriber + std::fmt::Debug + Send + Sync {}

#[async_trait]
pub trait Subscriber {
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()>;

    async fn subscribe(self);
}
