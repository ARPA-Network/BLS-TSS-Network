use super::{
    api::{
        BLSTasksFetcher, BLSTasksUpdater, BlockInfoFetcher, BlockInfoUpdater, GroupInfoFetcher,
        GroupInfoUpdater, NodeInfoFetcher, ResultCache, SignatureResultCacheFetcher,
        SignatureResultCacheUpdater,
    },
    types::{
        BLSTask, DKGStatus, DKGTask, Group, GroupRelayConfirmation, GroupRelayConfirmationTask,
        GroupRelayTask, Member, RandomnessTask, Task,
    },
};
use crate::node::contract_client::types::{
    Group as ContractGroup, SIGNATURE_TASK_EXCLUSIVE_WINDOW,
};
use crate::node::error::errors::{NodeError, NodeResult};
use dkg_core::primitives::DKGOutput;
use log::info;
use std::collections::HashMap;
use threshold_bls::group::Element;
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    sig::Share,
};

#[derive(Default)]
pub struct InMemoryBlockInfoCache {
    block_height: usize,
}

impl InMemoryBlockInfoCache {
    pub fn new() -> Self {
        InMemoryBlockInfoCache { block_height: 0 }
    }
}

impl BlockInfoFetcher for InMemoryBlockInfoCache {
    fn get_block_height(&self) -> usize {
        self.block_height
    }
}

impl BlockInfoUpdater for InMemoryBlockInfoCache {
    fn set_block_height(&mut self, block_height: usize) {
        self.block_height = block_height;
    }
}

pub struct InMemoryNodeInfoCache {
    id_address: String,
    node_rpc_endpoint: String,
    dkg_private_key: Option<Scalar>,
    dkg_public_key: Option<G1>,
}

impl InMemoryNodeInfoCache {
    pub fn new(
        id_address: String,
        node_rpc_endpoint: String,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> Self {
        InMemoryNodeInfoCache {
            id_address,
            node_rpc_endpoint,
            dkg_private_key: Some(dkg_private_key),
            dkg_public_key: Some(dkg_public_key),
        }
    }
}

impl NodeInfoFetcher for InMemoryNodeInfoCache {
    fn get_id_address(&self) -> &str {
        &self.id_address
    }

    fn get_node_rpc_endpoint(&self) -> &str {
        &self.node_rpc_endpoint
    }

    fn get_dkg_private_key(&self) -> NodeResult<&Scalar> {
        self.dkg_private_key.as_ref().ok_or(NodeError::NoDKGKeyPair)
    }

    fn get_dkg_public_key(&self) -> NodeResult<&G1> {
        self.dkg_public_key.as_ref().ok_or(NodeError::NoDKGKeyPair)
    }
}

pub struct InMemoryGroupInfoCache {
    share: Option<Share<Scalar>>,

    group: Group,

    dkg_status: DKGStatus,

    self_index: usize,

    dkg_start_block_height: usize,
}

impl Default for InMemoryGroupInfoCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGroupInfoCache {
    pub fn new() -> Self {
        let group: Group = Group::new();

        InMemoryGroupInfoCache {
            share: None,
            group,
            dkg_status: DKGStatus::None,
            self_index: 0,
            dkg_start_block_height: 0,
        }
    }

    fn only_has_group_task(&self) -> NodeResult<()> {
        if self.group.index == 0 {
            return Err(NodeError::NoGroupTask);
        }

        Ok(())
    }
}

impl GroupInfoUpdater for InMemoryGroupInfoCache {
    fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: DKGStatus,
    ) -> NodeResult<bool> {
        self.only_has_group_task()?;

        if index == self.group.index && epoch == self.group.epoch {
            self.dkg_status = dkg_status;

            info!("dkg_status transfered to {:?}", dkg_status);

            return Ok(true);
        }

        Ok(false)
    }

    fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> NodeResult<()> {
        self.self_index = self_index;

        self.group.index = task.group_index;

        self.group.epoch = task.epoch;

        self.group.size = task.size;

        self.group.threshold = task.threshold;

        self.group.public_key = None;

        self.group.state = false;

        self.group.members.clear();

        self.group.committers.clear();

        self.dkg_start_block_height = task.assignment_block_height;

        task.members.iter().for_each(|(address, index)| {
            let member = Member {
                index: *index,
                id_address: address.to_string(),
                rpc_endpint: None,
                partial_public_key: None,
            };
            self.group.members.insert(address.to_string(), member);
        });

        Ok(())
    }

    fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> NodeResult<(G1, G1, Vec<String>)> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(NodeError::GroupIndexObsolete(self.group.index));
        }

        if self.group.epoch != epoch {
            return Err(NodeError::GroupEpochObsolete(self.group.epoch));
        }

        if self.group.state {
            return Err(NodeError::GroupAlreadyReady);
        }

        self.share = Some(output.share);

        // every member index is started from 0
        let qualified_node_indices = output
            .qual
            .nodes
            .iter()
            .map(|node| node.id() as usize)
            .collect::<Vec<_>>();

        self.group.size = qualified_node_indices.len();

        let disqualified_nodes = self
            .group
            .members
            .iter()
            .filter(|(_, member)| !qualified_node_indices.contains(&member.index))
            .map(|(id_address, _)| id_address.to_string())
            .collect::<Vec<_>>();

        self.group
            .members
            .retain(|node, _| !disqualified_nodes.contains(node));

        let public_key = *output.public.public_key();

        self.group.public_key = Some(public_key);

        let mut partial_public_key = G1::new();

        for (_, member) in self.group.members.iter_mut() {
            if let Some(node) = output
                .qual
                .nodes
                .iter()
                .find(|node| member.index == node.id() as usize)
            {
                if let Some(rpc_endpoint) = node.get_rpc_endpoint() {
                    member.rpc_endpint = Some(rpc_endpoint.to_string());
                }
            }

            member.partial_public_key = Some(output.public.eval(member.index as u32).value);

            if self.self_index == member.index {
                partial_public_key = member.partial_public_key.unwrap();
            }
        }

        Ok((public_key, partial_public_key, disqualified_nodes))
    }

    fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<String>,
    ) -> NodeResult<()> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(NodeError::GroupIndexObsolete(self.group.index));
        }

        if self.group.epoch != epoch {
            return Err(NodeError::GroupEpochObsolete(self.group.epoch));
        }

        if self.group.state {
            return Err(NodeError::GroupAlreadyReady);
        }

        self.group.committers = committer_indices;

        self.group.state = true;

        Ok(())
    }
}

impl GroupInfoFetcher for InMemoryGroupInfoCache {
    fn get_index(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.index)
    }

    fn get_epoch(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.epoch)
    }

    fn get_size(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.size)
    }

    fn get_threshold(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.threshold)
    }

    fn get_state(&self) -> NodeResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.state)
    }

    fn get_public_key(&self) -> NodeResult<&G1> {
        self.only_has_group_task()?;

        self.group
            .public_key
            .as_ref()
            .ok_or(NodeError::GroupNotExisted)
    }

    fn get_secret_share(&self) -> NodeResult<&Share<Scalar>> {
        self.only_has_group_task()?;

        self.share.as_ref().ok_or(NodeError::GroupNotReady)
    }

    fn get_member(&self, id_address: &str) -> NodeResult<&Member> {
        self.only_has_group_task()?;

        self.group
            .members
            .get(id_address)
            .ok_or(NodeError::GroupNotExisted)
    }

    fn get_committers(&self) -> NodeResult<Vec<&str>> {
        self.only_has_group_task()?;

        Ok(self
            .group
            .committers
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>())
    }

    fn get_dkg_start_block_height(&self) -> NodeResult<usize> {
        self.only_has_group_task()?;

        Ok(self.dkg_start_block_height)
    }

    fn get_dkg_status(&self) -> NodeResult<DKGStatus> {
        self.only_has_group_task()?;

        Ok(self.dkg_status)
    }

    fn is_committer(&self, id_address: &str) -> NodeResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.committers.contains(&id_address.to_string()))
    }
}

#[derive(Default)]
pub struct InMemoryBLSTasksQueue<T: Task> {
    bls_tasks: Vec<BLSTask<T>>,
}

impl<T: Task> InMemoryBLSTasksQueue<T> {
    pub fn new() -> Self {
        InMemoryBLSTasksQueue {
            bls_tasks: Vec::new(),
        }
    }
}

