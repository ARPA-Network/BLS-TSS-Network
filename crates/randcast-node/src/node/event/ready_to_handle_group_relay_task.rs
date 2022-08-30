use crate::node::dal::types::GroupRelayTask;

use super::types::{Event, Topic};

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
}
