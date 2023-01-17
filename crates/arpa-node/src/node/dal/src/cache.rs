use crate::error::{DataAccessResult, GroupError, NodeInfoError};
use crate::MdcContextUpdater;

use super::{
    BLSTasksFetcher, BLSTasksUpdater, BlockInfoFetcher, BlockInfoUpdater, GroupInfoFetcher,
    GroupInfoUpdater, NodeInfoFetcher, NodeInfoUpdater, ResultCache, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use arpa_node_core::{
    BLSTask, BLSTaskError, ContractGroup, DKGStatus, DKGTask, Group, GroupRelayConfirmation,
    GroupRelayConfirmationTask, GroupRelayTask, Member, RandomnessTask, Task,
    RANDOMNESS_TASK_EXCLUSIVE_WINDOW,
};
use async_trait::async_trait;
use dkg_core::primitives::DKGOutput;
use ethers_core::types::Address;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use threshold_bls::group::Element;
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    sig::Share,
};

#[derive(Debug, Default)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct InMemoryNodeInfoCache {
    pub(crate) id_address: Address,
    pub(crate) node_rpc_endpoint: Option<String>,
    pub(crate) dkg_private_key: Option<Scalar>,
    pub(crate) dkg_public_key: Option<G1>,
}

impl std::fmt::Debug for InMemoryNodeInfoCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryNodeInfoCache")
            .field("id_address", &self.id_address)
            .field("node_rpc_endpoint", &self.node_rpc_endpoint)
            .field("dkg_private_key", &"ignored")
            .field("dkg_public_key", &self.dkg_public_key)
            .finish()
    }
}

impl InMemoryNodeInfoCache {
    pub fn new(id_address: Address) -> Self {
        InMemoryNodeInfoCache {
            id_address,
            node_rpc_endpoint: None,
            dkg_private_key: None,
            dkg_public_key: None,
        }
    }

    pub fn rebuild(
        id_address: Address,
        node_rpc_endpoint: String,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> Self {
        InMemoryNodeInfoCache {
            id_address,
            node_rpc_endpoint: Some(node_rpc_endpoint),
            dkg_private_key: Some(dkg_private_key),
            dkg_public_key: Some(dkg_public_key),
        }
    }
}

impl MdcContextUpdater for InMemoryNodeInfoCache {
    fn refresh_mdc_entry(&self) {
        log_mdc::insert("node_info", serde_json::to_string(&self).unwrap());
    }
}

#[async_trait]
impl NodeInfoUpdater for InMemoryNodeInfoCache {
    async fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> DataAccessResult<()> {
        self.node_rpc_endpoint = Some(node_rpc_endpoint);
        self.refresh_mdc_entry();
        Ok(())
    }

    async fn set_dkg_key_pair(
        &mut self,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> DataAccessResult<()> {
        self.dkg_private_key = Some(dkg_private_key);
        self.dkg_public_key = Some(dkg_public_key);
        self.refresh_mdc_entry();
        Ok(())
    }
}

impl NodeInfoFetcher for InMemoryNodeInfoCache {
    fn get_id_address(&self) -> DataAccessResult<Address> {
        Ok(self.id_address)
    }

    fn get_node_rpc_endpoint(&self) -> DataAccessResult<&str> {
        self.node_rpc_endpoint
            .as_ref()
            .map(|e| e as &str)
            .ok_or_else(|| NodeInfoError::NoRpcEndpoint.into())
    }

    fn get_dkg_private_key(&self) -> DataAccessResult<&Scalar> {
        self.dkg_private_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }

    fn get_dkg_public_key(&self) -> DataAccessResult<&G1> {
        self.dkg_public_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InMemoryGroupInfoCache {
    pub(crate) share: Option<Share<Scalar>>,
    pub(crate) group: Group,
    pub(crate) dkg_status: DKGStatus,
    pub(crate) self_index: usize,
    pub(crate) dkg_start_block_height: usize,
}

impl Default for InMemoryGroupInfoCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InMemoryGroupInfoCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryGroupInfoCache")
            .field("share", &"ignored")
            .field("group", &self.group)
            .field("dkg_status", &self.dkg_status)
            .field("self_index", &self.self_index)
            .field("dkg_start_block_height", &self.dkg_start_block_height)
            .finish()
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

    pub fn rebuild(
        share: Option<Share<Scalar>>,
        group: Group,
        dkg_status: DKGStatus,
        self_index: usize,
        dkg_start_block_height: usize,
    ) -> Self {
        InMemoryGroupInfoCache {
            share,
            group,
            dkg_status,
            self_index,
            dkg_start_block_height,
        }
    }

    fn only_has_group_task(&self) -> DataAccessResult<()> {
        if self.group.index == 0 {
            return Err(GroupError::NoGroupTask.into());
        }

        Ok(())
    }
}

impl MdcContextUpdater for InMemoryGroupInfoCache {
    fn refresh_mdc_entry(&self) {
        log_mdc::insert("group_info", serde_json::to_string(&self).unwrap());
    }
}

#[async_trait]
impl GroupInfoUpdater for InMemoryGroupInfoCache {
    async fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: DKGStatus,
    ) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(GroupError::GroupIndexObsolete(self.group.index).into());
        }

