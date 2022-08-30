use crate::node::dal::types::RandomnessTask;

use super::types::{Event, Topic};

#[derive(Clone)]
pub struct NewRandomnessTask {
    pub chain_id: usize,
    pub randomness_task: RandomnessTask,
}

impl NewRandomnessTask {
    pub fn new(chain_id: usize, randomness_task: RandomnessTask) -> Self {
        NewRandomnessTask {
            chain_id,
            randomness_task,
        }
    }
}

impl Event for NewRandomnessTask {
    fn topic(&self) -> Topic {
        Topic::NewRandomnessTask(self.chain_id)
    }
}
