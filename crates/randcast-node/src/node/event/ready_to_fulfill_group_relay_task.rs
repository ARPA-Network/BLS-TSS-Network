use crate::node::dao::cache::GroupRelayResultCache;

use super::types::{Event, Topic};

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
