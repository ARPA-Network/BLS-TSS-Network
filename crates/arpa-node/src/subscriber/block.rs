use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::{
    error::NodeResult,
    event::{new_block::NewBlock, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_dal::BlockInfoHandler;
use async_trait::async_trait;
use log::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BlockSubscriber {
    chain_id: u64,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl BlockSubscriber {
    pub fn new(
        chain_id: u64,
        block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        BlockSubscriber {
            chain_id,
            block_cache,
            eq,
        }
    }
}

#[async_trait]
impl Subscriber for BlockSubscriber {
    async fn notify(&self, topic: Topic, payload: &dyn DebuggableEvent) -> NodeResult<()> {
        debug!("{:?}", topic);

        let &NewBlock { block_height, .. } = payload.as_any().downcast_ref::<NewBlock>().unwrap();

        self.block_cache
            .write()
            .await
            .set_block_height(block_height);

        Ok(())
    }

    async fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write()
            .await
            .subscribe(Topic::NewBlock(chain_id), subscriber);
    }
}

impl DebuggableSubscriber for BlockSubscriber {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{new_block::NewBlock, types::Topic, Event},
        queue::event_queue::EventQueue,
    };
    use arpa_dal::{BlockInfoHandler, BlockInfoFetcher, BlockInfoUpdater};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug)]
    struct MockBlockInfoHandler {
        chain_id: usize,
        block_height: Arc<RwLock<usize>>,
        block_time: usize,
    }

    impl MockBlockInfoHandler {
        fn new(chain_id: usize, initial_block_height: usize, block_time: usize) -> Self {
            Self {
                chain_id,
                block_height: Arc::new(RwLock::new(initial_block_height)),
                block_time,
            }
        }
    }

    impl BlockInfoFetcher for MockBlockInfoHandler {
        fn get_chain_id(&self) -> usize {
            self.chain_id
        }

        fn get_block_height(&self) -> usize {
            futures::executor::block_on(async { *self.block_height.read().await })
        }

        fn get_block_time(&self) -> usize {
            self.block_time
        }
    }

    impl BlockInfoUpdater for MockBlockInfoHandler {
        fn set_block_height(&mut self, block_height: usize) {
            if let Ok(mut height) = self.block_height.try_write() {
                *height = block_height;
            }
        }
    }

    impl BlockInfoHandler for MockBlockInfoHandler {}

    #[tokio::test]
    async fn test_block_subscriber_creation() {
        let chain_id = 1;
        let block_cache = Arc::new(RwLock::new(
            Box::new(MockBlockInfoHandler::new(chain_id, 100, 12)) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        assert_eq!(subscriber.chain_id, chain_id);
    }

    #[tokio::test]
    async fn test_notify_updates_block_height() {
        let chain_id = 1;
        let initial_height = 100;
        let mock_handler = MockBlockInfoHandler::new(chain_id, initial_height, 12);
        let block_cache = Arc::new(RwLock::new(
            Box::new(mock_handler) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        let new_block_height = 150;
        let new_block_event = NewBlock::new(chain_id, new_block_height);

        let result = subscriber
            .notify(Topic::NewBlock(chain_id), &new_block_event)
            .await;

        assert!(result.is_ok());

        let cache_guard = block_cache.read().await;
        let current_height = cache_guard.get_block_height();
        assert_eq!(current_height, new_block_height);
    }

    #[tokio::test]
    async fn test_notify_with_different_chain_id() {
        let chain_id = 1;
        let different_chain_id = 2;
        let initial_height = 100;
        let mock_handler = MockBlockInfoHandler::new(chain_id, initial_height, 12);
        let block_cache = Arc::new(RwLock::new(
            Box::new(mock_handler) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

        let new_block_event = NewBlock::new(chain_id, 150);
        let result = subscriber
            .notify(Topic::NewBlock(different_chain_id), &new_block_event)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_registers_with_event_queue() {
        let chain_id = 1;
        let block_cache = Arc::new(RwLock::new(
            Box::new(MockBlockInfoHandler::new(chain_id, 100, 12)) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());
        subscriber.subscribe().await;
    }

    #[tokio::test]
    async fn test_multiple_notify_calls() {
        let chain_id = 1;
        let initial_height = 100;
        let mock_handler = MockBlockInfoHandler::new(chain_id, initial_height, 12);
        let block_cache = Arc::new(RwLock::new(
            Box::new(mock_handler) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());
        
        let first_block = NewBlock::new(chain_id, 150);
        let result1 = subscriber
            .notify(Topic::NewBlock(chain_id), &first_block)
            .await;
        assert!(result1.is_ok());
        
        let second_block = NewBlock::new(chain_id, 200);
        let result2 = subscriber
            .notify(Topic::NewBlock(chain_id), &second_block)
            .await;
        assert!(result2.is_ok());
        
        let cache_guard = block_cache.read().await;
        let final_height = cache_guard.get_block_height();
        assert_eq!(final_height, 200);
    }

    #[tokio::test]
    async fn test_debuggable_subscriber_trait() {
        let chain_id = 1;
        let block_cache = Arc::new(RwLock::new(
            Box::new(MockBlockInfoHandler::new(chain_id, 100, 12)) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache, eq);
        let _: &dyn DebuggableSubscriber = &subscriber;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_notify_with_wrong_event_type() {
        let chain_id = 1;
        let block_cache = Arc::new(RwLock::new(
            Box::new(MockBlockInfoHandler::new(chain_id, 100, 12)) as Box<dyn BlockInfoHandler>
        ));
        let eq = Arc::new(RwLock::new(EventQueue::new()));

        let subscriber = BlockSubscriber::new(chain_id, block_cache, eq);
        
        #[derive(Debug)]
        struct WrongEvent;

        impl Event for WrongEvent {
            fn topic(&self) -> Topic {
                Topic::NewBlock(1) 
            }
        
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
        
        impl DebuggableEvent for WrongEvent {}

        let wrong_event = WrongEvent;
        let _result = subscriber
            .notify(Topic::NewBlock(chain_id), &wrong_event)
            .await;
    }

    mod integration_tests {
        use super::*;

        #[tokio::test]
        async fn test_block_subscriber_integration() {
            let chain_id = 1;
            let mock_handler = MockBlockInfoHandler::new(chain_id, 0, 12);
            
            let block_cache = Arc::new(RwLock::new(
                Box::new(mock_handler) as Box<dyn BlockInfoHandler>
            ));
            let eq = Arc::new(RwLock::new(EventQueue::new()));

            let subscriber_for_subscribe = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());
            subscriber_for_subscribe.subscribe().await;
            let subscriber = BlockSubscriber::new(chain_id, block_cache.clone(), eq.clone());

            let heights = vec![10, 20, 30, 40, 50];
            
            for height in heights.iter() {
                let new_block = NewBlock::new(chain_id, *height);
                
                let result = subscriber
                    .notify(Topic::NewBlock(chain_id), &new_block)
                    .await;
                
                assert!(result.is_ok());
            }

            let cache_guard = block_cache.read().await;
            let final_height = cache_guard.get_block_height();
            assert_eq!(final_height, 50);
        }
    }
}