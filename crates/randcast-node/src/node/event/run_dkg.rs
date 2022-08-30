use crate::node::dal::types::DKGTask;

use super::types::{Event, Topic};

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
}
