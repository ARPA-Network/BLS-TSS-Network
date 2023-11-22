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
    chain_id: usize,
    block_cache: Arc<RwLock<Box<dyn BlockInfoHandler>>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl BlockSubscriber {
    pub fn new(
        chain_id: usize,
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
    async fn notify(&self, topic: Topic, payload: &(dyn DebuggableEvent)) -> NodeResult<()> {
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
