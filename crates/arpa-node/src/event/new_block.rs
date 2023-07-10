use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;

#[derive(Clone, Debug)]
pub struct NewBlock {
    pub chain_id: usize,
    pub block_height: usize,
}

impl NewBlock {
    pub fn new(chain_id: usize, block_height: usize) -> Self {
        NewBlock {
            chain_id,
            block_height,
        }
    }
}

impl Event for NewBlock {
    fn topic(&self) -> Topic {
        Topic::NewBlock(self.chain_id)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for NewBlock {}
