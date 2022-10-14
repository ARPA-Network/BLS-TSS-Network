use arpa_node_core::GroupRelayTask;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct ReadyToHandleGroupRelayTask {
    pub tasks: Vec<GroupRelayTask>,
}

impl ReadyToHandleGroupRelayTask {
    pub fn new(tasks: Vec<GroupRelayTask>) -> Self {
        ReadyToHandleGroupRelayTask { tasks }
    }
}

impl Event for ReadyToHandleGroupRelayTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToHandleGroupRelayTask
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