        if self.group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(self.group.epoch).into());
        }

        if self.dkg_status == dkg_status {
            return Ok(false);
        }

        info!(
            "dkg_status transfered from {:?} to {:?}",
            self.dkg_status, dkg_status
        );

        self.dkg_status = dkg_status;

        self.refresh_mdc_entry();

        Ok(true)
    }

    async fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> DataAccessResult<()> {
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
                id_address: *address,
                rpc_endpoint: None,
                partial_public_key: None,
            };
            self.group.members.insert(*address, member);
        });

        self.refresh_mdc_entry();

        Ok(())
    }

    async fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> DataAccessResult<(G1, G1, Vec<Address>)> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(GroupError::GroupIndexObsolete(self.group.index).into());
        }

        if self.group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(self.group.epoch).into());
        }

        if self.group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        // every member index is started from 0
        let qualified_node_indices = output
            .qual
            .nodes
            .iter()
            .map(|node| node.id() as usize)
            .collect::<Vec<_>>();

        let disqualified_nodes = self
            .group
            .members
            .iter()
            .filter(|(_, member)| !qualified_node_indices.contains(&member.index))
            .map(|(id_address, _)| *id_address)
            .collect::<Vec<_>>();

        let public_key = *output.public.public_key();

        let mut partial_public_key = G1::new();

        self.share = Some(output.share);
        self.group.size = qualified_node_indices.len();
        self.group
            .members
            .retain(|node, _| !disqualified_nodes.contains(node));
        self.group.public_key = Some(public_key);

        for (_, member) in self.group.members.iter_mut() {
            if let Some(node) = output
                .qual
                .nodes
                .iter()
                .find(|node| member.index == node.id() as usize)
            {
                if let Some(rpc_endpoint) = node.get_rpc_endpoint() {
                    member.rpc_endpoint = Some(rpc_endpoint.to_string());
                }
            }

            member.partial_public_key = Some(output.public.eval(member.index as u32).value);

            if self.self_index == member.index {
                partial_public_key = member.partial_public_key.unwrap();
            }
        }

        self.refresh_mdc_entry();

        Ok((public_key, partial_public_key, disqualified_nodes))
    }

    async fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<Address>,
    ) -> DataAccessResult<()> {
        self.only_has_group_task()?;

        if self.group.index != index {
            return Err(GroupError::GroupIndexObsolete(self.group.index).into());
        }

        if self.group.epoch != epoch {
            return Err(GroupError::GroupEpochObsolete(self.group.epoch).into());
        }

        if self.group.state {
            return Err(GroupError::GroupAlreadyReady.into());
        }

        self.group.committers = committer_indices;

        self.group.state = true;

        self.refresh_mdc_entry();

        Ok(())
    }
}

impl GroupInfoFetcher for InMemoryGroupInfoCache {
    fn get_group(&self) -> DataAccessResult<&Group> {
        self.only_has_group_task()?;

        Ok(&self.group)
    }

    fn get_index(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.index)
    }

    fn get_epoch(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.epoch)
    }

    fn get_size(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.size)
    }

    fn get_threshold(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.group.threshold)
    }

    fn get_state(&self) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.state)
    }

    fn get_self_index(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.self_index)
    }

    fn get_public_key(&self) -> DataAccessResult<&G1> {
        self.only_has_group_task()?;

        self.group
            .public_key
            .as_ref()
            .ok_or(GroupError::GroupNotExisted)
            .map_err(|e| e.into())
    }

    fn get_secret_share(&self) -> DataAccessResult<&Share<Scalar>> {
        self.only_has_group_task()?;

        self.share
            .as_ref()
            .ok_or(GroupError::GroupNotReady)
            .map_err(|e| e.into())
    }

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member>> {
        self.only_has_group_task()?;

        Ok(&self.group.members)
    }

    fn get_member(&self, id_address: Address) -> DataAccessResult<&Member> {
        self.only_has_group_task()?;

        self.group
            .members
            .get(&id_address)
            .ok_or(GroupError::MemberNotExisted)
            .map_err(|e| e.into())
    }

    fn get_committers(&self) -> DataAccessResult<Vec<Address>> {
        self.only_has_group_task()?;

        Ok(self.group.committers.clone())
    }

    fn get_dkg_start_block_height(&self) -> DataAccessResult<usize> {
        self.only_has_group_task()?;

        Ok(self.dkg_start_block_height)
    }

    fn get_dkg_status(&self) -> DataAccessResult<DKGStatus> {
        self.only_has_group_task()?;

        Ok(self.dkg_status)
    }

    fn is_committer(&self, id_address: Address) -> DataAccessResult<bool> {
        self.only_has_group_task()?;

        Ok(self.group.committers.contains(&id_address))
    }
}

