use super::{ChainIdentity, Task};
use crate::node::contract_client::types::Group as ContractGroup;
use ethers::{
    signers::{LocalWallet, Signer},
    types::Address,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use threshold_bls::curve::bls12381::G1;

#[derive(Debug)]
pub struct BLSTask<T: Task> {
    pub task: T,
    pub state: bool,
}

impl Task for RandomnessTask {
    fn index(&self) -> usize {
        self.index
    }
}

impl Task for GroupRelayTask {
    fn index(&self) -> usize {
        self.controller_global_epoch
    }
}

impl Task for GroupRelayConfirmationTask {
    fn index(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RandomnessTask {
    pub index: usize,
    pub message: String,
    pub group_index: usize,
    pub assignment_block_height: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DKGTask {
    pub group_index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub members: BTreeMap<Address, usize>,
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Group {
    pub index: usize,
    pub epoch: usize,
    pub size: usize,
    pub threshold: usize,
    pub state: bool,
    pub public_key: Option<G1>,
    pub members: BTreeMap<Address, Member>,
    pub committers: Vec<Address>,
}

impl Group {
    pub fn new() -> Group {
        Group {
            index: 0,
            epoch: 0,
            size: 0,
            threshold: 0,
            state: false,
            public_key: None,
            members: BTreeMap::new(),
            committers: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub index: usize,
    pub id_address: Address,
    pub rpc_endpint: Option<String>,
    pub partial_public_key: Option<G1>,
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
    pub(crate) fn to_i32(&self) -> i32 {
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

#[derive(Clone)]
pub struct MockChainIdentity {
    id: usize,
    chain_id: usize,
    id_address: Address,
    provider_rpc_endpoint: String,
}

impl MockChainIdentity {
    pub fn new(
        id: usize,
        chain_id: usize,
        id_address: Address,
        provider_rpc_endpoint: String,
    ) -> Self {
        MockChainIdentity {
            id,
            chain_id,
            id_address,
            provider_rpc_endpoint,
        }
    }
}

impl ChainIdentity for MockChainIdentity {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.id_address
    }

    fn get_provider_rpc_endpoint(&self) -> &str {
        &self.provider_rpc_endpoint
    }

    fn get_contract_address(&self) -> Address {
        Address::random()
    }
}

#[derive(Clone)]
pub struct GeneralChainIdentity {
    id: usize,
    chain_id: usize,
    pub(crate) wallet: LocalWallet,
    provider_rpc_endpoint: String,
    contract_address: Address,
}

impl GeneralChainIdentity {
    pub fn new(
        id: usize,
        chain_id: usize,
        wallet: LocalWallet,
        provider_rpc_endpoint: String,
        contract_address: Address,
    ) -> Self {
        GeneralChainIdentity {
            id,
            chain_id,
            wallet,
            provider_rpc_endpoint,
            contract_address,
        }
    }
}

impl ChainIdentity for GeneralChainIdentity {
    fn get_id(&self) -> usize {
        self.id
    }

    fn get_chain_id(&self) -> usize {
        self.chain_id
    }

    fn get_id_address(&self) -> Address {
        self.wallet.address()
    }

    fn get_provider_rpc_endpoint(&self) -> &str {
        &self.provider_rpc_endpoint
    }

    fn get_contract_address(&self) -> Address {
        self.contract_address
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum DKGStatus {
    None,
    InPhase,
    CommitSuccess,
    WaitForPostProcess,
}

impl DKGStatus {
    pub(crate) fn to_usize(self) -> usize {
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
