use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

pub struct Node {
    pub id_address: String,
    pub id_public_key: Vec<u8>,
    pub state: bool,
    pub pending_until_block: usize,
    pub staking: usize,
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
    pub members: BTreeMap<String, Member>,
    pub committers: Vec<String>,
    pub commit_cache: BTreeMap<String, CommitCache>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id_address: String,
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

#[derive(Clone)]
pub struct SignatureTask {
    pub index: usize,
    pub message: String,
    pub group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct DKGTask {
    pub group_index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub members: BTreeMap<String, usize>,
    pub assignment_block_height: usize,
    pub coordinator_address: String,
}

#[derive(Clone)]
pub struct UnresponsiveGroupEvent {
    pub group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct GroupRelayTask {
    pub controller_global_epoch: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub assignment_block_height: usize,
}

#[derive(Clone)]
pub struct GroupRelayConfirmationTask {
    pub index: usize,
    pub group_relay_cache_index: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub relayer_group_index: usize,
    pub assignment_block_height: usize,
}

pub struct GroupRelayCache {
    pub relayer_committer: String,
    pub group: Group,
    pub group_relay_confirmation_task_index: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GroupRelayConfirmation {
    pub group: Group,
    pub status: Status,
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
    pub(crate) fn is_success(self) -> bool {
        match self {
            Status::Success => true,
            Status::Complaint => false,
        }
    }
}

pub enum GroupRelayConfirmationTaskState {
    NotExisted,
    Available,
    Invalid,
}

impl GroupRelayConfirmationTaskState {
    pub(crate) fn to_i32(&self) -> i32 {
        match self {
            GroupRelayConfirmationTaskState::NotExisted => 0,
            GroupRelayConfirmationTaskState::Available => 1,
            GroupRelayConfirmationTaskState::Invalid => 2,
        }
    }
}

impl From<i32> for GroupRelayConfirmationTaskState {
    fn from(b: i32) -> Self {
        match b {
            1 => GroupRelayConfirmationTaskState::Available,
            2 => GroupRelayConfirmationTaskState::Invalid,
            _ => GroupRelayConfirmationTaskState::NotExisted,
        }
    }
}
