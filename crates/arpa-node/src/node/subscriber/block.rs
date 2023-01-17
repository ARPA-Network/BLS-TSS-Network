use super::{DebuggableEvent, DebuggableSubscriber, Subscriber};
use crate::node::{
    error::NodeResult,
    event::{new_block::NewBlock, types::Topic},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use arpa_node_dal::BlockInfoUpdater;
use async_trait::async_trait;
use log::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct BlockSubscriber<B: BlockInfoUpdater> {
    pub chain_id: usize,
    block_cache: Arc<RwLock<B>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl<B: BlockInfoUpdater> BlockSubscriber<B> {
    pub fn new(chain_id: usize, block_cache: Arc<RwLock<B>>, eq: Arc<RwLock<EventQueue>>) -> Self {
        BlockSubscriber {
            chain_id,
            block_cache,
            eq,
        }
    }
}

#[async_trait]
impl<B: BlockInfoUpdater + std::fmt::Debug + Sync + Send + 'static> Subscriber
    for BlockSubscriber<B>
{
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

impl<B: BlockInfoUpdater + std::fmt::Debug + Sync + Send + 'static> DebuggableSubscriber
    for BlockSubscriber<B>
{
}
