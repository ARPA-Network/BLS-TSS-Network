use crate::contract::utils::u256_to_vec;

use super::errors::{ControllerError, ControllerResult};
use super::types::{Group, SignatureTask};
use ethers_core::types::{Address, U256};
use ethers_core::utils::keccak256;
use std::collections::HashMap;
use threshold_bls::schemes::bn254::G2Scheme as SigScheme;
use threshold_bls::sig::SignatureScheme;

pub const REWARD_PER_SIGNATURE: usize = 50;

pub const COMMITTER_REWARD_PER_SIGNATURE: usize = 100;

pub const COMMITTER_PENALTY_PER_SIGNATURE: usize = 1000;

pub const CHALLENGE_REWARD_PER_SIGNATURE: usize = 300;

pub const SIGNATURE_TASK_EXCLUSIVE_WINDOW: usize = 10;

pub const MAX_FAIL_RANDOMNESS_TASK_COUNT: usize = 3;

pub struct Adapter {
    pub block_height: usize,
    pub epoch: usize,
    pub signature_count: usize,
    pub last_output: U256,
    pub last_assigned_group_index: usize,
    pub(crate) groups: HashMap<usize, Group>,
    pub rewards: HashMap<Address, usize>,
    pub(crate) pending_signature_tasks: HashMap<Vec<u8>, SignatureTask>,
    // mock for locally test environment
    pub(crate) signature_task: Option<SignatureTask>,
}

impl Adapter {
    pub fn new(initial_entropy: U256) -> Self {
        Adapter {
            block_height: 100,
            epoch: 1,
            signature_count: 0,
            last_output: initial_entropy,
            last_assigned_group_index: 0,
            groups: HashMap::new(),
            rewards: HashMap::new(),
            pending_signature_tasks: HashMap::new(),
            signature_task: None,
        }
    }
}

pub trait AdapterMockHelper {
    fn emit_signature_task(&self) -> ControllerResult<SignatureTask>;

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize>;
}

pub trait AdapterTransactions {
    fn claim(
        &mut self,
        id_address: &Address,
        reward_address: Address,
        token_requested: usize,
    ) -> ControllerResult<()>;

    fn request_randomness(&mut self, seed: U256) -> ControllerResult<()>;

    fn fulfill_randomness(
        &mut self,
        id_address: &Address,
        group_index: usize,
        task_request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> ControllerResult<()>;
}

pub trait AdapterViews {
    fn get_last_output(&self) -> U256;

    fn get_group(&self, index: usize) -> Option<&Group>;

    fn get_group_state(&self, index: usize) -> bool;

    fn is_task_pending(&self, request_id: &[u8]) -> bool;

    fn valid_group_indices(&self) -> Vec<usize>;

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask>;
}

trait AdapterInternal {
    fn reward_randomness(
        &mut self,
        committer_address: &Address,
        participant_members: Vec<&Address>,
    ) -> ControllerResult<()>;
}

impl AdapterMockHelper for Adapter {
    fn emit_signature_task(&self) -> ControllerResult<SignatureTask> {
        self.signature_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize> {
        self.block_height += block_number;

        println!("adapter block_height: {}", self.block_height);

        Ok(self.block_height)
    }
}

impl AdapterTransactions for Adapter {
    fn claim(
        &mut self,
        id_address: &Address,
        _reward_address: Address,
        token_amount: usize,
    ) -> ControllerResult<()> {
        if !self.rewards.contains_key(id_address) {
            return Err(ControllerError::RewardRecordNotExisted);
        }

        let actual_amount = self.rewards.get_mut(id_address).unwrap();

        let operate_amount = if *actual_amount >= token_amount {
            token_amount
        } else {
            *actual_amount
        };

        // mock redeem to reward_address

        *actual_amount -= operate_amount;

        Ok(())
    }

    fn request_randomness(&mut self, seed: U256) -> ControllerResult<()> {
        let valid_group_indices = self.valid_group_indices();

        println!("request randomness successfully");

        if valid_group_indices.is_empty() {
            println!("no available group!");
            return Err(ControllerError::NoVaildGroup);
        }
        // mock: payment for request

        let mut assigned_group_index = self.last_assigned_group_index;

        loop {
            assigned_group_index = (assigned_group_index + 1) % (self.groups.len());

            if valid_group_indices.contains(&assigned_group_index) {
                break;
            }
        }

        self.last_assigned_group_index = assigned_group_index;

        // This is different from contract implementation
        let user_seed = u256_to_vec(&seed);
        let last_output = u256_to_vec(&self.last_output);

        let raw_seed =
            U256::from_big_endian(&keccak256([&user_seed[..], &last_output[..]].concat()));

        let request_id = keccak256(u256_to_vec(&raw_seed)).to_vec();

        let signature_task = SignatureTask {
            request_id: request_id.clone(),
            seed: raw_seed,
            group_index: assigned_group_index,
            assignment_block_height: self.block_height,
        };

        self.signature_count += 1;

        self.signature_task = Some(signature_task.clone());
        // self.emit_signature_task(signature_task.clone());

        self.pending_signature_tasks
            .insert(request_id, signature_task);

        Ok(())
    }

