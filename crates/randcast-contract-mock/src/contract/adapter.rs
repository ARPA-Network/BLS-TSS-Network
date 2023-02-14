use super::errors::{ControllerError, ControllerResult};
use super::types::{
    Group, GroupRelayCache, GroupRelayConfirmation, GroupRelayConfirmationTask,
    GroupRelayConfirmationTaskState, SignatureTask, UnresponsiveGroupEvent,
};
use super::utils::calculate_hash;
use std::collections::HashMap;
use threshold_bls::schemes::bn254::G2Scheme as SigScheme;
use threshold_bls::sig::SignatureScheme;

pub const REWARD_PER_SIGNATURE: usize = 50;

pub const COMMITTER_REWARD_PER_SIGNATURE: usize = 100;

pub const COMMITTER_PENALTY_PER_SIGNATURE: usize = 1000;

pub const CHALLENGE_REWARD_PER_SIGNATURE: usize = 300;

pub const SIGNATURE_TASK_EXCLUSIVE_WINDOW: usize = 30;

pub const RELAY_CONFIRMATION_VALIDATION_WINDOW: usize = 30;

pub const MAX_FAIL_RANDOMNESS_TASK_COUNT: usize = 3;

pub struct Adapter {
    pub block_height: usize,
    pub epoch: usize,
    pub signature_count: usize,
    pub last_output: u64,
    pub last_assigned_group_index: usize,
    pub(crate) groups: HashMap<usize, Group>,
    relayed_group_cache: HashMap<usize, GroupRelayCache>,
    relayed_group_confirmation_tasks: HashMap<usize, GroupRelayConfirmationTask>,
    pub relayed_group_confirmation_count: usize,
    pub rewards: HashMap<String, usize>,
    pub(crate) pending_signature_tasks: HashMap<usize, SignatureTask>,
    // mock for locally test environment
    pub(crate) signature_task: Option<SignatureTask>,
    group_relay_confirmation_task: Option<GroupRelayConfirmationTask>,
    unresponsive_group_task: Option<UnresponsiveGroupEvent>,
    pub(crate) _deployed_address: String,
}

impl Adapter {
    pub fn new(initial_entropy: u64, deployed_address: String) -> Self {
        Adapter {
            block_height: 100,
            epoch: 1,
            signature_count: 0,
            last_output: initial_entropy,
            last_assigned_group_index: 0,
            groups: HashMap::new(),
            relayed_group_cache: HashMap::new(),
            relayed_group_confirmation_tasks: HashMap::new(),
            relayed_group_confirmation_count: 0,
            rewards: HashMap::new(),
            pending_signature_tasks: HashMap::new(),
            signature_task: None,
            group_relay_confirmation_task: None,
            unresponsive_group_task: None,
            _deployed_address: deployed_address,
        }
    }
}

pub trait AdapterMockHelper {
    fn emit_signature_task(&self) -> ControllerResult<SignatureTask>;

    fn emit_group_relay_confirmation_task(&self) -> ControllerResult<GroupRelayConfirmationTask>;

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize>;
}

pub trait AdapterTransactions {
    fn claim(
        &mut self,
        id_address: &str,
        reward_address: &str,
        token_requested: usize,
    ) -> ControllerResult<()>;

    fn request_randomness(&mut self, message: &str) -> ControllerResult<()>;

