pub mod cache;
pub mod error;

use std::collections::BTreeMap;

use arpa_node_core::{DKGStatus, DKGTask, Member, Task};
use async_trait::async_trait;
use cache::BLSResultCache;
pub use dkg_core::primitives::DKGOutput;
use error::DataAccessResult;
use ethers_core::types::Address;
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    sig::Share,
};

pub trait BlockInfoFetcher {
    fn get_block_height(&self) -> usize;
}

pub trait BlockInfoUpdater {
    fn set_block_height(&mut self, block_height: usize);
}

#[async_trait]
pub trait NodeInfoUpdater {
    async fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> DataAccessResult<()>;

    async fn set_dkg_key_pair(
        &mut self,
        dkg_private_key: Scalar,
        dkg_public_key: G1,
    ) -> DataAccessResult<()>;
}

pub trait NodeInfoFetcher {
    fn get_id_address(&self) -> DataAccessResult<Address>;

    fn get_node_rpc_endpoint(&self) -> DataAccessResult<&str>;

    fn get_dkg_private_key(&self) -> DataAccessResult<&Scalar>;

    fn get_dkg_public_key(&self) -> DataAccessResult<&G1>;
}

#[async_trait]
pub trait GroupInfoUpdater {
    async fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> DataAccessResult<()>;

    async fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> DataAccessResult<(G1, G1, Vec<Address>)>;

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

pub trait GroupInfoFetcher {
    fn get_index(&self) -> DataAccessResult<usize>;

    fn get_epoch(&self) -> DataAccessResult<usize>;

    fn get_size(&self) -> DataAccessResult<usize>;

    fn get_threshold(&self) -> DataAccessResult<usize>;

    fn get_state(&self) -> DataAccessResult<bool>;

    fn get_self_index(&self) -> DataAccessResult<usize>;

    fn get_public_key(&self) -> DataAccessResult<&G1>;

    fn get_secret_share(&self) -> DataAccessResult<&Share<Scalar>>;

    fn get_members(&self) -> DataAccessResult<&BTreeMap<Address, Member>>;

    fn get_member(&self, id_address: Address) -> DataAccessResult<&Member>;

    fn get_committers(&self) -> DataAccessResult<Vec<Address>>;

    fn get_dkg_start_block_height(&self) -> DataAccessResult<usize>;

    fn get_dkg_status(&self) -> DataAccessResult<DKGStatus>;

    fn is_committer(&self, id_address: Address) -> DataAccessResult<bool>;
}

#[async_trait]
pub trait BLSTasksFetcher<T> {
    async fn contains(&self, task_index: usize) -> DataAccessResult<bool>;

    async fn get(&self, task_index: usize) -> DataAccessResult<T>;

    async fn is_handled(&self, task_index: usize) -> DataAccessResult<bool>;
}

#[async_trait]
pub trait BLSTasksUpdater<T: Task> {
    async fn add(&mut self, task: T) -> DataAccessResult<()>;

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> DataAccessResult<Vec<T>>;
}

pub trait SignatureResultCacheFetcher<T: ResultCache> {
    fn contains(&self, signature_index: usize) -> bool;

    fn get(&self, signature_index: usize) -> Option<&BLSResultCache<T>>;
}

pub trait SignatureResultCacheUpdater<T: ResultCache> {
    fn get_ready_to_commit_signatures(&mut self) -> Vec<T>;

    fn add(
        &mut self,
        group_index: usize,
        signature_index: usize,
        message: T::M,
        threshold: usize,
    ) -> DataAccessResult<bool>;

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool>;
}

pub trait ResultCache: Task {
    type M;
}
