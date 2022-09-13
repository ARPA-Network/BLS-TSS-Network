pub mod cache;
pub mod sqlite;
pub mod test_helper;
pub mod types;

use crate::node::error::NodeResult;
use dkg_core::primitives::DKGOutput;
use ethers::types::Address;
use threshold_bls::{
    curve::bls12381::{Curve, Scalar, G1},
    sig::Share,
};
use {
    cache::BLSResultCache,
    types::{DKGStatus, DKGTask, Member},
};

pub trait BlockInfoFetcher {
    fn get_block_height(&self) -> usize;
}

pub trait BlockInfoUpdater {
    fn set_block_height(&mut self, block_height: usize);
}

pub trait NodeInfoUpdater {
    fn set_node_rpc_endpoint(&mut self, node_rpc_endpoint: String) -> NodeResult<()>;

    fn set_dkg_key_pair(&mut self, dkg_private_key: Scalar, dkg_public_key: G1) -> NodeResult<()>;
}

pub trait NodeInfoFetcher {
    fn get_id_address(&self) -> Address;

    fn get_node_rpc_endpoint(&self) -> NodeResult<&str>;

    fn get_dkg_private_key(&self) -> NodeResult<&Scalar>;

    fn get_dkg_public_key(&self) -> NodeResult<&G1>;
}

pub trait GroupInfoUpdater {
    fn update_dkg_status(
        &mut self,
        index: usize,
        epoch: usize,
        dkg_status: DKGStatus,
    ) -> NodeResult<bool>;

    fn save_task_info(&mut self, self_index: usize, task: DKGTask) -> NodeResult<()>;

    fn save_output(
        &mut self,
        index: usize,
        epoch: usize,
        output: DKGOutput<Curve>,
    ) -> NodeResult<(G1, G1, Vec<Address>)>;

    fn save_committers(
        &mut self,
        index: usize,
        epoch: usize,
        committer_indices: Vec<Address>,
    ) -> NodeResult<()>;
}

pub trait GroupInfoFetcher {
    fn get_index(&self) -> NodeResult<usize>;

    fn get_epoch(&self) -> NodeResult<usize>;

    fn get_size(&self) -> NodeResult<usize>;

    fn get_threshold(&self) -> NodeResult<usize>;

    fn get_state(&self) -> NodeResult<bool>;

    fn get_public_key(&self) -> NodeResult<&G1>;

    fn get_secret_share(&self) -> NodeResult<&Share<Scalar>>;

    fn get_member(&self, id_address: Address) -> NodeResult<&Member>;

    fn get_committers(&self) -> NodeResult<Vec<Address>>;

    fn get_dkg_start_block_height(&self) -> NodeResult<usize>;

    fn get_dkg_status(&self) -> NodeResult<DKGStatus>;

    fn is_committer(&self, id_address: Address) -> NodeResult<bool>;
}

pub trait BLSTasksFetcher<T> {
    fn contains(&self, task_index: usize) -> NodeResult<bool>;

    fn get(&self, task_index: usize) -> NodeResult<T>;

    fn is_handled(&self, task_index: usize) -> NodeResult<bool>;
}

pub trait BLSTasksUpdater<T: Task> {
    fn add(&mut self, task: T) -> NodeResult<()>;

    fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
    ) -> NodeResult<Vec<T>>;
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
    ) -> NodeResult<bool>;

    fn add_partial_signature(
        &mut self,
        signature_index: usize,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> NodeResult<bool>;
}

pub trait Task {
    fn index(&self) -> usize;
}

pub trait ResultCache: Task {
    type M;
}

pub trait ChainIdentity {
    fn get_id(&self) -> usize;

    fn get_chain_id(&self) -> usize;

    fn get_id_address(&self) -> Address;

    fn get_provider_rpc_endpoint(&self) -> &str;

    fn get_contract_address(&self) -> Address;
}
