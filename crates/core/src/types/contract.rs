use crate::address_to_string;
use crate::types::node::{Group as NodeGroup, Member as NodeMember};
use ethers_core::types::Address;
use ethers_core::utils::hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use threshold_bls::group::Curve;

pub struct Node {
    pub id_address: Address,
    pub id_public_key: Vec<u8>,
    pub state: bool,
    pub pending_until_block: usize,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id_address", &address_to_string(self.id_address))
            .field("id_public_key", &hex::encode(&self.id_public_key))
            .field("state", &self.state)
            .field("pending_until_block", &self.pending_until_block)
            .finish()
    }
}

impl<C: Curve> From<NodeGroup<C>> for ContractGroup {
    fn from(g: NodeGroup<C>) -> Self {
        let public_key = if let Some(k) = g.public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        let members: BTreeMap<String, ContractMember> = g
            .members
            .into_iter()
            .map(|(id_address, m)| (address_to_string(id_address), m.into()))
            .collect();

        let committers = g.committers.into_iter().map(address_to_string).collect();

        ContractGroup {
            index: g.index,
            epoch: g.epoch,
            size: g.size,
            threshold: g.threshold,
            is_strictly_majority_consensus_reached: g.state,
            public_key,
            members,
            committers,
            commit_cache: BTreeMap::new(),
        }
    }
}

impl<C: Curve> From<NodeMember<C>> for ContractMember {
    fn from(m: NodeMember<C>) -> Self {
        let partial_public_key = if let Some(k) = m.partial_public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        ContractMember {
            id_address: m.id_address,
            index: m.index,
            partial_public_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractGroup {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub is_strictly_majority_consensus_reached: bool,
    pub public_key: Vec<u8>,
    pub members: BTreeMap<String, ContractMember>,
    pub committers: Vec<String>,
    pub commit_cache: BTreeMap<String, CommitCache>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractMember {
    pub id_address: Address,
    pub index: usize,
    pub partial_public_key: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitCache {
    pub(crate) commit_result: CommitResult,
    pub(crate) partial_public_key: Vec<u8>,
}

#[derive(Debug, Eq, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub(crate) group_epoch: usize,
    pub(crate) public_key: Vec<u8>,
    pub(crate) disqualified_nodes: Vec<String>,
}

impl PartialEq for CommitResult {
    fn eq(&self, other: &Self) -> bool {
        self.group_epoch == other.group_epoch
            && self.public_key == other.public_key
            && self.disqualified_nodes == other.disqualified_nodes
    }
}

impl Hash for CommitResult {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.group_epoch.hash(state);
        self.public_key.hash(state);
        self.disqualified_nodes.hash(state);
    }
}
