use super::{types::Topic, Event};
use crate::node::subscriber::DebuggableEvent;
use arpa_node_core::GroupRelayTask;

#[derive(Clone, Debug)]
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
impl DebuggableEvent for ReadyToHandleGroupRelayTask {}
