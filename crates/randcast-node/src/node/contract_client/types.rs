use crate::node::dao::types::{Group as NodeGroup, Member as NodeMember};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

pub const GROUP_MAX_CAPACITY: usize = 10;

pub const SIGNATURE_TASK_EXCLUSIVE_WINDOW: usize = 30;

impl From<NodeGroup> for Group {
    fn from(g: NodeGroup) -> Self {
        let public_key = if let Some(k) = g.public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        let members: BTreeMap<String, Member> = g
            .members
            .into_iter()
            .map(|(id_address, m)| (id_address, m.into()))
            .collect();

        Group {
            index: g.index,
            epoch: g.epoch,
            capacity: GROUP_MAX_CAPACITY,
            size: g.size,
            threshold: g.threshold,
            is_strictly_majority_consensus_reached: true,
            public_key,
            fail_randomness_task_count: 0,
            members,
            committers: g.committers,
            commit_cache: BTreeMap::new(),
        }
    }
}

impl From<NodeMember> for Member {
    fn from(m: NodeMember) -> Self {
        let partial_public_key = if let Some(k) = m.partial_public_key {
            bincode::serialize(&k).unwrap()
        } else {
            vec![]
        };

        Member {
            id_address: m.id_address,
            index: m.index,
            partial_public_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id_address: String,
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
