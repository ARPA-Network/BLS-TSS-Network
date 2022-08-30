use crate::node::{
    event::types::{Event, Topic},
    subscriber::types::Subscriber,
};
use log::error;
use std::collections::HashMap;

#[derive(Default)]
pub struct EventQueue {
    subscribers: HashMap<Topic, Vec<Box<dyn Subscriber + Send + Sync>>>,
}

impl EventQueue {
    pub fn new() -> Self {
        EventQueue {
            subscribers: HashMap::new(),
        }
    }
}

pub trait EventSubscriber {
    fn subscribe(&mut self, topic: Topic, subscriber: Box<dyn Subscriber + Send + Sync>);
}

impl EventSubscriber for EventQueue {
    fn subscribe(&mut self, topic: Topic, subscriber: Box<dyn Subscriber + Send + Sync>) {
        self.subscribers.entry(topic).or_insert_with(Vec::new);

        self.subscribers.get_mut(&topic).unwrap().push(subscriber);
    }
}

pub trait EventPublisher<E: Event + Clone + Send + Sync + 'static> {
    fn publish(&self, event: E);
}

impl<E: Event + Clone + Send + Sync + 'static> EventPublisher<E> for EventQueue {
    fn publish(&self, event: E) {
        let topic = event.topic();

        let s_ptr = Box::new(event);

        if let Some(subscribers) = self.subscribers.get(&topic) {
            for subscriber in subscribers {
                if let Err(e) = subscriber.notify(topic, s_ptr.clone()) {
                    error!("{:?}", e);
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use parking_lot::RwLock;

    use crate::node::{
        dal::{api::BlockInfoFetcher, cache::InMemoryBlockInfoCache, types::ChainIdentity},
        event::new_block::NewBlock,
        listener::block::MockBlockListener,
        queue::event_queue::EventQueue,
        subscriber::{block::BlockSubscriber, types::Subscriber},
    };

    use super::EventPublisher;

    #[test]
    fn test() {
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let chain_id = 1;

        let block_cache = Arc::new(RwLock::new(InMemoryBlockInfoCache::new()));

        assert_eq!(0, block_cache.clone().read().get_block_height());

        let s = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        s.subscribe();

        let chain_identity = ChainIdentity::new(0, vec![], "".to_string(), "".to_string());

        let chain_identity = Arc::new(RwLock::new(chain_identity));

        let p = MockBlockListener::new(chain_id, "".to_string(), chain_identity, eq);

        p.publish(NewBlock {
            chain_id,
            block_height: 1,
        });

        assert_eq!(1, block_cache.clone().read().get_block_height());

        p.publish(NewBlock {
            chain_id,
            block_height: 10,
        });

        assert_eq!(10, block_cache.clone().read().get_block_height());

        p.publish(NewBlock {
            chain_id: 999,
            block_height: 10,
        });

        assert_eq!(10, block_cache.clone().read().get_block_height());
    }
}