impl<T: Task> BLSTasksFetcher<T> for InMemoryBLSTasksQueue<T> {
    fn contains(&self, task_index: usize) -> bool {
        self.bls_tasks
            .iter()
            .any(|task| task.task.index() == task_index)
    }

    fn get(&self, task_index: usize) -> Option<&T> {
        self.bls_tasks.get(task_index).map(|task| &task.task)
    }

    fn is_handled(&self, task_index: usize) -> bool {
        *self
            .bls_tasks
            .get(task_index)
            .map(|task| &task.state)
            .or(Some(&false))
            .unwrap()
    }
}

impl BLSTasksUpdater<RandomnessTask> for InMemoryBLSTasksQueue<RandomnessTask> {
    fn add(&mut self, task: RandomnessTask) -> NodeResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> Vec<RandomnessTask> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|task| !task.state)
            .filter(|task| {
                task.task.group_index == current_group_index
                    || current_block_height
                        > task.task.assignment_block_height + SIGNATURE_TASK_EXCLUSIVE_WINDOW
            })
            .map(|task| {
                task.state = true;
                task.task.clone()
            })
            .collect::<Vec<_>>();

        available_tasks
    }
}

impl BLSTasksUpdater<GroupRelayTask> for InMemoryBLSTasksQueue<GroupRelayTask> {
    fn add(&mut self, task: GroupRelayTask) -> NodeResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    fn check_and_get_available_tasks(
        &mut self,
        _: usize,
        current_group_index: usize,
    ) -> Vec<GroupRelayTask> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|task| !task.state)
            .filter(|task| task.task.relayed_group_index != current_group_index)
            .map(|task| {
                task.state = true;
                task.task.clone()
            })
            .collect::<Vec<_>>();

        available_tasks
    }
}

impl BLSTasksUpdater<GroupRelayConfirmationTask>
    for InMemoryBLSTasksQueue<GroupRelayConfirmationTask>
{
    fn add(&mut self, task: GroupRelayConfirmationTask) -> NodeResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    fn check_and_get_available_tasks(
        &mut self,
        _: usize,
        current_group_index: usize,
    ) -> Vec<GroupRelayConfirmationTask> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|task| !task.state)
            .filter(|task| task.task.relayed_group_index == current_group_index)
            .map(|task| {
                task.state = true;
                task.task.clone()
            })
            .collect::<Vec<_>>();

        available_tasks
    }
}

#[derive(Default)]
pub struct InMemorySignatureResultCache<T: ResultCache> {
    signature_result_caches: HashMap<usize, BLSResultCache<T>>,
}

impl<T: ResultCache> InMemorySignatureResultCache<T> {
    pub fn new() -> Self {
        InMemorySignatureResultCache {
            signature_result_caches: HashMap::new(),
        }
    }
}

impl Task for RandomnessResultCache {
    fn index(&self) -> usize {
        self.randomness_task_index
    }
}
impl Task for GroupRelayResultCache {
    fn index(&self) -> usize {
        self.group_relay_task_index
    }
}
impl Task for GroupRelayConfirmationResultCache {
    fn index(&self) -> usize {
        self.group_relay_confirmation_task_index
    }
}

impl ResultCache for RandomnessResultCache {}
impl ResultCache for GroupRelayResultCache {}
impl ResultCache for GroupRelayConfirmationResultCache {}

pub struct BLSResultCache<T: ResultCache> {
    pub result_cache: T,
    pub state: bool,
}

#[derive(Clone)]
pub struct RandomnessResultCache {
    pub group_index: usize,
    pub randomness_task_index: usize,
    pub message: String,
    pub threshold: usize,
    pub partial_signatures: HashMap<String, Vec<u8>>,
}

#[derive(Clone)]
pub struct GroupRelayResultCache {
    pub group_index: usize,
    pub group_relay_task_index: usize,
    pub relayed_group: ContractGroup,
    pub threshold: usize,
    pub partial_signatures: HashMap<String, Vec<u8>>,
}

#[derive(Clone)]
pub struct GroupRelayConfirmationResultCache {
    pub group_index: usize,
    pub group_relay_confirmation_task_index: usize,
    pub group_relay_confirmation: GroupRelayConfirmation,
    pub threshold: usize,
    pub partial_signatures: HashMap<String, Vec<u8>>,
}

