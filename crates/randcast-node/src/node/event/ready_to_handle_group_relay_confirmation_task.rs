use crate::node::dal::types::GroupRelayConfirmationTask;

use super::types::{Event, Topic};

#[derive(Clone)]
pub struct ReadyToHandleGroupRelayConfirmationTask {
    pub chain_id: usize,
    pub tasks: Vec<GroupRelayConfirmationTask>,
}

impl ReadyToHandleGroupRelayConfirmationTask {
    pub fn new(chain_id: usize, tasks: Vec<GroupRelayConfirmationTask>) -> Self {
        ReadyToHandleGroupRelayConfirmationTask { chain_id, tasks }
    }
}

impl Event for ReadyToHandleGroupRelayConfirmationTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToHandleGroupRelayConfirmationTask(self.chain_id)
    }
}
