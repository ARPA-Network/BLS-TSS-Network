use super::adapter::{Adapter, AdapterViews};
use super::coordinator::{
    Coordinator, MockHelper as CoordinatorMockHelper, Transactions as CoordinatorTransactions,
    Views as CoordinatorViews,
};
use super::errors::{ControllerError, ControllerResult};
use super::types::{
    CommitCache, CommitResult, DKGTask, Group, GroupRelayTask, Member, Node, SignatureTask,
};
use super::utils::{choose_randomly_from_indices, minimum_threshold};
use std::cmp::{max, Ordering};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use threshold_bls::curve::bls12381::G1;

pub const NODE_STAKING_AMOUNT: usize = 50000;

pub const DISQUALIFIED_NODE_PENALTY_AMOUNT: usize = 1000;

pub const DKG_POST_PROCESS_REWARD: usize = 100;

pub const DEFAULT_MINIMUM_THRESHOLD: usize = 3;

pub const DEFAULT_NUMBER_OF_COMMITTERS: usize = 3;

pub const DEFAULT_DKG_PHASE_DURATION: usize = 10;

pub const GROUP_MAX_CAPACITY: usize = 10;

pub const IDEAL_NUMBER_OF_GROUPS: usize = 5;

pub const PENDING_BLOCK_AFTER_QUIT: usize = 100;

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
    nodes: HashMap<String, Node>,
    // adapters: HashMap<String, String>,
    // mock for locally test environment
    dkg_task: Option<DKGTask>,
    group_relay_task: Option<GroupRelayTask>,
    pub coordinators: HashMap<usize, (String, Coordinator)>,
}

impl Controller {
    pub fn new(adapter: Adapter) -> Self {
        Controller {
            base: adapter,
            nodes: HashMap::new(),
            // adapters: HashMap::new(),
            dkg_task: None,
            group_relay_task: None,
            coordinators: HashMap::new(),
        }
    }

    // pub fn fulfill_relay(
    //     &mut self,
    //     _id_address: &str,
    //     _relayer_group_index: usize,
    //     _task_index: usize,
    //     _signature: Vec<u8>,
    //     _group_as_bytes: Vec<u8>,
    // ) -> ControllerResult<()> {
    //     Err(ControllerError::AuthenticationFailed)
    // }
}

