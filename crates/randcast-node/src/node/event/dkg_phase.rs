use super::types::{Event, Topic};

#[derive(Clone)]
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
}
