use super::adapter::{Adapter, AdapterViews};
use super::coordinator::{
    Coordinator, MockHelper as CoordinatorMockHelper, Transactions as CoordinatorTransactions,
    Views as CoordinatorViews,
};
use super::errors::{ControllerError, ControllerResult};
use super::types::{CommitCache, CommitResult, DKGTask, Group, Member, Node};
use super::utils::{choose_randomly_from_indices, minimum_threshold};
use ethers_core::types::{Address, U256};
use std::cmp::{max, Ordering};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use threshold_bls::curve::bn254::G2;

pub const NODE_STAKING_AMOUNT: usize = 50000;

pub const DISQUALIFIED_NODE_PENALTY_AMOUNT: usize = 1000;

pub const DKG_POST_PROCESS_REWARD: usize = 100;

pub const DEFAULT_MINIMUM_THRESHOLD: usize = 3;

pub const DEFAULT_NUMBER_OF_COMMITTERS: usize = 3;

pub const DEFAULT_DKG_PHASE_DURATION: usize = 10;

pub const GROUP_MAX_CAPACITY: usize = 10;

pub const IDEAL_NUMBER_OF_GROUPS: usize = 5;

pub const PENDING_BLOCK_AFTER_QUIT: usize = 100;

pub const COORDINATOR_ADDRESS_PREFIX: &str = "0x00000000000000000000000000000000000000c";

