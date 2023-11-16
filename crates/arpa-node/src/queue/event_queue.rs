use super::{EventPublisher, EventSubscriber};
use crate::{
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
    use crate::{
        context::ChainIdentityHandlerType,
        event::new_block::NewBlock,
        listener::block::BlockListener,
        queue::event_queue::EventQueue,
        subscriber::{block::BlockSubscriber, Subscriber},
    };
    use arpa_core::{Config, GeneralMainChainIdentity};
    use arpa_dal::{cache::InMemoryBlockInfoCache, BlockInfoHandler};
    use ethers::{
        providers::{Provider, Ws},
        types::Address,
        utils::Anvil,
    };
    use std::{sync::Arc, time::Duration};
    use threshold_bls::schemes::bn254::G2Curve;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test() {
        let config = Config::default();

        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let chain_id = 1;

        let block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>> =
            Arc::new(RwLock::new(Box::new(InMemoryBlockInfoCache::new(12))));

        assert_eq!(0, block_cache.clone().read().await.get_block_height());

        let s = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        s.subscribe().await;

        let fake_wallet = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let contract_transaction_retry_descriptor = config
            .get_time_limits()
            .contract_transaction_retry_descriptor;

        let contract_view_retry_descriptor =
            config.get_time_limits().contract_view_retry_descriptor;

        let avnil = Anvil::new().spawn();

        let provider = Arc::new(
            Provider::<Ws>::connect(avnil.ws_endpoint())
                .await
                .unwrap()
                .interval(Duration::from_millis(3000)),
        );

        let chain_identity = GeneralMainChainIdentity::new(
            0,
            fake_wallet,
            provider,
            avnil.ws_endpoint(),
            Address::random(),
            Address::random(),
            Address::random(),
            contract_transaction_retry_descriptor,
            contract_view_retry_descriptor,
        );

        let chain_identity: Arc<RwLock<ChainIdentityHandlerType<G2Curve>>> =
            Arc::new(RwLock::new(Box::new(chain_identity)));

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