#[derive(Default, Debug, Clone)]
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

#[async_trait]
impl<T: Task + Sync + Clone> BLSTasksFetcher<T> for InMemoryBLSTasksQueue<T> {
    async fn contains(&self, task_index: usize) -> DataAccessResult<bool> {
        Ok(self
            .bls_tasks
            .iter()
            .any(|task| task.task.index() == task_index))
    }

    async fn get(&self, task_index: usize) -> DataAccessResult<T> {
        self.bls_tasks
            .get(task_index)
            .map(|task| task.task.clone())
            .ok_or_else(|| BLSTaskError::TaskNotFound.into())
    }

    async fn is_handled(&self, task_index: usize) -> DataAccessResult<bool> {
        Ok(*self
            .bls_tasks
            .get(task_index)
            .map(|task| &task.state)
            .unwrap_or(&false))
    }
}

#[async_trait]
impl BLSTasksUpdater<RandomnessTask> for InMemoryBLSTasksQueue<RandomnessTask> {
    async fn add(&mut self, task: RandomnessTask) -> DataAccessResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> DataAccessResult<Vec<RandomnessTask>> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|task| !task.state)
            .filter(|task| {
                task.task.group_index == current_group_index
                    || current_block_height
                        > task.task.assignment_block_height + RANDOMNESS_TASK_EXCLUSIVE_WINDOW
            })
            .map(|task| {
                task.state = true;
                task.task.clone()
            })
            .collect::<Vec<_>>();

        Ok(available_tasks)
    }
}

#[async_trait]
impl BLSTasksUpdater<GroupRelayTask> for InMemoryBLSTasksQueue<GroupRelayTask> {
    async fn add(&mut self, task: GroupRelayTask) -> DataAccessResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        _: usize,
        current_group_index: usize,
    ) -> DataAccessResult<Vec<GroupRelayTask>> {
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

        Ok(available_tasks)
    }
}

#[async_trait]
impl BLSTasksUpdater<GroupRelayConfirmationTask>
    for InMemoryBLSTasksQueue<GroupRelayConfirmationTask>
{
    async fn add(&mut self, task: GroupRelayConfirmationTask) -> DataAccessResult<()> {
        self.bls_tasks.push(BLSTask { task, state: false });

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        _: usize,
        current_group_index: usize,
    ) -> DataAccessResult<Vec<GroupRelayConfirmationTask>> {
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

        Ok(available_tasks)
    }
}

#[derive(Debug, Default)]
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

impl ResultCache for RandomnessResultCache {
    type M = String;
}
impl ResultCache for GroupRelayResultCache {
    type M = ContractGroup;
}
impl ResultCache for GroupRelayConfirmationResultCache {
    type M = GroupRelayConfirmation;
}

#[derive(Debug)]
pub struct BLSResultCache<T: ResultCache> {
    pub result_cache: T,
    pub state: bool,
}

#[derive(Clone, Debug)]
pub struct RandomnessResultCache {
    pub group_index: usize,
    pub randomness_task_index: usize,
    pub message: String,
    pub threshold: usize,
    pub partial_signatures: HashMap<Address, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct GroupRelayResultCache {
    pub group_index: usize,
    pub group_relay_task_index: usize,
    pub relayed_group: ContractGroup,
    pub threshold: usize,
    pub partial_signatures: HashMap<Address, Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct GroupRelayConfirmationResultCache {
    pub group_index: usize,
    pub group_relay_confirmation_task_index: usize,
    pub group_relay_confirmation: GroupRelayConfirmation,
    pub threshold: usize,
    pub partial_signatures: HashMap<Address, Vec<u8>>,
}

impl<T: ResultCache> SignatureResultCacheFetcher<T> for InMemorySignatureResultCache<T> {
    fn contains(&self, signature_index: usize) -> bool {
        self.signature_result_caches.contains_key(&signature_index)
    }

    fn get(&self, signature_index: usize) -> Option<&BLSResultCache<T>> {
        self.signature_result_caches.get(&signature_index)
    }
}

impl SignatureResultCacheUpdater<RandomnessResultCache>
    for InMemorySignatureResultCache<RandomnessResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: String,
        threshold: usize,
    ) -> DataAccessResult<bool> {
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
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

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

impl SignatureResultCacheUpdater<GroupRelayResultCache>
    for InMemorySignatureResultCache<GroupRelayResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: ContractGroup,
        threshold: usize,
    ) -> DataAccessResult<bool> {
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
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

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

impl SignatureResultCacheUpdater<GroupRelayConfirmationResultCache>
    for InMemorySignatureResultCache<GroupRelayConfirmationResultCache>
{
    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: GroupRelayConfirmation,
        threshold: usize,
    ) -> DataAccessResult<bool> {
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
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&signature_index)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

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
