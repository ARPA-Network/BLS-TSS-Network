use crate::node::dal::types::GroupRelayTask;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct NewGroupRelayTask {
    pub group_relay_task: GroupRelayTask,
}

impl NewGroupRelayTask {
    pub fn new(group_relay_task: GroupRelayTask) -> Self {
        NewGroupRelayTask { group_relay_task }
    }
}

impl Event for NewGroupRelayTask {
    fn topic(&self) -> Topic {
        Topic::NewGroupRelayTask
    }
}
