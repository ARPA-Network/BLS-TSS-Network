use alloy::primitives::{Address, U256};
use arpa_core::PartialSignature;
use arpa_core::RandomnessRequestType;
use arpa_core::RandomnessTask;
use arpa_dal::cache::BLSResultCache;
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::error::DataAccessError;
use arpa_dal::BLSResultCacheState;
use entity::arpa_chain_randomness_task;
use entity::b3_randomness_task;
use entity::base_randomness_task;
use entity::bsc_randomness_task;
use entity::loot_randomness_task;
use entity::op_randomness_task;
use entity::randomness_task;
use entity::redstone_randomness_task;
use entity::taiko_randomness_task;
use sea_orm::FromQueryResult;
use sea_orm::{DatabaseConnection, DbErr};
use std::collections::BTreeMap;
use thiserror::Error;

pub type DBResult<A> = Result<A, DBError>;

#[derive(Debug, Error, PartialEq)]
pub enum DBError {
    #[error(transparent)]
    DbError(#[from] DbErr),
}

impl From<DBError> for DataAccessError {
    fn from(e: DBError) -> Self {
        DataAccessError::DBError(anyhow::Error::from(e))
    }
}

#[derive(Default, Debug, Clone)]
pub struct SqliteDB {
    pub(crate) connection: DatabaseConnection,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(crate) struct RandomnessRecord {
    // result
    pub request_id: Vec<u8>,
    pub group_index: i32,
    pub message: Vec<u8>,
    pub threshold: i32,
    pub partial_signatures: String,
    pub committed_times: i32,
    pub state: i32,
    // task
    pub subscription_id: i32,
    pub request_type: i32,
    pub params: Vec<u8>,
    pub requester: String,
    pub seed: Vec<u8>,
    pub request_confirmations: i32,
    pub callback_gas_limit: i32,
    pub callback_max_gas_price: Vec<u8>,
    pub assignment_block_height: i32,
}

impl From<RandomnessRecord> for BLSResultCache<RandomnessResultCache> {
    fn from(model: RandomnessRecord) -> Self {
        let task = RandomnessTask {
            request_id: model.request_id.clone(),
            subscription_id: model.subscription_id as u64,
            group_index: model.group_index as u32,
            request_type: RandomnessRequestType::from(model.request_type as u8),
            params: model.params,
            requester: model.requester.parse::<Address>().unwrap(),
            seed: U256::from_be_slice(&model.seed),
            request_confirmations: model.request_confirmations as u16,
            callback_gas_limit: model.callback_gas_limit as u32,
            callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
            assignment_block_height: model.assignment_block_height as usize,
        };

        let partial_signatures: BTreeMap<Address, PartialSignature> =
            serde_json::from_str(&model.partial_signatures).unwrap_or(BTreeMap::new());

        BLSResultCache {
            result_cache: RandomnessResultCache {
                group_index: model.group_index as usize,
                message: model.message,
                randomness_task: task,
                partial_signatures,
                threshold: model.threshold as usize,
                committed_times: model.committed_times as usize,
            },
            state: BLSResultCacheState::from(model.state),
        }
    }
}

pub(crate) fn model_to_randomness_task(model: randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn op_model_to_randomness_task(model: op_randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn base_model_to_randomness_task(model: base_randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn redstone_model_to_randomness_task(
    model: redstone_randomness_task::Model,
) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn loot_model_to_randomness_task(model: loot_randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn taiko_model_to_randomness_task(
    model: taiko_randomness_task::Model,
) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn b3_model_to_randomness_task(model: b3_randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn bsc_model_to_randomness_task(model: bsc_randomness_task::Model) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn arpa_chain_model_to_randomness_task(
    model: arpa_chain_randomness_task::Model,
) -> RandomnessTask {
    RandomnessTask {
        request_id: model.request_id,
        subscription_id: model.subscription_id as u64,
        group_index: model.group_index as u32,
        request_type: RandomnessRequestType::from(model.request_type as u8),
        params: model.params,
        requester: model.requester.parse::<Address>().unwrap(),
        seed: U256::from_be_slice(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: compatible_u256_vec_to_u128(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}

pub(crate) fn compatible_u256_vec_to_u128(vec: &[u8]) -> u128 {
    if vec.len() == 32 {
        let u256 = U256::from_be_slice(vec);
        u256.to::<u128>()
    } else {
        u128::from_be_bytes(vec.try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;

    use crate::types::compatible_u256_vec_to_u128;

    #[test]
    fn test_u128_vec_to_u128() {
        let u: u128 = 3124;
        let vec = u.to_be_bytes().to_vec();
        assert_eq!(vec.len(), 16);
        assert_eq!(compatible_u256_vec_to_u128(&vec), 3124);
        let u128 = u128::from_be_bytes(vec.try_into().unwrap());
        assert_eq!(u128, 3124);
    }

    #[test]
    fn test_old_u256_vec_to_u128() {
        let x: U256 = U256::from(3124);
        let vec = x.to_be_bytes_vec();
        assert_eq!(vec.len(), 32);
        assert_eq!(compatible_u256_vec_to_u128(&vec), 3124);
        let u256 = U256::from_be_slice(&vec);
        let u128 = u256.to::<u128>();
        assert_eq!(u128, 3124);
    }
}