trait ControllerInternal {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<String>);

    fn node_join(&mut self, id_address: String) -> ControllerResult<bool>;

    fn find_or_create_target_group(&mut self) -> (usize, bool);

    fn add_group(&mut self) -> usize;

    fn rebalance_group(
        &mut self,
        group_a_index: usize,
        group_b_index: usize,
    ) -> ControllerResult<bool>;

    fn rotate_group(
        &mut self,
        group_a_index: usize,
        group_b_index: usize,
    ) -> ControllerResult<bool>;

    fn add_to_group(
        &mut self,
        node_id_address: String,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()>;

    fn remove_from_group(
        &mut self,
        node_id_address: &str,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool>;

    fn emit_group_event(&mut self, group_index: usize) -> ControllerResult<()>;

    fn slash_node(
        &mut self,
        id_address: &str,
        staking_penalty: usize,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;

    fn freeze_node(
        &mut self,
        id_address: &str,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()>;
}

pub trait ControllerMockHelper {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask>;

    fn emit_group_relay_task(&self) -> ControllerResult<GroupRelayTask>;

    fn mine(&mut self, block_number: usize) -> ControllerResult<usize>;
}

pub trait ControllerTransactions {
    // fn add_adapter(
    //     &mut self,
    //     adapter_chain_name: String,
    //     adapter_address: String,
    // ) -> ControllerResult<()>;

    fn node_register(&mut self, id_address: String, id_public_key: Vec<u8>)
        -> ControllerResult<()>;

    fn node_activate(&mut self, id_address: String) -> ControllerResult<()>;

    fn node_quit(&mut self, id_address: &str) -> ControllerResult<()>;

    fn commit_dkg(
        &mut self,
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> ControllerResult<()>;

    fn post_process_dkg(
        &mut self,
        id_address: &str,
        group_index: usize,
        group_epoch: usize,
    ) -> ControllerResult<()>;

    fn report_unresponsive_group(
        &mut self,
        id_address: &str,
        group_index: usize,
    ) -> ControllerResult<()>;
}

pub trait ControllerViews {
    fn get_node(&self, id_address: &str) -> &Node;

    // fn get_adapters(&self) -> Vec<String>;
}

impl ControllerInternal for Controller {
    fn get_strictly_majority_identical_commitment_result(
        &self,
        group_index: usize,
    ) -> (Option<CommitResult>, Vec<String>) {
        let group = self.groups.get(&group_index).unwrap();

        let mut map: HashMap<CommitResult, Vec<String>> = HashMap::new();

        for (member, commit_cache) in group.commit_cache.iter() {
            let majority_members = map
                .entry(commit_cache.commit_result.clone())
                .or_insert(vec![]);

            majority_members.push(member.to_string());
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

    fn node_join(&mut self, id_address: String) -> ControllerResult<bool> {
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
                (m.id_address.clone(), m.index, public_key)
            })
            .collect::<Vec<_>>();

        members.sort_by(|a, b| a.1.cmp(&b.1));

        coordinator.initialize(self.block_height, members)?;

        let group_index = group.index;

        // mock: destruct existed coordinator

        self.coordinators.insert(
            group_index,
            (format!("0xcoordinator{}", group_index), coordinator),
        );

        // emit event
        let group = self.groups.get(&group_index).unwrap();

        let mut members = BTreeMap::new();

        for (member_id_address, member) in group.members.iter() {
            members.insert(member_id_address.clone(), member.index);
        }

        let dkg_task = DKGTask {
            group_index: group.index,
            epoch: group.epoch,
            size: group.size,
            threshold: group.threshold,
            members,
            assignment_block_height: self.block_height,
            coordinator_address: self.deployed_address.clone(),
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

        if (valid_group_count < IDEAL_NUMBER_OF_GROUPS || min_size == GROUP_MAX_CAPACITY)
            && valid_group_count == self.groups.len()
        {
            return (self.add_group(), true);
        }

        (index_of_min_size, false)
    }

    fn add_group(&mut self) -> usize {
        let group_index = self.groups.len() + 1;

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
            self.last_output as usize,
            &qualified_indices,
            expected_size_to_move,
        );

        let mut index_member_map: HashMap<usize, String> = HashMap::new();

        group_a.members.iter().for_each(|(id_address, member)| {
            index_member_map.insert(member.index, id_address.clone());
        });

        for m in members_to_move.iter() {
            self.remove_from_group(index_member_map.get(m).unwrap(), group_a_index, false)?;

            self.add_to_group(
                index_member_map.get(m).unwrap().clone(),
                group_b_index,
                false,
            )?;
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
            self.last_output as usize,
            &group_a_member_indices,
            expected_size_to_move,
        );

        let group_b_member_indices = group_b
            .members
            .values()
            .map(|member| member.index)
            .collect::<Vec<_>>();

        let group_b_members_to_move = choose_randomly_from_indices(
            self.last_output as usize,
            &group_b_member_indices,
            expected_size_to_move,
        );

        let mut member_index_id_map: HashMap<usize, String> = HashMap::new();

        group_a
            .members
            .iter()
            .filter(|(_, member)| group_a_members_to_move.contains(&member.index))
            .for_each(|(id_address, member)| {
                member_index_id_map.insert(member.index, id_address.clone());
            });

        group_b
            .members
            .iter()
            .filter(|(_, member)| group_b_members_to_move.contains(&member.index))
            .for_each(|(id_address, member)| {
                member_index_id_map.insert(member.index, id_address.clone());
            });

        for m in group_a_members_to_move.iter() {
            self.remove_from_group(member_index_id_map.get(m).unwrap(), group_a_index, false)?;

            self.add_to_group(
                member_index_id_map.get(m).unwrap().clone(),
                group_b_index,
                false,
            )?;
        }

        for m in group_b_members_to_move.iter() {
            self.remove_from_group(member_index_id_map.get(m).unwrap(), group_b_index, false)?;

            self.add_to_group(
                member_index_id_map.get(m).unwrap().clone(),
                group_a_index,
                false,
            )?;
        }

        self.emit_group_event(group_a_index)?;

        self.emit_group_event(group_b_index)?;

        Ok(true)
    }

    fn add_to_group(
        &mut self,
        node_id_address: String,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<()> {
        let group = self.groups.get_mut(&group_index).unwrap();

        let member = Member {
            index: group.size,
            id_address: node_id_address.clone(),
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
        node_id_address: &str,
        group_index: usize,
        emit_event_instantly: bool,
    ) -> ControllerResult<bool> {
        let group = self.groups.get_mut(&group_index).unwrap();

        group.size -= 1;

        if group.size == 0 {
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
        id_address: &str,
        staking_penalty: usize,
        pending_block: usize,
        handle_group: bool,
    ) -> ControllerResult<()> {
        let node = self.nodes.get_mut(id_address).unwrap();

        node.staking -= staking_penalty;

        if node.staking < NODE_STAKING_AMOUNT || pending_block > 0 {
            self.freeze_node(id_address, pending_block, handle_group)?;
        }

        Ok(())
    }

    fn freeze_node(
        &mut self,
        id_address: &str,
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
                // TODO check if the group ready to dkg
                // if true, do dkg
                // else try rebalance

                if need_rebalance {
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
                        let members_left_in_group = self
                            .groups
                            .get(&group_index)
                            .unwrap()
                            .members
                            .keys()
                            .map(|m| m.to_string())
                            .collect::<Vec<_>>();

                        let mut invovled_groups = HashSet::new();

                        for member_address in members_left_in_group.iter() {
                            let (target_group_index, _) = self.find_or_create_target_group();

                            if group_index == target_group_index {
                                break;
                            }

                            self.add_to_group(
                                member_address.to_string(),
                                target_group_index,
                                false,
                            )?;

                            invovled_groups.insert(target_group_index);
                        }

                        for index in invovled_groups.iter() {
                            let group = self.groups.get(index).unwrap();

                            if group.size >= 3 {
                                self.emit_group_event(*index)?;
                            }
                        }
                    }
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
}

impl ControllerMockHelper for Controller {
    fn emit_dkg_task(&self) -> ControllerResult<DKGTask> {
        self.dkg_task
            .clone()
            .ok_or(ControllerError::NoTaskAvailable)
    }

    fn emit_group_relay_task(&self) -> ControllerResult<GroupRelayTask> {
        self.group_relay_task
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
        id_address: String,
        id_public_key: Vec<u8>,
    ) -> ControllerResult<()> {
        if self.nodes.contains_key(&id_address) {
            return Err(ControllerError::NodeExisted);
        }
        // mock: initial staking

        let node = Node {
            id_address: id_address.clone(),
            id_public_key,
            state: true,
            pending_until_block: 0,
            staking: NODE_STAKING_AMOUNT,
        };

        self.nodes.insert(id_address.clone(), node);

        self.rewards.insert(id_address.clone(), 0);

        self.node_join(id_address)?;

        Ok(())
    }

    fn node_activate(&mut self, id_address: String) -> ControllerResult<()> {
        if !self.nodes.contains_key(&id_address) {
            return Err(ControllerError::NodeNotExisted);
        }

        let block_height = self.block_height;

        let node = self.nodes.get_mut(&id_address).unwrap();

        if node.state {
            return Err(ControllerError::NodeActivated);
        }

        if node.pending_until_block > block_height {
            return Err(ControllerError::NodeNotAvailable(node.pending_until_block));
        }

        // mock: fill staking
        node.staking = NODE_STAKING_AMOUNT;

        node.state = true;

        self.node_join(id_address)?;

        Ok(())
    }

    fn node_quit(&mut self, id_address: &str) -> ControllerResult<()> {
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
        id_address: String,
        group_index: usize,
        group_epoch: usize,
        public_key: Vec<u8>,
        partial_public_key: Vec<u8>,
        disqualified_nodes: Vec<String>,
    ) -> ControllerResult<()> {
        if !self.groups.contains_key(&group_index) {
            return Err(ControllerError::GroupNotExisted);
        }

        bincode::deserialize::<G1>(&public_key)?;

        bincode::deserialize::<G1>(&partial_public_key)?;

        let (_, coordinator) = self
            .coordinators
            .get(&group_index)
            .ok_or(ControllerError::CoordinatorNotExisted(group_index))?;

        if coordinator.in_phase().is_err() {
            return Err(ControllerError::CoordinatorEnded);
        }

        let group = self.groups.get_mut(&group_index).unwrap();

        if !group.members.contains_key(&id_address) {
            return Err(ControllerError::ParticipantNotExisted);
        }

        if group.epoch != group_epoch {
            return Err(ControllerError::GroupEpochObsolete(group.epoch));
        }

        if group.commit_cache.contains_key(&id_address) {
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

        group.commit_cache.insert(id_address.clone(), commit_cache);

        if group.is_strictly_majority_consensus_reached {
            // it's no good for a qualified node to miscommits here. So far we don't verify this commitment.
            let member = group.members.get_mut(&id_address).unwrap();

            member.partial_public_key = partial_public_key;
        } else {
            match self.get_strictly_majority_identical_commitment_result(group_index) {
                (None, _) => {}

                (Some(identical_commit), mut majority_members) => {
                    let last_output = self.last_output as usize;

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
                        let mut index_member_map: HashMap<usize, String> = HashMap::new();

                        group.members.iter().for_each(|(id_address, member)| {
                            index_member_map.insert(member.index, id_address.clone());
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
                            group
                                .committers
                                .push(index_member_map.get(c).unwrap().clone());
                        });

                        // move out these disqualified_nodes from the group
                        group
                            .members
                            .retain(|node, _| !disqualified_nodes.contains(node));

                        for disqualified_node in disqualified_nodes {
                            self.slash_node(
                                &disqualified_node,
                                DISQUALIFIED_NODE_PENALTY_AMOUNT,
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
        id_address: &str,
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

        if coordinator.in_phase().is_ok() {
            return Err(ControllerError::CoordinatorNotEnded);
        }

        // mock coordinator selfdestruct

        self.coordinators.remove(&group_index);

        if is_strictly_majority_consensus_reached {
            if self.groups.len() > 1 {
                let group_relay_task = GroupRelayTask {
                    controller_global_epoch: self.epoch,
                    relayed_group_index: group_index,
                    relayed_group_epoch: group_epoch,
                    assignment_block_height: self.block_height,
                };

                self.group_relay_task = Some(group_relay_task);
            }
        } else {
            match self.get_strictly_majority_identical_commitment_result(group_index) {
                (None, _) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    group.size = 0;

                    group.threshold = 0;

                    let members = group
                        .members
                        .keys()
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>();

                    group.members.clear();

                    for m in members {
                        self.slash_node(&m, DISQUALIFIED_NODE_PENALTY_AMOUNT, 0, false)?;
                    }
                }

                (Some(_), majority_members) => {
                    let group = self.groups.get_mut(&group_index).unwrap();

                    let disqualified_nodes = group
                        .members
                        .keys()
                        .filter(|m| !majority_members.contains(m))
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>();

                    group.size -= disqualified_nodes.len();

                    let minimum = minimum_threshold(group.size);

                    group.threshold = max(DEFAULT_MINIMUM_THRESHOLD, minimum);

                    group
                        .members
                        .retain(|node, _| !disqualified_nodes.contains(node));

                    for (index, disqualified_node) in disqualified_nodes.iter().enumerate() {
                        // last time try to rebalance to arrange honest nodes
                        let handle_group = index == disqualified_node.len() - 1;

                        self.slash_node(
                            disqualified_node,
                            DISQUALIFIED_NODE_PENALTY_AMOUNT,
                            0,
                            handle_group,
                        )?;
                    }
                }
            }
        }

        if !self.rewards.contains_key(id_address) {
            self.rewards.insert(id_address.to_string(), 0);
        }

        let trigger_reward = self.rewards.get_mut(id_address).unwrap();

        *trigger_reward += DKG_POST_PROCESS_REWARD;

        Ok(())
    }

    fn report_unresponsive_group(
        &mut self,
        id_address: &str,
        group_index: usize,
    ) -> ControllerResult<()> {
        let group = self
            .groups
            .get(&group_index)
            .ok_or(ControllerError::GroupNotExisted)?;

        if !group.members.contains_key(id_address) {
            return Err(ControllerError::ParticipantNotExisted);
        }

        // TODO should be different type of signature tasks
        let signature_task = SignatureTask {
            index: self.signature_count,
            message: format!(
                "unresponsive{}{}{}{}",
                group_index, group.epoch, &self.block_height, &self.last_output
            ),
            group_index,
            assignment_block_height: self.block_height,
        };

        self.signature_count += 1;

        self.signature_task = Some(signature_task);

        // self.pending_signature_tasks
        //     .insert(signature_task.index, signature_task);

        // // find the other group with the most members
        // if let Some((_, group_with_most_members)) = self
        //     .groups
        //     .iter()
        //     .filter(|(i, _)| **i != late_group_index)
        //     .max_by(|g1, g2| g1.1.size.cmp(&g2.1.size))
        // {}

        // // if let Ok(true) = self.rebalance_group(group_with_most_members, group_index) {
        // //     return None;
        // // }

        // // if rebalance_failure.is_some() {}

        Ok(())
    }
}

impl ControllerViews for Controller {
    fn get_node(&self, id_address: &str) -> &Node {
        self.nodes.get(id_address).unwrap()
    }

    // fn get_adapters(&self) -> Vec<String> {
    //     self.adapters
    //         .iter()
    //         .map(|(name, address)| format!("{}: {}", name, address))
    //         .collect::<Vec<_>>()
    // }
}

#[cfg(test)]
pub mod tests {

    use std::collections::HashMap;

    use crate::contract::adapter::AdapterTransactions;

    use super::{Adapter, Controller};

    #[test]
    fn test() {
        let initial_entropy = 0x8762_4875_6548_6346;

        let adapter = Adapter::new(initial_entropy, "0xcontroller_address".to_string());

        let mut controller = Controller::new(adapter);

        let node_address = "0x1";

        controller.rewards.insert(node_address.to_string(), 1000);

        controller.claim(node_address, node_address, 200).unwrap();

        println!("{:?}", controller.rewards.get(node_address));
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
}
