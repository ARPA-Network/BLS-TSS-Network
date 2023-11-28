use crate::error::{DataAccessResult, GroupError, NodeInfoError};
use crate::{BLSResultCacheState, BlockInfoHandler, ContextInfoUpdater};

use super::{
    BLSTasksFetcher, BLSTasksUpdater, BlockInfoFetcher, BlockInfoUpdater, GroupInfoFetcher,
    GroupInfoUpdater, NodeInfoFetcher, NodeInfoUpdater, ResultCache, SignatureResultCacheFetcher,
    SignatureResultCacheUpdater,
};
use arpa_core::log::encoder;
use arpa_core::{BLSTask, BLSTaskError, DKGStatus, DKGTask, Group, Member, RandomnessTask, Task};
use async_trait::async_trait;
use dkg_core::primitives::DKGOutput;
use ethers_core::types::Address;
use log::info;
use std::collections::{BTreeMap, HashMap};
use threshold_bls::group::{Curve, Element};
use threshold_bls::serialize::point_to_hex;
use threshold_bls::sig::Share;

#[derive(Debug, Default)]
pub struct InMemoryBlockInfoCache {
    block_height: usize,
    block_time: usize,
}

impl InMemoryBlockInfoCache {
    pub fn new(block_time: usize) -> Self {
        InMemoryBlockInfoCache {
            block_height: 0,
            block_time,
        }
    }
}

impl BlockInfoHandler for InMemoryBlockInfoCache {}

impl BlockInfoFetcher for InMemoryBlockInfoCache {
    fn get_block_height(&self) -> usize {
        self.block_height
    }

    fn get_block_time(&self) -> usize {
        self.block_time
    }
}

impl BlockInfoUpdater for InMemoryBlockInfoCache {
    fn set_block_height(&mut self, block_height: usize) {
        self.block_height = block_height;
    }
}

#[derive(Clone)]
pub struct InMemoryNodeInfoCache<C: Curve> {
    pub(crate) id_address: Address,
    pub(crate) node_rpc_endpoint: Option<String>,
    pub(crate) dkg_private_key: Option<C::Scalar>,
    pub(crate) dkg_public_key: Option<C::Point>,
}

impl<C: Curve> std::fmt::Debug for InMemoryNodeInfoCache<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryNodeInfoCache")
            .field("id_address", &self.id_address)
            .field("node_rpc_endpoint", &self.node_rpc_endpoint)
            .field("dkg_private_key", &"ignored")
            .field(
                "dkg_public_key",
                &(self.dkg_public_key.as_ref()).map(point_to_hex),
            )
            .finish()
    }
}

impl<C: Curve> InMemoryNodeInfoCache<C> {
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
        dkg_private_key: C::Scalar,
        dkg_public_key: C::Point,
    ) -> Self {
        InMemoryNodeInfoCache {
            id_address,
            node_rpc_endpoint: Some(node_rpc_endpoint),
            dkg_private_key: Some(dkg_private_key),
            dkg_public_key: Some(dkg_public_key),
        }
    }
}

impl<C: Curve> ContextInfoUpdater for InMemoryNodeInfoCache<C> {
    fn refresh_context_entry(&self) {
        encoder::CONTEXT_INFO.write()[0] = format!("{:?}", &self);
    }
}

#[async_trait]
impl<C: Curve> NodeInfoUpdater<C> for InMemoryNodeInfoCache<C> {
    async fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> DataAccessResult<()> {
        self.node_rpc_endpoint = Some(node_rpc_endpoint);
        self.refresh_context_entry();
        Ok(())
    }

    async fn set_dkg_key_pair(
        &mut self,
        dkg_private_key: C::Scalar,
        dkg_public_key: C::Point,
    ) -> DataAccessResult<()> {
        self.dkg_private_key = Some(dkg_private_key);
        self.dkg_public_key = Some(dkg_public_key);
        self.refresh_context_entry();
        Ok(())
    }
}

impl<C: Curve> NodeInfoFetcher<C> for InMemoryNodeInfoCache<C> {
    fn get_id_address(&self) -> DataAccessResult<Address> {
        Ok(self.id_address)
    }

    fn get_node_rpc_endpoint(&self) -> DataAccessResult<&str> {
        self.node_rpc_endpoint
            .as_ref()
            .map(|e| e as &str)
            .ok_or_else(|| NodeInfoError::NoRpcEndpoint.into())
    }

    fn get_dkg_private_key(&self) -> DataAccessResult<&C::Scalar> {
        self.dkg_private_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }

    fn get_dkg_public_key(&self) -> DataAccessResult<&C::Point> {
        self.dkg_public_key
            .as_ref()
            .ok_or_else(|| NodeInfoError::NoDKGKeyPair.into())
    }
}

