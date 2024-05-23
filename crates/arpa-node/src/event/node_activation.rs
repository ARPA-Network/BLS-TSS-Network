use ethers::types::Address;

use crate::subscriber::DebuggableEvent;

use super::{types::Topic, Event};

#[derive(Clone, Debug)]
pub struct NodeActivation {
    pub chain_id: usize,
    pub is_eigenlayer: bool,
    pub node_registry_address: Address,
}

impl NodeActivation {
    pub fn new(chain_id: usize, is_eigenlayer: bool, node_registry_address: Address) -> Self {
        NodeActivation {
            chain_id,
            is_eigenlayer,
            node_registry_address,
        }
    }
}

impl Event for NodeActivation {
    fn topic(&self) -> Topic {
        Topic::NodeActivation
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl DebuggableEvent for NodeActivation {}
