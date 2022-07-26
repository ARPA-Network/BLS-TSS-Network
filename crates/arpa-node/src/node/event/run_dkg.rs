use arpa_node_core::DKGTask;

use super::{types::Topic, Event};

#[derive(Clone)]
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
