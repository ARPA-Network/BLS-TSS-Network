use crate::task::RandomnessTaskQuery;
use crate::types::DBError;
use crate::types::SqliteDB;
use arpa_core::format_now_date;
use arpa_core::BLSTaskError;
use arpa_core::RandomnessRequestType;
use arpa_core::{RandomnessTask, Task};
use arpa_dal::cache::BLSResultCache;
use arpa_dal::cache::InMemorySignatureResultCache;
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::error::DataAccessResult;
use arpa_dal::error::RandomnessTaskError;
use arpa_dal::BLSResultCacheState;
use arpa_dal::ResultCache;
use arpa_dal::SignatureResultCacheFetcher;
use arpa_dal::SignatureResultCacheUpdater;
use async_trait::async_trait;
use entity::prelude::RandomnessResult as RandomnessResultEntity;
use entity::randomness_result;
use ethers_core::types::Address;
use ethers_core::types::U256;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::BTreeMap;
use std::sync::Arc;

impl SqliteDB {
    pub async fn get_randomness_result_client(
        &self,
    ) -> DataAccessResult<SignatureResultDBClient<RandomnessResultCache>> {
        // set commit result of committing records(if any) to not committed
        let committing_models = RandomnessResultQuery::select_by_state(
            &self.connection,
            BLSResultCacheState::Committing.to_i32(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        for committing_model in committing_models {
            RandomnessResultMutation::update_commit_result(
                &self.connection,
                committing_model,
                BLSResultCacheState::NotCommitted.to_i32(),
            )
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        }

        // load all not committed records
        let models = RandomnessResultQuery::select_by_state(
            &self.connection,
            BLSResultCacheState::NotCommitted.to_i32(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let mut results = vec![];

        for model in models {
            let task =
                RandomnessTaskQuery::select_by_request_id(&self.connection, &model.request_id)
                    .await
                    .map_err(|e| {
                        let e: DBError = e.into();
                        e
                    })?
                    .map(|model| RandomnessTask {
                        request_id: model.request_id,
                        subscription_id: model.subscription_id as u64,
                        group_index: model.group_index as u32,
                        request_type: RandomnessRequestType::from(model.request_type as u8),
                        params: model.params,
                        requester: model.requester.parse::<Address>().unwrap(),
                        seed: U256::from_big_endian(&model.seed),
                        request_confirmations: model.request_confirmations as u16,
                        callback_gas_limit: model.callback_gas_limit as u32,
                        callback_max_gas_price: U256::from_big_endian(
                            &model.callback_max_gas_price,
                        ),
                        assignment_block_height: model.assignment_block_height as usize,
                    })
                    .ok_or_else(|| {
                        RandomnessTaskError::NoRandomnessTask(format!("{:?}", &model.request_id))
                    })?;

            let partial_signatures: BTreeMap<Address, Vec<u8>> =
                serde_json::from_str(&model.partial_signatures).unwrap();

            let signature_result_cache = RandomnessResultCache {
                group_index: model.group_index as usize,
                randomness_task: task,
                message: model.message,
                threshold: model.threshold as usize,
                partial_signatures,
                committed_times: model.committed_times as usize,
            };

            results.push(BLSResultCache {
                result_cache: signature_result_cache,
                state: BLSResultCacheState::from(model.state),
            });
        }

        Ok(SignatureResultDBClient {
            db_client: Arc::new(self.clone()),
            signature_results_cache: InMemorySignatureResultCache::<RandomnessResultCache>::rebuild(
                results,
            ),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SignatureResultDBClient<C: ResultCache> {
    db_client: Arc<SqliteDB>,
    signature_results_cache: InMemorySignatureResultCache<C>,
}

impl SignatureResultDBClient<RandomnessResultCache> {
    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

#[async_trait]
impl SignatureResultCacheFetcher<RandomnessResultCache>
    for SignatureResultDBClient<RandomnessResultCache>
{
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let model =
            RandomnessResultQuery::select_by_request_id(self.get_connection(), task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?;

        Ok(model.is_some())
    }

    async fn get(
        &self,
        task_request_id: &[u8],
    ) -> DataAccessResult<BLSResultCache<RandomnessResultCache>> {
        let model =
            RandomnessResultQuery::select_by_request_id(self.get_connection(), task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?
                .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        let task =
            RandomnessTaskQuery::select_by_request_id(self.get_connection(), task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?
                .map(|model| RandomnessTask {
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
                })
                .ok_or_else(|| {
                    RandomnessTaskError::NoRandomnessTask(format!("{:?}", task_request_id))
                })?;

        let partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&model.partial_signatures).unwrap();

        Ok(BLSResultCache {
            result_cache: RandomnessResultCache {
                group_index: model.group_index as usize,
                message: model.message,
                randomness_task: task,
                partial_signatures,
                threshold: model.threshold as usize,
                committed_times: model.committed_times as usize,
            },
            state: BLSResultCacheState::from(model.state),
        })
    }
}

#[async_trait]
impl SignatureResultCacheUpdater<RandomnessResultCache>
    for SignatureResultDBClient<RandomnessResultCache>
{
    async fn get_ready_to_commit_signatures(
        &mut self,
        current_block_height: usize,
    ) -> DataAccessResult<Vec<RandomnessResultCache>> {
        let ready_to_commit_signatures = self
            .signature_results_cache
            .get_ready_to_commit_signatures(current_block_height)
            .await?;

        for signature in ready_to_commit_signatures.iter() {
            let model = RandomnessResultQuery::select_by_request_id(
                self.get_connection(),
                signature.request_id(),
            )
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?
            .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

            RandomnessResultMutation::update_commit_result(
                self.get_connection(),
                model,
                BLSResultCacheState::Committing.to_i32(),
            )
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        }

        Ok(ready_to_commit_signatures)
    }

    async fn update_commit_result(
        &mut self,
        task_request_id: &[u8],
        status: BLSResultCacheState,
    ) -> DataAccessResult<()> {
        let model =
            RandomnessResultQuery::select_by_request_id(self.get_connection(), task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?
                .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        RandomnessResultMutation::update_commit_result(
            self.get_connection(),
            model,
            status.to_i32(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.signature_results_cache
            .update_commit_result(task_request_id, status)
            .await?;

        Ok(())
    }

    async fn add(
        &mut self,
        group_index: usize,
        task: RandomnessTask,
        message: Vec<u8>,
        threshold: usize,
    ) -> DataAccessResult<bool> {
        RandomnessResultMutation::add(
            self.get_connection(),
            task.request_id.clone(),
            group_index as i32,
            message.clone(),
            threshold as i32,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.signature_results_cache
            .add(group_index, task, message, threshold)
            .await?;

        Ok(true)
    }

    async fn add_partial_signature(
        &mut self,
        task_request_id: Vec<u8>,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> DataAccessResult<bool> {
        let model =
            RandomnessResultQuery::select_by_request_id(self.get_connection(), &task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?
                .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        RandomnessResultMutation::add_partial_signature(
            self.get_connection(),
            model,
            member_address,
            partial_signature.clone(),
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.signature_results_cache
            .add_partial_signature(task_request_id, member_address, partial_signature)
            .await?;

        Ok(true)
    }

    async fn incr_committed_times(&mut self, task_request_id: &[u8]) -> DataAccessResult<()> {
        let model =
            RandomnessResultQuery::select_by_request_id(self.get_connection(), task_request_id)
                .await
                .map_err(|e| {
                    let e: DBError = e.into();
                    e
                })?
                .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        RandomnessResultMutation::incr_committed_times(self.get_connection(), model)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        self.signature_results_cache
            .incr_committed_times(task_request_id)
            .await?;

        Ok(())
    }
}

pub struct RandomnessResultQuery;

impl RandomnessResultQuery {
    pub async fn select_by_request_id(
        db: &DbConn,
        request_id: &[u8],
    ) -> Result<Option<randomness_result::Model>, DbErr> {
        RandomnessResultEntity::find()
            .filter(randomness_result::Column::RequestId.eq(request_id))
            .one(db)
            .await
    }

    pub async fn select_by_state(
        db: &DbConn,
        state: i32,
    ) -> Result<Vec<randomness_result::Model>, DbErr> {
        RandomnessResultEntity::find()
            .filter(randomness_result::Column::State.eq(state))
            .all(db)
            .await
    }
}

pub struct RandomnessResultMutation;

impl RandomnessResultMutation {
    pub async fn add(
        db: &DbConn,
        request_id: Vec<u8>,
        group_index: i32,
        message: Vec<u8>,
        threshold: i32,
    ) -> Result<randomness_result::ActiveModel, DbErr> {
        randomness_result::ActiveModel {
            request_id: Set(request_id),
            group_index: Set(group_index),
            message: Set(message),
            threshold: Set(threshold),
            partial_signatures: Set(
                serde_json::to_string(&BTreeMap::<Address, Vec<u8>>::new()).unwrap()
            ),
            committed_times: Set(0),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(BLSResultCacheState::NotCommitted.to_i32()),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn add_partial_signature(
        db: &DbConn,
        model: randomness_result::Model,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> Result<randomness_result::Model, DbErr> {
        let mut partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&model.partial_signatures).unwrap();

        partial_signatures.insert(member_address, partial_signature);

        let mut randomness_result: randomness_result::ActiveModel = model.into();

        randomness_result.partial_signatures =
            Set(serde_json::to_string(&partial_signatures).unwrap());

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn update_commit_result(
        db: &DbConn,
        model: randomness_result::Model,
        status: i32,
    ) -> Result<randomness_result::Model, DbErr> {
        let mut randomness_result: randomness_result::ActiveModel = model.into();

        randomness_result.state = Set(status);

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }

    pub async fn incr_committed_times(
        db: &DbConn,
        model: randomness_result::Model,
    ) -> Result<randomness_result::Model, DbErr> {
        let committed_times = model.committed_times + 1;
        let mut randomness_result: randomness_result::ActiveModel = model.into();
        randomness_result.committed_times = Set(committed_times);
        randomness_result.update_at = Set(format_now_date());
        randomness_result.update(db).await
    }
}
