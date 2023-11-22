use arpa_core::RandomnessRequestType;
use arpa_core::RandomnessTask;
use arpa_dal::cache::BLSResultCache;
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::error::DataAccessError;
use arpa_dal::BLSResultCacheState;
use entity::base_randomness_task;
use entity::op_randomness_task;
use entity::randomness_task;
use ethers_core::types::Address;
use ethers_core::types::U256;
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
    fn from(randomness_record: RandomnessRecord) -> Self {
        let task = RandomnessTask {
            request_id: randomness_record.request_id.clone(),
            subscription_id: randomness_record.subscription_id as u64,
            group_index: randomness_record.group_index as u32,
            request_type: RandomnessRequestType::from(randomness_record.request_type as u8),
            params: randomness_record.params,
            requester: randomness_record.requester.parse::<Address>().unwrap(),
            seed: U256::from_big_endian(&randomness_record.seed),
            request_confirmations: randomness_record.request_confirmations as u16,
            callback_gas_limit: randomness_record.callback_gas_limit as u32,
            callback_max_gas_price: U256::from_big_endian(
                &randomness_record.callback_max_gas_price,
            ),
            assignment_block_height: randomness_record.assignment_block_height as usize,
        };

        let partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&randomness_record.partial_signatures).unwrap();

        BLSResultCache {
            result_cache: RandomnessResultCache {
                group_index: randomness_record.group_index as usize,
                message: randomness_record.message,
                randomness_task: task,
                partial_signatures,
                threshold: randomness_record.threshold as usize,
                committed_times: randomness_record.committed_times as usize,
            },
            state: BLSResultCacheState::from(randomness_record.state),
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
        seed: U256::from_big_endian(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: U256::from_big_endian(&model.callback_max_gas_price),
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
        seed: U256::from_big_endian(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: U256::from_big_endian(&model.callback_max_gas_price),
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
        seed: U256::from_big_endian(&model.seed),
        request_confirmations: model.request_confirmations as u16,
        callback_gas_limit: model.callback_gas_limit as u32,
        callback_max_gas_price: U256::from_big_endian(&model.callback_max_gas_price),
        assignment_block_height: model.assignment_block_height as usize,
    }
}
