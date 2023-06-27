use arpa_core::DKGTask;

use crate::subscriber::DebuggableEvent;

use super::{types::Topic, Event};

#[derive(Clone, Debug)]
pub struct RunDKG {
    pub dkg_task: DKGTask,
}

impl RunDKG {
    pub fn new(dkg_task: DKGTask) -> Self {
        RunDKG { dkg_task }
    }
}

impl Event for RunDKG {
    fn topic(&self) -> Topic {
        Topic::RunDKG
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DebuggableEvent for RunDKG {}
