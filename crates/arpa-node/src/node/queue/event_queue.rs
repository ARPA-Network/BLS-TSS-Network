use super::{EventPublisher, EventSubscriber};
use crate::node::{
    event::types::Topic,
    subscriber::{DebuggableEvent, DebuggableSubscriber},
};
use async_trait::async_trait;
use log::error;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct EventQueue {
    subscribers: HashMap<Topic, Vec<Box<dyn DebuggableSubscriber>>>,
}

impl EventQueue {
    pub fn new() -> Self {
        EventQueue {
            subscribers: HashMap::new(),
        }
    }
}

impl EventSubscriber for EventQueue {
    fn subscribe(&mut self, topic: Topic, subscriber: Box<dyn DebuggableSubscriber>) {
        self.subscribers.entry(topic).or_insert_with(Vec::new);

        self.subscribers.get_mut(&topic).unwrap().push(subscriber);
    }
}

#[async_trait]
impl<E: DebuggableEvent + Clone + Send + Sync + 'static> EventPublisher<E> for EventQueue {
    async fn publish(&self, event: E) {
        let topic = event.topic();

        if let Some(subscribers) = self.subscribers.get(&topic) {
            for subscriber in subscribers {
                if let Err(e) = subscriber.notify(topic, &event).await {
                    error!("{:?}", e);
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::EventPublisher;
    use crate::node::{
        event::new_block::NewBlock,
        listener::block::BlockListener,
        queue::event_queue::EventQueue,
        subscriber::{block::BlockSubscriber, Subscriber},
    };
    use arpa_node_core::{Config, GeneralChainIdentity};
    use arpa_node_dal::cache::InMemoryBlockInfoCache;
    use arpa_node_dal::BlockInfoFetcher;
    use ethers::types::Address;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test() {
        Config::default().initialize();

        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let chain_id = 1;

        let block_cache = Arc::new(RwLock::new(InMemoryBlockInfoCache::new()));

        assert_eq!(0, block_cache.clone().read().await.get_block_height());

        let s = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        s.subscribe().await;

        let fake_wallet = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let chain_identity = GeneralChainIdentity::new(
            0,
            fake_wallet,
            "localhost:8545".to_string(),
            3000,
            Address::random(),
            Address::random(),
        );

        let chain_identity = Arc::new(RwLock::new(chain_identity));

        let p = BlockListener::new(chain_id, chain_identity, eq);

        p.publish(NewBlock {
            chain_id,
            block_height: 1,
        })
        .await;

        assert_eq!(1, block_cache.clone().read().await.get_block_height());

        p.publish(NewBlock {
            chain_id,
            block_height: 10,
        })
        .await;

        assert_eq!(10, block_cache.clone().read().await.get_block_height());

        p.publish(NewBlock {
            chain_id: 999,
            block_height: 10,
        })
        .await;

        assert_eq!(10, block_cache.clone().read().await.get_block_height());
    }
}
