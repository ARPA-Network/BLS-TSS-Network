use crate::node::dal::types::GroupRelayConfirmationTask;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct NewGroupRelayConfirmationTask {
    pub chain_id: usize,
    pub group_relay_confirmation_task: GroupRelayConfirmationTask,
}

impl NewGroupRelayConfirmationTask {
    pub fn new(chain_id: usize, group_relay_confirmation_task: GroupRelayConfirmationTask) -> Self {
        NewGroupRelayConfirmationTask {
            chain_id,
            group_relay_confirmation_task,
        }
    }
}

impl Event for NewGroupRelayConfirmationTask {
    fn topic(&self) -> Topic {
        Topic::NewGroupRelayConfirmationTask(self.chain_id)
    }
}