#[derive(Clone)]
pub struct InMemoryGroupInfoCache<C: Curve> {
    pub(crate) share: Option<Share<C::Scalar>>,
    pub(crate) group: Group<C>,
    pub(crate) dkg_status: DKGStatus,
    pub(crate) self_index: usize,
    pub(crate) dkg_start_block_height: usize,
}

impl<C: Curve> Default for InMemoryGroupInfoCache<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Curve> std::fmt::Debug for InMemoryGroupInfoCache<C> {
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

impl<C: Curve> InMemoryGroupInfoCache<C> {
    pub fn new() -> Self {
        let group: Group<C> = Group::new();

        InMemoryGroupInfoCache {
            share: None,
            group,
            dkg_status: DKGStatus::None,
            self_index: 0,
            dkg_start_block_height: 0,
        }
    }

    pub fn rebuild(
        share: Option<Share<C::Scalar>>,
        group: Group<C>,
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
        if self.dkg_start_block_height == 0 {
            return Err(GroupError::NoGroupTask.into());
        }

        Ok(())
    }
}

impl<C: Curve> ContextInfoUpdater for InMemoryGroupInfoCache<C> {
    fn refresh_context_entry(&self) {
        encoder::CONTEXT_INFO.write()[1] = format!("{:?}", &self);
    }
}

#[async_trait]
impl<C: Curve> GroupInfoUpdater<C> for InMemoryGroupInfoCache<C> {
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

        self.refresh_context_entry();

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

        task.members
            .iter()
            .enumerate()
            .for_each(|(index, address)| {
                let member = Member {
                    index,
                    id_address: *address,
                    rpc_endpoint: None,
                    partial_public_key: None,
                };
                self.group.members.insert(*address, member);
            });

        self.refresh_context_entry();

        Ok(())
    }

    async fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<C>,
    ) -> DataAccessResult<(C::Point, C::Point, Vec<Address>)> {
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

        let public_key = output.public.public_key().clone();

        let mut partial_public_key = C::Point::new();

        let share = bincode::deserialize(&bincode::serialize(&output.share)?)?;

        self.share = Some(share);
        self.group.size = qualified_node_indices.len();
        self.group
            .members
            .retain(|node, _| !disqualified_nodes.contains(node));
        self.group.public_key = Some(public_key.clone());

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

            let member_partial_public_key = bincode::deserialize(&bincode::serialize(
                &output.public.eval(member.index as u32).value,
            )?)?;
            member.partial_public_key = Some(member_partial_public_key);

            if self.self_index == member.index {
                partial_public_key = member.partial_public_key.clone().unwrap();
            }
        }

        self.refresh_context_entry();

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

        self.refresh_context_entry();

        Ok(())
    }
}

impl<C: Curve> GroupInfoFetcher<C> for InMemoryGroupInfoCache<C> {
    fn get_group(&self) -> DataAccessResult<&Group<C>> {
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

    fn get_public_key(&self) -> DataAccessResult<&C::Point> {
        self.only_has_group_task()?;

        self.group
            .public_key
            .as_ref()
            .ok_or(GroupError::GroupNotExisted)
            .map_err(|e| e.into())
    }

    fn get_secret_share(&self) -> DataAccessResult<&Share<C::Scalar>> {
        self.only_has_group_task()?;

        self.share
            .as_ref()
            .ok_or(GroupError::GroupNotReady)
            .map_err(|e| e.into())
    }

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member<C>>> {
        self.only_has_group_task()?;

        Ok(&self.group.members)
    }

    fn get_member(&self, id_address: Address) -> DataAccessResult<&Member<C>> {
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
    bls_tasks: HashMap<Vec<u8>, BLSTask<T>>,
}

impl<T: Task> InMemoryBLSTasksQueue<T> {
    pub fn new() -> Self {
        InMemoryBLSTasksQueue {
            bls_tasks: HashMap::new(),
        }
    }
}

#[async_trait]
impl<T: Task + Sync + Clone> BLSTasksFetcher<T> for InMemoryBLSTasksQueue<T> {
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        Ok(self.bls_tasks.contains_key(task_request_id))
    }

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<T> {
        self.bls_tasks
            .get(task_request_id)
            .map(|task| task.task.clone())
            .ok_or_else(|| BLSTaskError::TaskNotFound.into())
    }

    async fn is_handled(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        Ok(*self
            .bls_tasks
            .get(task_request_id)
            .map(|task| &task.state)
            .unwrap_or(&false))
    }
}

#[async_trait]
impl BLSTasksUpdater<RandomnessTask> for InMemoryBLSTasksQueue<RandomnessTask> {
    async fn add(&mut self, task: RandomnessTask) -> DataAccessResult<()> {
        self.bls_tasks
            .insert(task.request_id().to_vec(), BLSTask { task, state: false });

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
        randomness_task_exclusive_window: usize,
    ) -> DataAccessResult<Vec<RandomnessTask>> {
        let available_tasks = self
            .bls_tasks
            .iter_mut()
            .filter(|(_, task)| !task.state)
            .filter(|(_, task)| {
                task.task.group_index == current_group_index as u32
                    || current_block_height
                        > task.task.assignment_block_height + randomness_task_exclusive_window
            })
            .map(|(_, task)| {
                task.state = true;
                task.task.clone()
            })
            .collect::<Vec<_>>();

        Ok(available_tasks)
    }
}

#[derive(Debug, Default, Clone)]
pub struct InMemorySignatureResultCache<C: ResultCache> {
    signature_result_caches: BTreeMap<Vec<u8>, BLSResultCache<C>>,
}

impl<C: ResultCache> InMemorySignatureResultCache<C> {
    pub fn new() -> Self {
        InMemorySignatureResultCache {
            signature_result_caches: BTreeMap::new(),
        }
    }

