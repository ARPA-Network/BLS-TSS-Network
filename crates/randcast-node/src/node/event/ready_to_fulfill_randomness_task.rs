use crate::node::dao::cache::RandomnessResultCache;

use super::types::{Event, Topic};

#[derive(Clone)]
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
}
