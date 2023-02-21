use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use ethers_core::types::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id_address: Address,
    pub id_public_key: Vec<u8>,
    pub state: bool,
    pub pending_until_block: usize,
    pub staking: U256,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub index: usize,
    pub epoch: usize,
    pub capacity: usize,
    pub size: usize,
    pub threshold: usize,
    pub is_strictly_majority_consensus_reached: bool,
    pub public_key: Vec<u8>,
    pub fail_randomness_task_count: usize,
    pub members: BTreeMap<Address, Member>,
    pub committers: Vec<Address>,
    pub commit_cache: BTreeMap<Address, CommitCache>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id_address: Address,
    pub index: usize,
    pub partial_public_key: Vec<u8>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitCache {
    pub(crate) commit_result: CommitResult,
    pub(crate) partial_public_key: Vec<u8>,
}

#[derive(Eq, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub(crate) group_epoch: usize,
    pub(crate) public_key: Vec<u8>,
    pub(crate) disqualified_nodes: Vec<Address>,
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

#[derive(Clone)]
pub struct SignatureTask {
    pub request_id: Vec<u8>,
    pub seed: U256,
    pub group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct DKGTask {
    pub group_index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub members: BTreeMap<Address, usize>,
    pub assignment_block_height: usize,
    pub coordinator_address: Address,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Status {
    Success,
    Complaint,
}

impl From<bool> for Status {
    fn from(b: bool) -> Self {
        if b {
            Status::Success
        } else {
            Status::Complaint
        }
    }
}

impl Status {
    pub(crate) fn _is_success(self) -> bool {
        match self {
            Status::Success => true,
            Status::Complaint => false,
        }
    }
}
