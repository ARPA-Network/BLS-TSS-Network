use crate::node::dal::cache::GroupRelayConfirmationResultCache;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct ReadyToFulfillGroupRelayConfirmationTask {
    pub chain_id: usize,
    pub tasks: Vec<GroupRelayConfirmationResultCache>,
}

impl ReadyToFulfillGroupRelayConfirmationTask {
    pub fn new(chain_id: usize, tasks: Vec<GroupRelayConfirmationResultCache>) -> Self {
        ReadyToFulfillGroupRelayConfirmationTask { chain_id, tasks }
    }
}

impl Event for ReadyToFulfillGroupRelayConfirmationTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToFulfillGroupRelayConfirmationTask(self.chain_id)
    }
}
