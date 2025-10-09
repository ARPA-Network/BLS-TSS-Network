use super::{types::Topic, Event};
use crate::subscriber::DebuggableEvent;

#[derive(Clone, Debug)]
pub struct ProviderReconnection {
    pub chain_id: u64,
}

impl ProviderReconnection {
    pub fn new(chain_id: u64) -> Self {
        ProviderReconnection { chain_id }
    }
}

impl Event for ProviderReconnection {
    fn topic(&self) -> Topic {
        Topic::ProviderReconnection
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for ProviderReconnection {}
