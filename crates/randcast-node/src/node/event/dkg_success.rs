use arpa_node_core::Group;

use super::{types::Topic, Event};

#[derive(Clone)]
pub struct DKGSuccess {
    pub group: Group,
}

impl DKGSuccess {
    pub fn new(group: Group) -> Self {
        DKGSuccess { group }
    }
}

impl Event for DKGSuccess {
    fn topic(&self) -> Topic {
        Topic::DKGSuccess
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
