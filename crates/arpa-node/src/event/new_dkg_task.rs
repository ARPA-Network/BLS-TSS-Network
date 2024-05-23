use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;
use arpa_core::DKGTask;

#[derive(Clone, Debug)]
pub struct NewDKGTask {
    pub chain_id: usize,
    pub dkg_task: DKGTask,
    pub self_index: usize,
}

impl NewDKGTask {
    pub fn new(chain_id: usize, dkg_task: DKGTask, self_index: usize) -> Self {
        NewDKGTask {
            chain_id,
            dkg_task,
            self_index,
        }
    }
}

impl Event for NewDKGTask {
    fn topic(&self) -> Topic {
        Topic::NewDKGTask
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for NewDKGTask {}
