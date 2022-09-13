use async_trait::async_trait;

use self::error::NodeResult;

pub mod error;

pub mod dal;

pub mod context;

pub mod queue;

pub mod subscriber;

pub mod listener;

pub mod event;

pub mod scheduler;

pub mod algorithm;

pub mod contract_client;

pub mod committer;

#[async_trait]
pub trait ServiceClient<C> {
    async fn prepare_service_client(&self) -> NodeResult<C>;
}
