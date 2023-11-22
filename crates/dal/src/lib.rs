pub mod cache;
pub mod error;

use arpa_core::{DKGStatus, DKGTask, Group, Member, Task};
use async_trait::async_trait;
use cache::BLSResultCache;
pub use dkg_core::primitives::DKGOutput;
use error::DataAccessResult;
use ethers_core::types::Address;
use std::collections::BTreeMap;
use std::fmt::Debug;
use threshold_bls::{group::Curve, sig::Share};

pub trait BlockInfoHandler:
    BlockInfoFetcher + BlockInfoUpdater + std::fmt::Debug + Sync + Send
{
}

pub trait NodeInfoHandler<PC: Curve>:
    NodeInfoFetcher<PC> + NodeInfoUpdater<PC> + ContextInfoUpdater + std::fmt::Debug + Sync + Send
{
}

pub trait GroupInfoHandler<PC: Curve>:
    GroupInfoFetcher<PC> + GroupInfoUpdater<PC> + ContextInfoUpdater + std::fmt::Debug + Sync + Send
{
}
pub trait BLSTasksHandler<T: Task>:
    BLSTasksFetcher<T> + BLSTasksUpdater<T> + std::fmt::Debug + Sync + Send
{
}
pub trait SignatureResultCacheHandler<T: ResultCache>:
    SignatureResultCacheFetcher<T> + SignatureResultCacheUpdater<T> + std::fmt::Debug + Sync + Send
{
}

pub trait BlockInfoFetcher {
    fn get_block_height(&self) -> usize;

    fn get_block_time(&self) -> usize;
}

pub trait BlockInfoUpdater {
    fn set_block_height(&mut self, block_height: usize);
}

pub trait ContextInfoUpdater: std::fmt::Debug {
    fn refresh_context_entry(&self);
}

#[async_trait]
pub trait NodeInfoUpdater<C: Curve> {
    async fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> DataAccessResult<()>;

    async fn set_dkg_key_pair(
        &mut self,
        dkg_private_key: C::Scalar,
        dkg_public_key: C::Point,
    ) -> DataAccessResult<()>;
}

pub trait NodeInfoFetcher<C: Curve>: std::fmt::Debug {
    fn get_id_address(&self) -> DataAccessResult<Address>;

    fn get_node_rpc_endpoint(&self) -> DataAccessResult<&str>;

    fn get_dkg_private_key(&self) -> DataAccessResult<&C::Scalar>;

    fn get_dkg_public_key(&self) -> DataAccessResult<&C::Point>;
}

#[async_trait]
pub trait GroupInfoUpdater<C: Curve> {
    async fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> DataAccessResult<()>;

    async fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<C>,
    ) -> DataAccessResult<(C::Point, C::Point, Vec<Address>)>;

    async fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: DKGStatus,
    ) -> DataAccessResult<bool>;

    async fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<Address>,
    ) -> DataAccessResult<()>;
}

pub trait GroupInfoFetcher<C: Curve>: std::fmt::Debug {
    fn get_group(&self) -> DataAccessResult<&Group<C>>;

    fn get_index(&self) -> DataAccessResult<usize>;

    fn get_epoch(&self) -> DataAccessResult<usize>;

    fn get_size(&self) -> DataAccessResult<usize>;

    fn get_threshold(&self) -> DataAccessResult<usize>;

    fn get_state(&self) -> DataAccessResult<bool>;

    fn get_self_index(&self) -> DataAccessResult<usize>;

    fn get_public_key(&self) -> DataAccessResult<&C::Point>;

    fn get_secret_share(&self) -> DataAccessResult<&Share<C::Scalar>>;

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member<C>>>;

    fn get_member(&self, id_address: Address) -> DataAccessResult<&Member<C>>;

    fn get_committers(&self) -> DataAccessResult<Vec<Address>>;

    fn get_dkg_start_block_height(&self) -> DataAccessResult<usize>;

    fn get_dkg_status(&self) -> DataAccessResult<DKGStatus>;

    fn is_committer(&self, id_address: Address) -> DataAccessResult<bool>;
}

#[async_trait]
pub trait BLSTasksFetcher<T: Task> {
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool>;

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<T>;

    async fn is_handled(&self, task_request_id: &[u8]) -> DataAccessResult<bool>;
}

#[async_trait]
pub trait BLSTasksUpdater<T: Task> {
    async fn add(&mut self, task: T) -> DataAccessResult<()>;

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
        randomness_task_exclusive_window: usize,
    ) -> DataAccessResult<Vec<T>>;
}

#[async_trait]
pub trait SignatureResultCacheFetcher<T: ResultCache> {
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool>;

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<BLSResultCache<T>>;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BLSResultCacheState {
    NotCommitted,
    Committing,
    Committed,
    CommittedByOthers,
    Expired,
    FAULTY,
}

impl BLSResultCacheState {
    pub fn to_i32(&self) -> i32 {
        match self {
            BLSResultCacheState::NotCommitted => 0,
            BLSResultCacheState::Committing => 1,
            BLSResultCacheState::Committed => 2,
            BLSResultCacheState::CommittedByOthers => 3,
            BLSResultCacheState::Expired => 4,
            BLSResultCacheState::FAULTY => 5,
        }
    }
}

impl From<i32> for BLSResultCacheState {
    fn from(b: i32) -> Self {
        match b {
            0 => BLSResultCacheState::NotCommitted,
            1 => BLSResultCacheState::Committing,
            2 => BLSResultCacheState::Committed,
            3 => BLSResultCacheState::CommittedByOthers,
            4 => BLSResultCacheState::Expired,
            5 => BLSResultCacheState::FAULTY,
            _ => panic!("Invalid BLSResultCacheState"),
        }
    }
}

#[async_trait]
pub trait SignatureResultCacheUpdater<T: ResultCache> {
    async fn get_ready_to_commit_signatures(
        &mut self,
        current_block_height: usize,
    ) -> DataAccessResult<Vec<T>>;

    async fn add(
        &mut self,
        group_index: usize,
        task: T::Task,
        message: T::M,
        threshold: usize,
    ) -> DataAccessResult<bool>;

    async fn add_partial_signature(
        &mut self,
        task_request_id: Vec<u8>,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool>;

    async fn update_commit_result(
        &mut self,
        task_request_id: &[u8],
        status: BLSResultCacheState,
    ) -> DataAccessResult<()>;

    async fn incr_committed_times(&mut self, task_request_id: &[u8]) -> DataAccessResult<()>;
}

pub trait ResultCache: Task + Clone {
    type Task: Debug;
    type M;
}
