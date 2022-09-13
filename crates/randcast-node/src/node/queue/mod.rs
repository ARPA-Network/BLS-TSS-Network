use super::{
    event::{types::Topic, Event},
    subscriber::Subscriber,
};

pub mod event_queue;

pub trait EventSubscriber {
    fn subscribe(&mut self, topic: Topic, subscriber: Box<dyn Subscriber + Send + Sync>);
}

pub trait EventPublisher<E: Event + Clone + Send + Sync + 'static> {
    fn publish(&self, event: E);
}
