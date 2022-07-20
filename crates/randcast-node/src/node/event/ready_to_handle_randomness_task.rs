use crate::node::dao::types::RandomnessTask;

use super::types::{Event, Topic};

#[derive(Clone)]
pub struct ReadyToHandleRandomnessTask {
    pub chain_id: usize,
    pub tasks: Vec<RandomnessTask>,
}

impl ReadyToHandleRandomnessTask {
    pub fn new(chain_id: usize, tasks: Vec<RandomnessTask>) -> Self {
        ReadyToHandleRandomnessTask { chain_id, tasks }
    }
}

impl Event for ReadyToHandleRandomnessTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToHandleRandomnessTask(self.chain_id)
    }
}
