use ethers_core::{
    types::{Address, U256},
    utils::hex,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, marker::PhantomData};
use threshold_bls::{group::Curve, serialize::point_to_hex};

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
    pub subscription_id: u64,
    pub group_index: u32,
    pub request_type: RandomnessRequestType,
    pub params: Vec<u8>,
    pub requester: Address,
    pub seed: U256,
    pub request_confirmations: u16,
    pub callback_gas_limit: u32,
    pub callback_max_gas_price: U256,
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
pub struct Group<C: Curve> {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Option<C::Point>,
    pub members: BTreeMap<Address, Member<C>>,
    pub committers: Vec<Address>,
    pub c: PhantomData<C>,
}

impl<C: Curve> std::fmt::Debug for Group<C> {
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

impl<C: Curve> Group<C> {
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
pub struct Member<C: Curve> {
    pub index: usize,
    pub id_address: Address,
    pub rpc_endpoint: Option<String>,
    pub partial_public_key: Option<C::Point>,
}

impl<C: Curve> std::fmt::Debug for Member<C> {
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

#[derive(Clone, Debug)]
pub struct PartialSignature {
    pub index: usize,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug)]
pub enum BLSTaskType {
    Randomness,
    GroupRelay,
    GroupRelayConfirmation,
}

impl BLSTaskType {
    pub fn to_i32(&self) -> i32 {
        match self {
            BLSTaskType::Randomness => 0,
            BLSTaskType::GroupRelay => 1,
            BLSTaskType::GroupRelayConfirmation => 2,
        }
    }
}

impl From<i32> for BLSTaskType {
    fn from(b: i32) -> Self {
        match b {
            1 => BLSTaskType::GroupRelay,
            2 => BLSTaskType::GroupRelayConfirmation,
            _ => BLSTaskType::Randomness,
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

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum RandomnessRequestType {
    Randomness,
    RandomWords,
    Shuffling,
}

impl RandomnessRequestType {
    pub fn to_u8(self) -> u8 {
        match self {
            RandomnessRequestType::Randomness => 0,
            RandomnessRequestType::RandomWords => 1,
            RandomnessRequestType::Shuffling => 2,
        }
    }
}

impl From<u8> for RandomnessRequestType {
    fn from(s: u8) -> Self {
        match s {
            1 => RandomnessRequestType::RandomWords,
            2 => RandomnessRequestType::Shuffling,
            _ => RandomnessRequestType::Randomness,
        }
    }
}