impl Deref for Controller {
    type Target = Adapter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Controller {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub struct Controller {
    base: Adapter,
    nodes: HashMap<Address, Node>,
    // adapters: HashMap<String, String>,
    // mock for locally test environment
    dkg_task: Option<DKGTask>,
    pub coordinators: HashMap<usize, (Address, Coordinator)>,
}

impl Controller {
    pub fn new(adapter: Adapter) -> Self {
        Controller {
            base: adapter,
            nodes: HashMap::new(),
            // adapters: HashMap::new(),
            dkg_task: None,
            coordinators: HashMap::new(),
        }
    }
}

trait ControllerInternal {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<Address>);

    fn node_join(&mut self, id_address: Address) -> ControllerResult<bool>;

    fn find_or_create_target_group(&mut self) -> (usize, bool);

    fn add_group(&mut self) -> usize;

    fn rebalance_group(
        &mut self,
        group_a_index: usize,
        group_b_index: usize,
    ) -> ControllerResult<bool>;

    fn arrange_members_in_group(&mut self, group_index: usize) -> ControllerResult<()>;

    fn rotate_group(
        &mut self,
        group_a_index: usize,
        group_b_index: usize,
    ) -> ControllerResult<bool>;

    fn add_to_group(
        &mut self,
        node_id_address: Address,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()>;

    fn remove_from_group(
        &mut self,
        node_id_address: &Address,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool>;

    fn emit_group_event(&mut self, group_index: usize) -> ControllerResult<()>;

    fn slash_node(
        &mut self,
        id_address: &Address,
        staking_penalty: U256,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;

    fn freeze_node(
        &mut self,
        id_address: &Address,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;
}

pub trait ControllerMockHelper {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask>;

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize>;
}

pub trait ControllerTransactions {
    // fn add_adapter(
    //     &mut self,
    //     adapter_chain_name: String,
    //     adapter_address: String,
    // ) -> ControllerResult<()>;

    fn node_register(
        &mut self,
        id_address: Address,
        id_public_key: Vec<u8>,
    ) -> ControllerResult<()>;

    fn node_activate(&mut self, id_address: &Address) -> ControllerResult<()>;

    fn node_quit(&mut self, id_address: &Address) -> ControllerResult<()>;

    fn commit_dkg(
        &mut self,
        id_address: &Address,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ControllerResult<()>;

    fn post_process_dkg(
        &mut self,
        id_address: &Address,
        group_index: usize,
        group_epoch: usize,
    ) -> ControllerResult<()>;
}

pub trait ControllerViews {
    fn get_node(&self, id_address: &Address) -> Option<&Node>;
}

impl ControllerInternal for Controller {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<Address>) {
        let group = self.groups.get(&group_index).unwrap();

        let mut map: HashMap<CommitResult, Vec<Address>> = HashMap::new();

        for (member_address, commit_cache) in group.commit_cache.iter() {
            let majority_members = map
                .entry(commit_cache.commit_result.clone())
                .or_insert(vec![]);

            majority_members.push(*member_address);
        }

        let (r, majority_members, is_strictly_majority) =
            map.into_iter().fold((None, vec![], false), |acc, r| {
                match r.1.len().cmp(&acc.1.len()) {
                    Ordering::Greater => (Some(r.0), r.1, true),
                    Ordering::Equal => (acc.0, acc.1, false),
                    _ => acc,
                }
            });

        if is_strictly_majority {
            return (r, majority_members);
        }

        (None, vec![])
    }

    fn node_join(&mut self, id_address: Address) -> ControllerResult<bool> {
        let (group_index, need_rebalance) = self.find_or_create_target_group();

        self.add_to_group(id_address, group_index, true)?;

        let group_indices = self
            .groups
            .keys()
            .copied()
            .filter(|i| *i != group_index)
            .collect::<Vec<_>>();

        if need_rebalance {
            group_indices.iter().try_for_each(|index| {
                if let Ok(true) = self.rebalance_group(*index, group_index) {
                    return None;
                }
                Some(())
            });
        }

        Ok(true)
    }

    fn emit_group_event(&mut self, group_index: usize) -> ControllerResult<()> {
        if !self.groups.contains_key(&group_index) {
            return Ok(());
        }

        self.epoch += 1;

        let group = self.groups.get_mut(&group_index).unwrap();

        //TODO refactor updating group state process to a utility method, like "prepare_for_next_epoch", so as to global variables updating
        group.epoch += 1;

        group.is_strictly_majority_consensus_reached = false;

        group.commit_cache = BTreeMap::new();

        group.committers = vec![];

        let group = self.groups.get(&group_index).unwrap();

        // create coordinator instance
        let mut coordinator =
            Coordinator::new(group.epoch, group.threshold, DEFAULT_DKG_PHASE_DURATION);

        let mut members = group
            .members
            .values()
            .map(|m| {
                let public_key = self.nodes.get(&m.id_address).unwrap().id_public_key.clone();
                (m.id_address, m.index, public_key)
            })
            .collect::<Vec<_>>();

        members.sort_by(|a, b| a.1.cmp(&b.1));

        coordinator.initialize(self.block_height, members)?;

        let group_index = group.index;

        // mock: destruct existed coordinator

        let coordinator_address = format!("{}{}", COORDINATOR_ADDRESS_PREFIX, group_index)
            .parse::<Address>()
            .unwrap();
        self.coordinators
            .insert(group_index, (coordinator_address, coordinator));

        // emit event
        let group = self.groups.get(&group_index).unwrap();

        let mut members = BTreeMap::new();

        for (member_id_address, member) in group.members.iter() {
            members.insert(*member_id_address, member.index);
        }

        let dkg_task = DKGTask {
            group_index: group.index,
            epoch: group.epoch,
            size: group.size,
            threshold: group.threshold,
            members,
            assignment_block_height: self.block_height,
            coordinator_address,
        };

        self.dkg_task = Some(dkg_task);
        // self.emit_dkg_task(dkg_task);

        Ok(())
    }

    fn find_or_create_target_group(&mut self) -> (usize, bool) {
        if self.groups.is_empty() {
            return (self.add_group(), false);
        }

        let (index_of_min_size, min_size) = self
            .groups
            .values()
            .map(|g| (g.index, g.size))
            .min_by(|x, y| x.1.cmp(&y.1))
            .unwrap();

        let valid_group_count = self.valid_group_indices().len();

        if (valid_group_count < IDEAL_NUMBER_OF_GROUPS && valid_group_count == self.groups.len())
            || min_size == GROUP_MAX_CAPACITY
        {
            return (self.add_group(), true);
        }

        (index_of_min_size, false)
    }

    fn add_group(&mut self) -> usize {
        let group_index = self.groups.len();

        let group = Group {
            index: group_index,
            epoch: 0,
            capacity: GROUP_MAX_CAPACITY,
            size: 0,
            threshold: DEFAULT_MINIMUM_THRESHOLD,
            is_strictly_majority_consensus_reached: false,
            public_key: vec![],
            fail_randomness_task_count: 0,
            members: BTreeMap::new(),
            committers: vec![],
            commit_cache: BTreeMap::new(),
        };

        self.groups.insert(group_index, group);

        group_index
    }

    fn rebalance_group(
        &mut self,
        mut group_a_index: usize,
        mut group_b_index: usize,
    ) -> ControllerResult<bool> {
        let mut group_a = self.groups.get(&group_a_index).unwrap();

        let mut group_b = self.groups.get(&group_b_index).unwrap();

        if group_b.size > group_a.size {
            std::mem::swap(&mut group_a, &mut group_b);

            std::mem::swap(&mut group_a_index, &mut group_b_index);
        }

        let expected_size_to_move = group_a.size - (group_a.size + group_b.size) / 2;

        if expected_size_to_move == 0
            || group_a.size - expected_size_to_move < DEFAULT_MINIMUM_THRESHOLD
        {
            return Ok(false);
        }

        let qualified_indices = group_a
            .members
            .values()
            .map(|member| member.index)
            .collect::<Vec<_>>();

        let members_to_move = choose_randomly_from_indices(
            self.last_output,
            &qualified_indices,
            expected_size_to_move,
        );

        let mut index_member_map: HashMap<usize, Address> = HashMap::new();

        group_a.members.iter().for_each(|(id_address, member)| {
            index_member_map.insert(member.index, *id_address);
        });

        for m in members_to_move.iter() {
            self.remove_from_group(index_member_map.get(m).unwrap(), group_a_index, false)?;

            self.add_to_group(*index_member_map.get(m).unwrap(), group_b_index, false)?;
        }

        self.emit_group_event(group_a_index)?;

        self.emit_group_event(group_b_index)?;

        Ok(true)
    }

    fn rotate_group(
        &mut self,
        mut group_a_index: usize,
        mut group_b_index: usize,
    ) -> ControllerResult<bool> {
        let mut group_a = self.groups.get(&group_a_index).unwrap();

        let mut group_b = self.groups.get(&group_b_index).unwrap();

        if group_b.size > group_a.size {
            std::mem::swap(&mut group_a, &mut group_b);

            std::mem::swap(&mut group_a_index, &mut group_b_index);
        }

        let expected_size_to_move = minimum_threshold(group_b.size);

        if expected_size_to_move == 0 {
            return Ok(false);
        }

        let group_a_member_indices = group_a
            .members
            .values()
            .map(|member| member.index)
            .collect::<Vec<_>>();

        let group_a_members_to_move = choose_randomly_from_indices(
            self.last_output,
            &group_a_member_indices,
            expected_size_to_move,
        );

        let group_b_member_indices = group_b
            .members
            .values()
            .map(|member| member.index)
            .collect::<Vec<_>>();

        let group_b_members_to_move = choose_randomly_from_indices(
            self.last_output,
            &group_b_member_indices,
            expected_size_to_move,
        );

        let mut member_index_id_map: HashMap<usize, Address> = HashMap::new();

        group_a
            .members
            .iter()
            .filter(|(_, member)| group_a_members_to_move.contains(&member.index))
            .for_each(|(id_address, member)| {
                member_index_id_map.insert(member.index, *id_address);
            });

        group_b
            .members
            .iter()
            .filter(|(_, member)| group_b_members_to_move.contains(&member.index))
            .for_each(|(id_address, member)| {
                member_index_id_map.insert(member.index, *id_address);
            });

        for m in group_a_members_to_move.iter() {
            self.remove_from_group(member_index_id_map.get(m).unwrap(), group_a_index, false)?;

            self.add_to_group(*member_index_id_map.get(m).unwrap(), group_b_index, false)?;
        }

        for m in group_b_members_to_move.iter() {
            self.remove_from_group(member_index_id_map.get(m).unwrap(), group_b_index, false)?;

            self.add_to_group(*member_index_id_map.get(m).unwrap(), group_a_index, false)?;
        }

        self.emit_group_event(group_a_index)?;

        self.emit_group_event(group_b_index)?;

        Ok(true)
    }

    fn add_to_group(
        &mut self,
        node_id_address: Address,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()> {
        let group = self.groups.get_mut(&group_index).unwrap();

        let member = Member {
            index: group.size,
            id_address: node_id_address,
            partial_public_key: vec![],
        };

        group.size += 1;

        group.members.insert(node_id_address, member);

        let minimum = minimum_threshold(group.size);

        group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

        if group.size >= 3 && emit_event_instantly {
            self.emit_group_event(group_index)?;
        }

        Ok(())
    }

    fn remove_from_group(
        &mut self,
        node_id_address: &Address,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool> {
        let group = self.groups.get_mut(&group_index).unwrap();

        group.size -= 1;

        if group.size == 0 {
            group.members.clear();
            group.threshold = 0;
            return Ok(false);
        }

        group.members.remove(node_id_address);

        let minimum = minimum_threshold(group.size);

        group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

        if group.size < 3 {
            return Ok(true);
        }

        if emit_event_instantly {
            self.emit_group_event(group_index)?;
        }

        Ok(false)
    }

    fn slash_node(
        &mut self,
        id_address: &Address,
        staking_penalty: U256,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()> {
        let node = self.nodes.get_mut(id_address).unwrap();

        node.staking -= staking_penalty;

        if node.staking < NODE_STAKING_AMOUNT.into() || pending_block > 0 {
            self.freeze_node(id_address, pending_block, handle_group)?;
        }

        Ok(())
    }

    fn freeze_node(
        &mut self,
        id_address: &Address,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()> {
        if handle_group {
            let belong_to_group = self
                .groups
                .values()
                .find(|g| g.members.contains_key(id_address));

            if let Some(group) = belong_to_group {
                let group_index = group.index;

                let need_rebalance = self.remove_from_group(id_address, group_index, true)?;

                if need_rebalance {
                    self.arrange_members_in_group(group_index)?;
                }
            }
        }

        let block_height = self.block_height;

        let node = self.nodes.get_mut(id_address).unwrap();

        node.state = false;

        node.pending_until_block = if node.pending_until_block > block_height {
            node.pending_until_block + pending_block
        } else {
            block_height + pending_block
        };

        Ok(())
    }

    fn arrange_members_in_group(&mut self, group_index: usize) -> ControllerResult<()> {
        let group_indices = self
            .groups
            .keys()
            .copied()
            .filter(|i| *i != group_index)
            .collect::<Vec<_>>();

        let rebalance_failure = group_indices.iter().try_for_each(|index| {
            if let Ok(true) = self.rebalance_group(*index, group_index) {
                return None;
            }
            Some(())
        });

        if rebalance_failure.is_some() {
            let group = self.groups.get_mut(&group_index).unwrap();
            group.is_strictly_majority_consensus_reached = false;

            let members_left_in_group = self
                .groups
                .get(&group_index)
                .unwrap()
                .members
                .keys()
                .cloned()
                .collect::<Vec<_>>();

            let mut invovled_groups = HashSet::new();

            for member_address in members_left_in_group.iter() {
                let (target_group_index, _) = self.find_or_create_target_group();

                if group_index == target_group_index {
                    break;
                }

                self.add_to_group(*member_address, target_group_index, false)?;

                invovled_groups.insert(target_group_index);
            }

            for index in invovled_groups.iter() {
                let group = self.groups.get(index).unwrap();

                if group.size >= 3 {
                    self.emit_group_event(*index)?;
                }
            }
        }

        Ok(())
    }
}

impl ControllerMockHelper for Controller {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask> {
        self.dkg_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize> {
        self.block_height += block_number;

        self.coordinators
            .values_mut()
            .for_each(|(_, c)| c.mine(block_number));

        println!("chain block_height: {}", self.block_height);

        Ok(self.block_height)
    }
}

impl ControllerTransactions for Controller {
    // fn add_adapter(
    //     &mut self,
    //     adapter_chain_name: String,
    //     adapter_address: String,
    // ) -> ControllerResult<()> {
    //     self.adapters.insert(adapter_chain_name, adapter_address);

    //     Ok(())
    // }

    fn node_register(
        &mut self,
        id_address: Address,
        id_public_key: Vec<u8>,
    ) -> ControllerResult<()> {
        if self.nodes.contains_key(&id_address) {
            return Err(ControllerError::NodeExisted);
        }
        // mock: initial staking

        let node = Node {
            id_address,
            id_public_key,
            state: true,
            pending_until_block: 0,
            staking: NODE_STAKING_AMOUNT.into(),
        };

        self.nodes.insert(id_address, node);

        self.rewards.insert(id_address, 0);

        self.node_join(id_address)?;

        Ok(())
    }

    fn node_activate(&mut self, id_address: &Address) -> ControllerResult<()> {
        if !self.nodes.contains_key(id_address) {
            return Err(ControllerError::NodeNotExisted);
        }

        let block_height = self.block_height;

        let node = self.nodes.get_mut(id_address).unwrap();

        if node.state {
            return Err(ControllerError::NodeActivated);
        }

        if node.pending_until_block > block_height {
            return Err(ControllerError::NodeNotAvailable(node.pending_until_block));
        }

        // mock: fill staking
        node.staking = NODE_STAKING_AMOUNT.into();

        node.state = true;

        self.node_join(*id_address)?;

        Ok(())
    }

    fn node_quit(&mut self, id_address: &Address) -> ControllerResult<()> {
        if !self.nodes.contains_key(id_address) {
            return Err(ControllerError::NodeNotExisted);
        }

        self.freeze_node(id_address, PENDING_BLOCK_AFTER_QUIT, true)?;
        // TODO decouple: take handle_group to here

        // mock token redeem

        Ok(())
    }

    fn commit_dkg(
        &mut self,
        id_address: &Address,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<Address>,
    ) -> ControllerResult<()> {
        if !self.groups.contains_key(&group_index) {
            return Err(ControllerError::GroupNotExisted);
        }

        bincode::deserialize::<G2>(&public_key)?;

        bincode::deserialize::<G2>(&partial_public_key)?;

        let (_, coordinator) = self
            .coordinators
            .get(&group_index)
            .ok_or(ControllerError::CoordinatorNotExisted(group_index))?;

        if coordinator.in_phase().is_err() {
            return Err(ControllerError::CoordinatorEnded);
        }

        let group = self.groups.get_mut(&group_index).unwrap();

        if !group.members.contains_key(id_address) {
            return Err(ControllerError::ParticipantNotExisted);
        }

        if group.epoch != group_epoch {
            return Err(ControllerError::GroupEpochObsolete(group.epoch));
        }

        if group.commit_cache.contains_key(id_address) {
            return Err(ControllerError::CommitCacheExisted);
        }

        let commit_result = CommitResult {
            group_epoch,
            public_key,
            disqualified_nodes,
        };

        let commit_cache = CommitCache {
            commit_result,
            partial_public_key: partial_public_key.clone(),
        };

        group.commit_cache.insert(*id_address, commit_cache);

        if group.is_strictly_majority_consensus_reached {
            // it's no good for a qualified node to miscommits here. So far we don't verify this commitment.
            let member = group.members.get_mut(id_address).unwrap();

            member.partial_public_key = partial_public_key;
        } else {
            match self.get_strictly_majority_identical_commitment_result(group_index) {
                (None, _) => {}

                (Some(identical_commit), mut majority_members) => {
                    let last_output = self.last_output;

                    let group = self.groups.get_mut(&group_index).unwrap();

                    // every majority_member should't be contained in disqualified_nodes
                    majority_members.retain(|m| !identical_commit.disqualified_nodes.contains(m));

                    if majority_members.len() >= group.threshold {
                        group.is_strictly_majority_consensus_reached = true;

                        group.size -= identical_commit.disqualified_nodes.len();

                        group.public_key = identical_commit.public_key.clone();

                        let disqualified_nodes = identical_commit.disqualified_nodes;

                        for (id_address, cache) in group.commit_cache.iter_mut() {
                            if !disqualified_nodes.contains(id_address) {
                                let member = group.members.get_mut(id_address).unwrap();

                                member.partial_public_key = cache.partial_public_key.clone();
                            }
                        }

                        // choose DEFAULT_NUMBER_OF_COMMITTERS committers randomly by last randomness output
                        let mut index_member_map: HashMap<usize, Address> = HashMap::new();

                        group.members.iter().for_each(|(id_address, member)| {
                            index_member_map.insert(member.index, *id_address);
                        });

                        let qualified_indices = group
                            .members
                            .values()
                            .filter(|member| majority_members.contains(&member.id_address))
                            .map(|member| member.index)
                            .collect::<Vec<_>>();

                        let committer_indices = choose_randomly_from_indices(
                            last_output,
                            &qualified_indices,
                            DEFAULT_NUMBER_OF_COMMITTERS,
                        );

                        committer_indices.iter().for_each(|c| {
                            group.committers.push(*index_member_map.get(c).unwrap());
                        });

                        // move out these disqualified_nodes from the group
                        group
                            .members
                            .retain(|node, _| !disqualified_nodes.contains(node));

                        for disqualified_node in disqualified_nodes {
                            self.slash_node(
                                &disqualified_node,
                                DISQUALIFIED_NODE_PENALTY_AMOUNT.into(),
                                0,
                                false,
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn post_process_dkg(
        &mut self,
        id_address: &Address,
        group_index: usize,
        group_epoch: usize,
    ) -> ControllerResult<()> {
        // handles coordinator selfdestruct if it reaches DKG timeout, then
        // 1. emit GroupRelayTask if grouping successfully
        // 2. arrange members if fail to group
        // and rewards trigger (sender)

        let group = self
            .groups
            .get(&group_index)
            .ok_or(ControllerError::GroupNotExisted)?;

        if !group.members.contains_key(id_address) {
            return Err(ControllerError::ParticipantNotExisted);
        }

        if group.epoch != group_epoch {
            return Err(ControllerError::GroupEpochObsolete(group.epoch));
        }

        let is_strictly_majority_consensus_reached = group.is_strictly_majority_consensus_reached;

        let (_, coordinator) = self
            .coordinators
            .get(&group_index)
            .ok_or(ControllerError::CoordinatorNotExisted(group_index))?;

        if coordinator.in_phase()? > 0 {
            return Err(ControllerError::CoordinatorNotEnded);
        }

        // mock coordinator selfdestruct

        self.coordinators.remove(&group_index);

        if !is_strictly_majority_consensus_reached {
            match self.get_strictly_majority_identical_commitment_result(group_index) {
                (None, _) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    group.size = 0;

                    group.threshold = 0;

                    let members = group.members.keys().cloned().collect::<Vec<_>>();

                    group.members.clear();

                    for m in members {
                        self.slash_node(&m, DISQUALIFIED_NODE_PENALTY_AMOUNT.into(), 0, false)?;
                    }
                }

                (Some(_), majority_members) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    let disqualified_nodes = group
                        .members
                        .keys()
                        .filter(|m| !majority_members.contains(m))
                        .copied()
                        .collect::<Vec<_>>();

                    group.size -= disqualified_nodes.len();

                    let minimum = minimum_threshold(group.size);

                    group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

                    group
                        .members
                        .retain(|node, _| !disqualified_nodes.contains(node));

                    for disqualified_node in disqualified_nodes.iter() {
                        self.slash_node(
                            disqualified_node,
                            DISQUALIFIED_NODE_PENALTY_AMOUNT.into(),
                            0,
                            false,
                        )?;
                    }

                    self.arrange_members_in_group(group_index)?;
                }
            }
        }

        if !self.rewards.contains_key(id_address) {
            self.rewards.insert(*id_address, 0);
        }

        let trigger_reward = self.rewards.get_mut(id_address).unwrap();

        *trigger_reward += DKG_POST_PROCESS_REWARD;

        Ok(())
    }
}

impl ControllerViews for Controller {
    fn get_node(&self, id_address: &Address) -> Option<&Node> {
        self.nodes.get(id_address)
    }
}

#[cfg(test)]
pub mod tests {

    use std::collections::HashMap;

    use ethers_core::types::{Address, U256};

    use crate::contract::adapter::AdapterTransactions;

    use super::{Adapter, Controller};

    #[test]
    fn test() {
        let initial_entropy: U256 = (0x8762_4875_6548_6346 as u64).into();

        let adapter = Adapter::new(initial_entropy);

        let mut controller = Controller::new(adapter);

        let node_address = "0x0000000000000000000000000000000000000001"
            .parse::<Address>()
            .unwrap();

        controller.rewards.insert(node_address, 1000);

        controller.claim(&node_address, node_address, 200).unwrap();

        println!("{:?}", controller.rewards.get(&node_address));
    }

    #[test]
    fn test2() {
        let vec1 = vec![String::from("232wer3")];
        let vec2 = vec![String::from("232wer3")];
        println!("{}", vec1 == vec2);
    }

    #[test]
    fn test3() {
        let str = String::from("ewrfw");
        let vec = bincode::serialize(&str).unwrap();
        let asda: String = bincode::deserialize(&vec).unwrap();
        println!("{}", asda);
        println!("{}", str == asda);
    }

    #[test]
    fn test4() {
        let str = String::from("ewrfw");
        let mut map: HashMap<usize, String> = HashMap::new();

        map.insert(1, str.clone());
        map.insert(1, str);

        println!("{}", map.get(&1).unwrap());
    }

    #[test]
    fn test5() {
        let address = "COORDINATOR_ADDRESS_PREFIX1";
        let index = address["COORDINATOR_ADDRESS_PREFIX".len()..]
            .parse::<usize>()
            .unwrap();
        assert_eq!(1, index);
    }
}
