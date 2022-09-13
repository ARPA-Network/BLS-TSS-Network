use super::Subscriber;
use crate::node::{
    dal::{cache::InMemoryBlockInfoCache, BlockInfoUpdater},
    error::NodeResult,
    event::{new_block::NewBlock, types::Topic, Event},
    queue::{event_queue::EventQueue, EventSubscriber},
};
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct BlockSubscriber {
    pub chain_id: usize,
    block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
    eq: Arc<RwLock<EventQueue>>,
}

impl BlockSubscriber {
    pub fn new(
        chain_id: usize,
        block_cache: Arc<RwLock<InMemoryBlockInfoCache>>,
        eq: Arc<RwLock<EventQueue>>,
    ) -> Self {
        BlockSubscriber {
            chain_id,
            block_cache,
            eq,
        }
    }
}

impl Subscriber for BlockSubscriber {
    fn notify(&self, topic: Topic, payload: Box<dyn Event>) -> NodeResult<()> {
        info!("{:?}", topic);

        unsafe {
            let ptr = Box::into_raw(payload);

            let struct_ptr = ptr as *mut NewBlock;

            let payload = *Box::from_raw(struct_ptr);

            self.block_cache
                .write()
                .set_block_height(payload.block_height);
        }

        Ok(())
    }

    fn subscribe(self) {
        let eq = self.eq.clone();

        let chain_id = self.chain_id;

        let subscriber = Box::new(self);

        eq.write().subscribe(Topic::NewBlock(chain_id), subscriber);
    }
}
