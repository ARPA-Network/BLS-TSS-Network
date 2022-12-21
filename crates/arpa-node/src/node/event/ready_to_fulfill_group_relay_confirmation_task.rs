use super::{types::Topic, Event};
use crate::node::subscriber::DebuggableEvent;
use arpa_node_dal::cache::GroupRelayConfirmationResultCache;

#[derive(Clone, Debug)]
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for ReadyToFulfillGroupRelayConfirmationTask {}
