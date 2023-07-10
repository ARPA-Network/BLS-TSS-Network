use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;
use arpa_dal::cache::RandomnessResultCache;

#[derive(Clone, Debug)]
pub struct ReadyToFulfillRandomnessTask {
    pub chain_id: usize,
    pub tasks: Vec<RandomnessResultCache>,
}

impl ReadyToFulfillRandomnessTask {
    pub fn new(chain_id: usize, tasks: Vec<RandomnessResultCache>) -> Self {
        ReadyToFulfillRandomnessTask { chain_id, tasks }
    }
}

impl Event for ReadyToFulfillRandomnessTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToFulfillRandomnessTask(self.chain_id)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for ReadyToFulfillRandomnessTask {}
