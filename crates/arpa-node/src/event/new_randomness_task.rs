use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;
use arpa_core::RandomnessTask;

#[derive(Clone, Debug)]
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for NewRandomnessTask {}
