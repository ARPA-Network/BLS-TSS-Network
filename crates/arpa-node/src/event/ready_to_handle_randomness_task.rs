use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;
use arpa_core::RandomnessTask;

#[derive(Clone, Debug)]
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for ReadyToHandleRandomnessTask {}