    pub fn rebuild(results: Vec<BLSResultCache<C>>) -> Self {
        let mut cache = InMemorySignatureResultCache::new();

        for result in results {
            cache
                .signature_result_caches
                .insert(result.result_cache.request_id().to_vec(), result);
        }

        cache
    }
}

impl Task for RandomnessResultCache {
    fn request_id(&self) -> &[u8] {
        &self.randomness_task.request_id
    }
}

impl ResultCache for RandomnessResultCache {
    type Task = RandomnessTask;
    type M = Vec<u8>;
}

#[derive(Debug, Clone)]
pub struct BLSResultCache<C: ResultCache> {
    pub result_cache: C,
    pub state: BLSResultCacheState,
}

#[derive(Clone, Debug)]
pub struct RandomnessResultCache {
    pub group_index: usize,
    pub randomness_task: RandomnessTask,
    pub message: Vec<u8>,
    pub threshold: usize,
    pub partial_signatures: BTreeMap<Address, Vec<u8>>,
    pub committed_times: usize,
}

#[async_trait]
impl<C: ResultCache + Send + Sync> SignatureResultCacheFetcher<C>
    for InMemorySignatureResultCache<C>
{
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        Ok(self.signature_result_caches.contains_key(task_request_id))
    }

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<BLSResultCache<C>> {
        self.signature_result_caches
            .get(task_request_id)
            .cloned()
            .ok_or_else(|| BLSTaskError::CommitterCacheNotExisted.into())
    }
}

#[async_trait]
impl SignatureResultCacheUpdater<RandomnessResultCache>
    for InMemorySignatureResultCache<RandomnessResultCache>
{
    async fn add(
        &mut self,
        group_index: usize,
        task: RandomnessTask,
        message: Vec<u8>,
        threshold: usize,
    ) -> DataAccessResult<bool> {
        let signature_result_cache = RandomnessResultCache {
            group_index,
            randomness_task: task,
            message,
            threshold,
            partial_signatures: BTreeMap::new(),
            committed_times: 0,
        };

        if self
            .signature_result_caches
            .contains_key(&signature_result_cache.randomness_task.request_id)
        {
            return Ok(false);
        }

        self.signature_result_caches.insert(
            signature_result_cache.randomness_task.request_id.clone(),
            BLSResultCache {
                result_cache: signature_result_cache,
                state: BLSResultCacheState::NotCommitted,
            },
        );

        Ok(true)
    }

    async fn add_partial_signature(
        &mut self,
        task_request_id: Vec<u8>,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(&task_request_id)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        if signature_result_cache
            .result_cache
            .partial_signatures
            .contains_key(&member_address)
        {
            return Ok(false);
        }

        signature_result_cache
            .result_cache
            .partial_signatures
            .insert(member_address, partial_signature);

        Ok(true)
    }

    async fn get_ready_to_commit_signatures(
        &mut self,
        current_block_height: usize,
    ) -> DataAccessResult<Vec<RandomnessResultCache>> {
        let ready_to_commit_signatures = self
            .signature_result_caches
            .values_mut()
            .filter(|v| {
                (current_block_height
                    >= v.result_cache.randomness_task.assignment_block_height
                        + v.result_cache.randomness_task.request_confirmations as usize)
                    && v.state == BLSResultCacheState::NotCommitted
                    && v.result_cache.partial_signatures.len() >= v.result_cache.threshold
            })
            .map(|v| {
                v.state = BLSResultCacheState::Committing;
                v.result_cache.clone()
            })
            .collect::<Vec<_>>();

        Ok(ready_to_commit_signatures)
    }

    async fn update_commit_result(
        &mut self,
        task_request_id: &[u8],
        status: BLSResultCacheState,
    ) -> DataAccessResult<()> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(task_request_id)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        signature_result_cache.state = status;

        Ok(())
    }

    async fn incr_committed_times(&mut self, task_request_id: &[u8]) -> DataAccessResult<()> {
        let signature_result_cache = self
            .signature_result_caches
            .get_mut(task_request_id)
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        signature_result_cache.result_cache.committed_times += 1;

        Ok(())
    }
}
