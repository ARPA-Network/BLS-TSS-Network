use crate::types::contract::ContractGroup;
use ethers_core::{
    types::{Address, U256},
    utils::hex,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, marker::PhantomData};
use threshold_bls::{group::PairingCurve, serialize::point_to_hex};

pub trait Task {
    fn request_id(&self) -> &[u8];
}

#[derive(Debug, Clone)]
pub struct BLSTask<T: Task> {
    pub task: T,
    pub state: bool,
}

impl Task for RandomnessTask {
    fn request_id(&self) -> &[u8] {
        &self.request_id
    }
}

#[derive(Clone, PartialEq)]
pub struct RandomnessTask {
    pub request_id: Vec<u8>,
    pub seed: U256,
    pub group_index: usize,
    pub request_confirmations: usize,
    pub assignment_block_height: usize,
}

impl std::fmt::Debug for RandomnessTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandomnessTask")
            .field("request_id", &hex::encode(&self.request_id))
            .field("seed", &self.seed)
            .field("group_index", &self.group_index)
            .field("assignment_block_height", &self.assignment_block_height)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DKGTask {
    pub group_index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub members: Vec<Address>,
    pub assignment_block_height: usize,
    pub coordinator_address: Address,
}

#[derive(Debug, Clone)]
pub struct GroupRelayTask {
    pub controller_global_epoch: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub assignment_block_height: usize,
}

#[derive(Debug, Clone)]
pub struct GroupRelayConfirmationTask {
    pub index: usize,
    pub group_relay_cache_index: usize,
    pub relayed_group_index: usize,
    pub relayed_group_epoch: usize,
    pub relayer_group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Group<C: PairingCurve> {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Option<C::G2>,
    pub members: BTreeMap<Address, Member<C>>,
    pub committers: Vec<Address>,
    pub c: PhantomData<C>,
}

impl<C: PairingCurve> std::fmt::Debug for Group<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("index", &self.index)
            .field("epoch", &self.epoch)
            .field("size", &self.size)
            .field("threshold", &self.threshold)
            .field("state", &self.state)
            .field("public_key", &(self.public_key.as_ref()).map(point_to_hex))
            .field("members", &self.members)
            .field("committers", &self.committers)
            .finish()
    }
}

impl<C: PairingCurve> Group<C> {
    pub fn new() -> Group<C> {
        Group {
            index: 0,
            epoch: 0,
            size: 0,
            threshold: 0,
            state: false,
            public_key: None,
            members: BTreeMap::new(),
            committers: vec![],
            c: PhantomData,
        }
    }

    pub fn remove_disqualified_nodes(&mut self, disqualified_nodes: &[Address]) {
        self.members
            .retain(|node, _| !disqualified_nodes.contains(node));
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Member<C: PairingCurve> {
    pub index: usize,
    pub id_address: Address,
    pub rpc_endpoint: Option<String>,
    pub partial_public_key: Option<C::G2>,
}

impl<C: PairingCurve> std::fmt::Debug for Member<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("index", &self.index)
            .field("id_address", &self.id_address)
            .field("rpc_endpoint", &self.rpc_endpoint)
            .field(
                "partial_public_key",
                &(self.partial_public_key.as_ref()).map(point_to_hex),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRelayConfirmation {
    pub group: ContractGroup,
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
    pub fn is_success(self) -> bool {
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
    pub fn to_i32(&self) -> i32 {
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

pub enum TaskType {
    Randomness,
    GroupRelay,
    GroupRelayConfirmation,
}

impl TaskType {
    pub fn to_i32(&self) -> i32 {
        match self {
            TaskType::Randomness => 0,
            TaskType::GroupRelay => 1,
            TaskType::GroupRelayConfirmation => 2,
        }
    }
}

impl From<i32> for TaskType {
    fn from(b: i32) -> Self {
        match b {
            1 => TaskType::GroupRelay,
            2 => TaskType::GroupRelayConfirmation,
            _ => TaskType::Randomness,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum DKGStatus {
    None,
    InPhase,
    CommitSuccess,
    WaitForPostProcess,
}

impl DKGStatus {
    pub fn to_usize(self) -> usize {
        match self {
            DKGStatus::None => 0,
            DKGStatus::InPhase => 1,
            DKGStatus::CommitSuccess => 2,
            DKGStatus::WaitForPostProcess => 3,
        }
    }
}

impl From<usize> for DKGStatus {
    fn from(s: usize) -> Self {
        match s {
            1 => DKGStatus::InPhase,
            2 => DKGStatus::CommitSuccess,
            3 => DKGStatus::WaitForPostProcess,
            _ => DKGStatus::None,
        }
    }
}
