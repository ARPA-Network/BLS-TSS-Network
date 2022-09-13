use crate::node::dal::cache::GroupRelayResultCache;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct ReadyToFulfillGroupRelayTask {
    pub tasks: Vec<GroupRelayResultCache>,
}

impl ReadyToFulfillGroupRelayTask {
    pub fn new(tasks: Vec<GroupRelayResultCache>) -> Self {
        ReadyToFulfillGroupRelayTask { tasks }
    }
}

impl Event for ReadyToFulfillGroupRelayTask {
    fn topic(&self) -> Topic {
        Topic::ReadyToFulfillGroupRelayTask
    }
}