    fn fulfill_randomness(
        &mut self,
        id_address: &Address,
        group_index: usize,
        task_request_id: Vec<u8>,
        signature: Vec<u8>,
        partial_signatures: HashMap<Address, Vec<u8>>,
    ) -> ControllerResult<()> {
        if !self.pending_signature_tasks.contains_key(&task_request_id) {
            return Err(ControllerError::TaskNotFound);
        }

        let signature_task = self
            .pending_signature_tasks
            .get(&task_request_id)
            .unwrap()
            .clone();

        if (self.block_height
            <= signature_task.assignment_block_height + SIGNATURE_TASK_EXCLUSIVE_WINDOW)
            && group_index != signature_task.group_index
        {
            return Err(ControllerError::TaskStillExclusive);
        }

        let group = self
            .groups
            .get(&group_index)
            .ok_or(ControllerError::GroupNotExisted)?
            .clone();

        if !group.committers.contains(id_address) {
            return Err(ControllerError::NotFromCommitter);
        }

        let seed_bytes = u256_to_vec(&signature_task.seed);
        let block_num_bytes = u256_to_vec(&U256::from(signature_task.assignment_block_height));

        let message = [&seed_bytes[..], &block_num_bytes[..]].concat();

        let group_public_key = bincode::deserialize(&group.public_key)?;

        // verify tss-aggregation signature for randomness
        SigScheme::verify(&group_public_key, &message, &signature)?;

        println!("verify randomness signature successfully");

        // verify bls-aggregation signature for incentivizing worker list
        let sigs = partial_signatures
            .values()
            .map(|sig| sig as &[u8])
            .collect::<Vec<_>>();

        let mut public_keys = Vec::new();

        for member_id_address in partial_signatures.keys() {
            if !group.members.contains_key(member_id_address) {
                return Err(ControllerError::MemberNotExisted(
                    member_id_address.to_string(),
                    group_index,
                ));
            }

            let partial_public_key_as_bytes = &group
                .members
                .get(member_id_address)
                .unwrap()
                .partial_public_key;

            let partial_public_key = bincode::deserialize(partial_public_key_as_bytes)?;

            public_keys.push(partial_public_key);
        }

        SigScheme::aggregation_verify_on_the_same_msg(&public_keys, &message, &sigs)?;

        println!("verify partial signatures successfully");

        if group_index != signature_task.group_index {
            let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

            late_group.fail_randomness_task_count += 1;

            let late_group = self.groups.get(&signature_task.group_index).unwrap();

            if late_group.fail_randomness_task_count >= MAX_FAIL_RANDOMNESS_TASK_COUNT
                && self.groups.len() > 1
            {
                //TODO regrouping

                let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

                late_group.fail_randomness_task_count = 0;
            }
        }

        // MOCK: call user fulfill_randomness callback

        self.reward_randomness(id_address, partial_signatures.keys().collect::<Vec<_>>())?;

        self.last_output = U256::from(&keccak256(&signature));
        self.pending_signature_tasks.remove(&task_request_id);

        Ok(())
    }
}

impl AdapterViews for Adapter {
    fn get_group_state(&self, index: usize) -> bool {
        if !self.groups.contains_key(&index) {
            return false;
        }
        self.groups
            .get(&index)
            .unwrap()
            .is_strictly_majority_consensus_reached
    }

    fn get_last_output(&self) -> U256 {
        self.last_output
    }

    fn get_group(&self, index: usize) -> Option<&Group> {
        self.groups.get(&index)
    }

    fn is_task_pending(&self, request_id: &[u8]) -> bool {
        self.pending_signature_tasks.contains_key(request_id)
    }

    fn valid_group_indices(&self) -> Vec<usize> {
        self.groups
            .values()
            .filter(|g| g.is_strictly_majority_consensus_reached)
            .map(|g| g.index)
            .collect::<Vec<_>>()
    }

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask> {
        self.pending_signature_tasks.values().collect::<Vec<_>>()
    }
}

impl AdapterInternal for Adapter {
    fn reward_randomness(
        &mut self,
        committer_address: &Address,
        participant_members: Vec<&Address>,
    ) -> ControllerResult<()> {
        if !self.rewards.contains_key(committer_address) {
            self.rewards.insert(*committer_address, 0);
        }

        let committer_reward = self
            .rewards
            .get_mut(committer_address)
            .ok_or(ControllerError::RewardRecordNotExisted)?;

        *committer_reward += COMMITTER_REWARD_PER_SIGNATURE;

        for member_id_address in participant_members {
            if !self.rewards.contains_key(member_id_address) {
                self.rewards.insert(*member_id_address, 0);
            }

            let member_reward = self
                .rewards
                .get_mut(member_id_address)
                .ok_or(ControllerError::RewardRecordNotExisted)?;

            *member_reward += REWARD_PER_SIGNATURE;
        }

        Ok(())
    }
}
