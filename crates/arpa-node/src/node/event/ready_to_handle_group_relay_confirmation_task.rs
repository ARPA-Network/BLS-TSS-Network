use arpa_node_core::GroupRelayConfirmationTask;

use super::{types::Topic, Event};

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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
