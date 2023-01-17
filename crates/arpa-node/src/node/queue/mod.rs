use async_trait::async_trait;

use super::{
    event::{types::Topic, Event},
    subscriber::DebuggableSubscriber,
};

pub mod event_queue;

pub trait EventSubscriber {
    fn subscribe(&mut self, topic: Topic, subscriber: Box<dyn DebuggableSubscriber>);
}

#[async_trait]
pub trait EventPublisher<E: Event + Clone + Send + Sync + 'static> {
    async fn publish(&self, event: E);
}
