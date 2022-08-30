use super::types::{Event, Topic};
use crate::node::dal::types::DKGTask;

#[derive(Clone)]
pub struct NewDKGTask {
    pub dkg_task: DKGTask,
    pub self_index: usize,
}

impl NewDKGTask {
    pub fn new(dkg_task: DKGTask, self_index: usize) -> Self {
        NewDKGTask {
            dkg_task,
            self_index,
        }
    }
}

impl Event for NewDKGTask {
    fn topic(&self) -> Topic {
        Topic::NewDKGTask
    }
}