impl<T: ResultCache> SignatureResultCacheFetcher<T> for InMemorySignatureResultCache<T> {
    fn contains(&self, signature_index: usize) -> bool {
        self.signature_result_caches.contains_key(&signature_index)
    }

    fn get(&self, signature_index: usize) -> Option<&BLSResultCache<T>> {
        self.signature_result_caches.get(&signature_index)
    }
}

impl SignatureResultCacheUpdater<RandomnessResultCache, String>
    for InMemorySignatureResultCache<RandomnessResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: String,
        threshold: usize,
    ) -> NodeResult<bool> {
        let signature_result_cache = RandomnessResultCache {
            group_index,
            randomness_task_index: signature_index,
            message,
            threshold,
            partial_signatures: HashMap::new(),
        };

        self.signature_result_caches.insert(
            signature_index,
            BLSResultCache {
                result_cache: signature_result_cache,
                state: false,
            },
        );

        Ok(true)
    }

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: String,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(NodeError::CommitterCacheNotExisted)?;

        signature_result_cache
            .result_cache
            .partial_signatures
            .insert(member_address, partial_signature);

        Ok(true)
    }

    fn get_ready_to_commit_signatures(&mut self) -> Vec<RandomnessResultCache> {
        self.signature_result_caches
            .values_mut()
            .filter(|v| {
                !v.state && v.result_cache.partial_signatures.len() >= v.result_cache.threshold
            })
            .map(|v| {
                v.state = true;
                v.result_cache.clone()
            })
            .collect::<Vec<_>>()
    }
}

impl SignatureResultCacheUpdater<GroupRelayResultCache, ContractGroup>
    for InMemorySignatureResultCache<GroupRelayResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: ContractGroup,
        threshold: usize,
    ) -> NodeResult<bool> {
        let signature_result_cache = GroupRelayResultCache {
            group_index,
            group_relay_task_index: signature_index,
            relayed_group: message,
            threshold,
            partial_signatures: HashMap::new(),
        };

        self.signature_result_caches.insert(
            signature_index,
            BLSResultCache {
                result_cache: signature_result_cache,
                state: false,
            },
        );

        Ok(true)
    }

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: String,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(NodeError::CommitterCacheNotExisted)?;

        signature_result_cache
            .result_cache
            .partial_signatures
            .insert(member_address, partial_signature);

        Ok(true)
    }

    fn get_ready_to_commit_signatures(&mut self) -> Vec<GroupRelayResultCache> {
        self.signature_result_caches
            .values_mut()
            .filter(|v| {
                !v.state && v.result_cache.partial_signatures.len() >= v.result_cache.threshold
            })
            .map(|v| {
                v.state = true;
                v.result_cache.clone()
            })
            .collect::<Vec<_>>()
    }
}

impl SignatureResultCacheUpdater<GroupRelayConfirmationResultCache, GroupRelayConfirmation>
    for InMemorySignatureResultCache<GroupRelayConfirmationResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: GroupRelayConfirmation,
        threshold: usize,
    ) -> NodeResult<bool> {
        let signature_result_cache = GroupRelayConfirmationResultCache {
            group_index,
            group_relay_confirmation_task_index: signature_index,
            group_relay_confirmation: message,
            threshold,
            partial_signatures: HashMap::new(),
        };

        self.signature_result_caches.insert(
            signature_index,
            BLSResultCache {
                result_cache: signature_result_cache,
                state: false,
            },
        );

        Ok(true)
    }

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: String,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(NodeError::CommitterCacheNotExisted)?;

        signature_result_cache
            .result_cache
            .partial_signatures
            .insert(member_address, partial_signature);

        Ok(true)
    }

    fn get_ready_to_commit_signatures(&mut self) -> Vec<GroupRelayConfirmationResultCache> {
        self.signature_result_caches
            .values_mut()
            .filter(|v| {
                !v.state && v.result_cache.partial_signatures.len() >= v.result_cache.threshold
            })
            .map(|v| {
                v.state = true;
                v.result_cache.clone()
            })
            .collect::<Vec<_>>()
    }
}
