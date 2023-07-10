use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;

#[derive(Clone, Debug)]
pub struct DKGPhase {
    pub phase: usize,
}

impl DKGPhase {
    pub fn new(phase: usize) -> Self {
        DKGPhase { phase }
    }
}

impl Event for DKGPhase {
    fn topic(&self) -> Topic {
        Topic::DKGPhase
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for DKGPhase {}