    fn fulfill_randomness(
        &mut self,
        id_address: &str,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> ControllerResult<()>;

    fn fulfill_relay(
        &mut self,
        id_address: &str,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> ControllerResult<()>;

    fn cancel_invalid_relay_confirmation_task(
        &mut self,
        id_address: &str,
        task_index: usize,
    ) -> ControllerResult<()>;

    fn confirm_relay(
        &mut self,
        id_address: &str,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> ControllerResult<()>;

    fn set_initial_group(&mut self, id_address: &str, group: Vec<u8>) -> ControllerResult<()>;
}

pub trait AdapterViews {
    fn get_last_output(&self) -> u64;

    fn get_group(&self, index: usize) -> Option<&Group>;

    fn get_group_state(&self, index: usize) -> bool;

    fn get_group_relay_cache(&self, index: usize) -> Option<&Group>;

    fn get_signature_task_completion_state(&self, index: usize) -> bool;

    fn get_group_relay_confirmation_task_state(&self, task_index: usize) -> i32;

    fn valid_group_indices(&self) -> Vec<usize>;

    fn pending_signature_tasks(&self) -> Vec<&SignatureTask>;
}

trait AdapterInternal {
    fn reward_randomness(
        &mut self,
        committer_address: String,
        participant_members: Vec<String>,
    ) -> ControllerResult<()>;
}

impl AdapterMockHelper for Adapter {
    fn emit_signature_task(&self) -> ControllerResult<SignatureTask> {
        self.signature_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn emit_group_relay_confirmation_task(&self) -> ControllerResult<GroupRelayConfirmationTask> {
        self.group_relay_confirmation_task
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
        id_address: &str,
        _reward_address: &str,
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

    fn request_randomness(&mut self, message: &str) -> ControllerResult<()> {
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

        let signature_task = SignatureTask {
            index: self.signature_count,
            message: format!("{}{}{}", message, &self.block_height, &self.last_output),
            group_index: assigned_group_index,
            assignment_block_height: self.block_height,
        };

        self.signature_count += 1;

        self.signature_task = Some(signature_task.clone());
        // self.emit_signature_task(signature_task.clone());

        self.pending_signature_tasks
            .insert(signature_task.index, signature_task);

        Ok(())
    }

    fn fulfill_randomness(
        &mut self,
        id_address: &str,
        group_index: usize,
        signature_index: usize,
        signature: Vec<u8>,
        partial_signatures: HashMap<String, Vec<u8>>,
    ) -> ControllerResult<()> {
        if !self.pending_signature_tasks.contains_key(&signature_index) {
            return Err(ControllerError::TaskNotFound);
        }

        let signature_task = self
            .pending_signature_tasks
            .get(&signature_index)
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

        if !group.committers.contains(&id_address.to_string()) {
            return Err(ControllerError::NotFromCommitter);
        }

        let message = &signature_task.message;

        let group_public_key = bincode::deserialize(&group.public_key)?;

        // verify tss-aggregation signature for randomness
        SigScheme::verify(&group_public_key, message.as_bytes(), &signature)?;

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

        SigScheme::aggregation_verify_on_the_same_msg(&public_keys, message.as_bytes(), &sigs)?;

        if group_index != signature_task.group_index {
            let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

            late_group.fail_randomness_task_count += 1;

            let late_group = self.groups.get(&signature_task.group_index).unwrap();

            if late_group.fail_randomness_task_count >= MAX_FAIL_RANDOMNESS_TASK_COUNT
                && self.groups.len() > 1
            {
                let late_group_index = late_group.index;

                self.unresponsive_group_task = Some(UnresponsiveGroupEvent {
                    group_index: late_group_index,
                    assignment_block_height: self.block_height,
                });

                let late_group = self.groups.get_mut(&signature_task.group_index).unwrap();

                late_group.fail_randomness_task_count = 0;
            }
        }

        // MOCK: call user fulfill_randomness callback

        self.reward_randomness(
            id_address.to_string(),
            partial_signatures.keys().cloned().collect::<Vec<_>>(),
        )?;

        self.last_output = calculate_hash(&signature);

        self.pending_signature_tasks.remove(&signature_index);

        Ok(())
    }

    fn fulfill_relay(
        &mut self,
        id_address: &str,
        relayer_group_index: usize,
        task_index: usize,
        signature: Vec<u8>,
        group_as_bytes: Vec<u8>,
    ) -> ControllerResult<()> {
        // TODO maybe we should value Availability more than Consistency
        // solution 1(current)
        // relay can be multiple in progress(cache), and can be fulfilled not in strict order

        // solution 2
        // // only allow relaying in order, thus only one in progress(cache), for there will be some valid relayer for sure
        // if self.epoch + 1 != task_index {
        //     return Err(ControllerError::RelayFulfillmentNotInOrder);
        // }

        if self.relayed_group_cache.contains_key(&task_index) {
            return Err(ControllerError::RelayFulfillmentRepeated);
        }

        if !self.groups.contains_key(&relayer_group_index) {
            return Err(ControllerError::GroupNotExisted);
        }

        let relayer_group = self.groups.get(&relayer_group_index).unwrap();

        if !relayer_group.committers.contains(&id_address.to_string()) {
            return Err(ControllerError::NotFromCommitter);
        }

        let relayer_group_public_key = bincode::deserialize(&relayer_group.public_key)?;

        let relayed_group: Group = bincode::deserialize(&group_as_bytes)?;

        let relayed_group_index = relayed_group.index;

        println!("relayed_group_index: {}", relayed_group_index);

        let relayed_group_epoch = relayed_group.epoch;

        SigScheme::verify(&relayer_group_public_key, &group_as_bytes, &signature)?;

        if self.groups.contains_key(&relayed_group_index) {
            let current_relayed_group = self.groups.get(&relayed_group_index).unwrap();

            let current_relayed_group_epoch = current_relayed_group.epoch;

            if relayed_group_epoch <= current_relayed_group_epoch {
                return Err(ControllerError::RelayGroupDataObsolete(
                    current_relayed_group_epoch,
                ));
            }

            let current_relayed_group = self.groups.get_mut(&relayed_group_index).unwrap();

            current_relayed_group.is_strictly_majority_consensus_reached = false;
        }

        let group_relay_confirmation_task_index = self.relayed_group_confirmation_count;

        let group_relay_confirmation_task = GroupRelayConfirmationTask {
            index: group_relay_confirmation_task_index,
            group_relay_cache_index: task_index,
            relayed_group_index,
            relayed_group_epoch,
            relayer_group_index,
            assignment_block_height: self.block_height,
        };

        self.group_relay_confirmation_task = Some(group_relay_confirmation_task.clone());

        self.relayed_group_confirmation_count += 1;

        self.relayed_group_confirmation_tasks.insert(
            group_relay_confirmation_task_index,
            group_relay_confirmation_task,
        );

        let group_relay_cache = GroupRelayCache {
            relayer_committer: id_address.to_string(),
            group: relayed_group,
            group_relay_confirmation_task_index,
        };

        self.relayed_group_cache
            .insert(task_index, group_relay_cache);

        Ok(())
    }

    fn cancel_invalid_relay_confirmation_task(
        &mut self,
        _id_address: &str,
        task_index: usize,
    ) -> ControllerResult<()> {
        if !self
            .relayed_group_confirmation_tasks
            .contains_key(&task_index)
        {
            return Err(ControllerError::RelayTaskNotFound);
        }

        let group_relay_confirmation_task = self
            .relayed_group_confirmation_tasks
            .get(&task_index)
            .unwrap();

        let relayed_group_cache_index = group_relay_confirmation_task.group_relay_cache_index;

        let relayed_group_epoch = group_relay_confirmation_task.relayed_group_epoch;

        let relayed_group_index = group_relay_confirmation_task.relayed_group_index;

        let current_relayed_group = self.groups.get(&relayed_group_index).unwrap();

        let current_relayed_group_epoch = current_relayed_group.epoch;

        if self.block_height - group_relay_confirmation_task.assignment_block_height
            <= RELAY_CONFIRMATION_VALIDATION_WINDOW
            && relayed_group_epoch > current_relayed_group_epoch
        {
            return Err(ControllerError::RelayConfirmationTaskStillAvailable);
        }

        let current_relayed_group = self.groups.get_mut(&relayed_group_index).unwrap();

        current_relayed_group.is_strictly_majority_consensus_reached = true;

        self.relayed_group_cache.remove(&relayed_group_cache_index);

        self.relayed_group_confirmation_tasks.remove(&task_index);

        Ok(())
    }

    fn confirm_relay(
        &mut self,
        _id_address: &str,
        task_index: usize,
        group_relay_confirmation_as_bytes: Vec<u8>,
        signature: Vec<u8>,
    ) -> ControllerResult<()> {
        if !self
            .relayed_group_confirmation_tasks
            .contains_key(&task_index)
        {
            return Err(ControllerError::RelayTaskNotFound);
        }

        let group_relay_confirmation_task = self
            .relayed_group_confirmation_tasks
            .get(&task_index)
            .unwrap();

        let relayed_group_cache_index = group_relay_confirmation_task.group_relay_cache_index;

        let GroupRelayCache {
            relayer_committer,
            group: cached_group,
            group_relay_confirmation_task_index: _,
        } = self
            .relayed_group_cache
            .get(&relayed_group_cache_index)
            .unwrap();

        let group_relay_confirmation: GroupRelayConfirmation =
            bincode::deserialize(&group_relay_confirmation_as_bytes)?;

        let relayed_group = group_relay_confirmation.group;

        if relayed_group != *cached_group {
            return Err(ControllerError::RelayGroupDataInconsistency);
        }

        let group_public_key = bincode::deserialize(&relayed_group.public_key)?;

        SigScheme::verify(
            &group_public_key,
            &group_relay_confirmation_as_bytes,
            &signature,
        )?;

        let relayer_committer = relayer_committer.to_string();

        if group_relay_confirmation.status.is_success() {
            if self.groups.contains_key(&relayed_group.index) {
                let relayed_group_epoch = relayed_group.epoch;

                let current_relayed_group = self.groups.get(&relayed_group.index).unwrap();

                let current_relayed_group_epoch = current_relayed_group.epoch;

                if relayed_group_epoch <= current_relayed_group_epoch {
                    self.relayed_group_cache.remove(&relayed_group_cache_index);

                    self.relayed_group_confirmation_tasks.remove(&task_index);

                    return Err(ControllerError::RelayGroupDataObsolete(
                        current_relayed_group_epoch,
                    ));
                }
            }

            let relayer_group = self
                .groups
                .get(&group_relay_confirmation_task.relayer_group_index)
                .unwrap();

            let relayer_group_members = relayer_group.members.keys().cloned().collect::<Vec<_>>();

            self.reward_randomness(relayer_committer, relayer_group_members)?;

            self.groups.insert(relayed_group.index, relayed_group);

            self.epoch += 1;
        } else {
            // TODO for now we don't punish relayer group
            if self.groups.contains_key(&relayed_group.index) {
                let current_relayed_group = self.groups.get_mut(&relayed_group.index).unwrap();

                current_relayed_group.is_strictly_majority_consensus_reached = true;
            }
        }

        self.relayed_group_cache.remove(&relayed_group_cache_index);

        self.relayed_group_confirmation_tasks.remove(&task_index);

        Ok(())
    }

    fn set_initial_group(&mut self, id_address: &str, group: Vec<u8>) -> ControllerResult<()> {
        if id_address != "0x00000000000000000000000000000000000000ad" {
            return Err(ControllerError::AuthenticationFailed);
        }

        if !self.groups.is_empty() {
            return Err(ControllerError::InitialGroupExisted);
        }

        let initial_group: Group = bincode::deserialize(&group)?;

        self.groups.insert(initial_group.index, initial_group);

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

    fn get_last_output(&self) -> u64 {
        self.last_output
    }

    fn get_group(&self, index: usize) -> Option<&Group> {
        self.groups.get(&index)
    }

    fn get_signature_task_completion_state(&self, index: usize) -> bool {
        index < self.signature_count && !self.pending_signature_tasks.contains_key(&index)
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

    fn get_group_relay_cache(&self, index: usize) -> Option<&Group> {
        self.relayed_group_cache
            .get(&index)
            .map(|cache| &cache.group)
    }

    fn get_group_relay_confirmation_task_state(&self, task_index: usize) -> i32 {
        if task_index >= self.relayed_group_confirmation_count
            || !self
                .relayed_group_confirmation_tasks
                .contains_key(&task_index)
        {
            GroupRelayConfirmationTaskState::NotExisted.to_i32()
        } else {
            let group_relay_confirmation_task = self
                .relayed_group_confirmation_tasks
                .get(&task_index)
                .unwrap();

            let relayed_group_epoch = group_relay_confirmation_task.relayed_group_epoch;

            let relayed_group_index = group_relay_confirmation_task.relayed_group_index;

            let current_relayed_group = self.groups.get(&relayed_group_index).unwrap();

            let current_relayed_group_epoch = current_relayed_group.epoch;

            if self.block_height - group_relay_confirmation_task.assignment_block_height
                <= RELAY_CONFIRMATION_VALIDATION_WINDOW
                && relayed_group_epoch > current_relayed_group_epoch
            {
                return GroupRelayConfirmationTaskState::Available.to_i32();
            }

            GroupRelayConfirmationTaskState::Invalid.to_i32()
        }
    }
}

impl AdapterInternal for Adapter {
    fn reward_randomness(
        &mut self,
        committer_address: String,
        participant_members: Vec<String>,
    ) -> ControllerResult<()> {
        if !self.rewards.contains_key(&committer_address) {
            self.rewards.insert(committer_address.to_string(), 0);
        }

        let committer_reward = self
            .rewards
            .get_mut(&committer_address)
            .ok_or(ControllerError::RewardRecordNotExisted)?;

        *committer_reward += COMMITTER_REWARD_PER_SIGNATURE;

        for member_id_address in participant_members {
            if !self.rewards.contains_key(&member_id_address) {
                self.rewards.insert(member_id_address.to_string(), 0);
            }

            let member_reward = self
                .rewards
                .get_mut(&member_id_address)
                .ok_or(ControllerError::RewardRecordNotExisted)?;

            *member_reward += REWARD_PER_SIGNATURE;
        }

        Ok(())
    }
}
