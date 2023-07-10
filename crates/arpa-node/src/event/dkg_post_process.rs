use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;

#[derive(Clone, Debug)]
pub struct DKGPostProcess {
    pub group_index: usize,
    pub group_epoch: usize,
}

impl DKGPostProcess {
    pub fn new(group_index: usize, group_epoch: usize) -> Self {
        DKGPostProcess {
            group_index,
            group_epoch,
        }
    }
}

impl Event for DKGPostProcess {
    fn topic(&self) -> Topic {
        Topic::DKGPostProcess
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for DKGPostProcess {}
