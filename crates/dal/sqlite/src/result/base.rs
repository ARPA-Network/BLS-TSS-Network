use crate::types::DBError;
use crate::types::RandomnessRecord;
use crate::types::SqliteDB;
use arpa_core::format_now_date;
use arpa_core::BLSTaskError;
use arpa_core::{RandomnessTask, Task};
use arpa_dal::cache::BLSResultCache;
use arpa_dal::cache::InMemorySignatureResultCache;
use arpa_dal::cache::RandomnessResultCache;
use arpa_dal::error::DataAccessResult;
use arpa_dal::BLSResultCacheState;
use arpa_dal::ResultCache;
use arpa_dal::SignatureResultCacheFetcher;
use arpa_dal::SignatureResultCacheUpdater;
use async_trait::async_trait;
use entity::base_randomness_result;
use entity::prelude::BaseRandomnessResult;
use ethers_core::types::Address;
use migration::Expr;
use migration::Query;
use migration::SelectStatement;
use migration::SimpleExpr;
use migration::{
    BaseRandomnessResult as BaseRandomnessResultTable,
    BaseRandomnessTask as BaseRandomnessTaskTable,
};
use sea_orm::TransactionTrait;
use sea_orm::{ActiveModelTrait, DbConn, DbErr, Set};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::BTreeMap;
use std::sync::Arc;

impl SqliteDB {
    pub async fn get_base_randomness_result_client(
        &self,
    ) -> DataAccessResult<BaseSignatureResultDBClient<RandomnessResultCache>> {
        let txn = self.connection.begin().await.map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        // set commit result of committing records(if any) to not committed
        let update_stmt = Query::update()
            .table(BaseRandomnessResultTable::Table)
            .values([
                (
                    BaseRandomnessResultTable::State,
                    BLSResultCacheState::NotCommitted.to_i32().into(),
                ),
                (
                    BaseRandomnessResultTable::UpdateAt,
                    format_now_date().into(),
                ),
            ])
            .and_where(
                Expr::col(BaseRandomnessResultTable::State)
                    .eq(BLSResultCacheState::Committing.to_i32()),
            )
            .to_owned();

        self.execute_update_statement(&update_stmt).await?;

        // load all not committed records
        let query_stmt = build_randomness_record_query(Some(
            Expr::col((
                BaseRandomnessResultTable::Table,
                BaseRandomnessResultTable::State,
            ))
            .eq(BLSResultCacheState::NotCommitted.to_i32()),
        ));
        let randomness_results: Vec<RandomnessRecord> =
            self.query_all_statement(&query_stmt).await?;

        txn.commit().await.map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let results = randomness_results
            .into_iter()
            .map(|r| r.into())
            .collect::<Vec<_>>();

        Ok(BaseSignatureResultDBClient {
            db_client: Arc::new(self.clone()),
            signature_results_cache: InMemorySignatureResultCache::<RandomnessResultCache>::rebuild(
                results,
            ),
        })
    }
}

#[derive(Debug, Clone)]
pub struct BaseSignatureResultDBClient<C: ResultCache> {
    db_client: Arc<SqliteDB>,
    signature_results_cache: InMemorySignatureResultCache<C>,
}

impl BaseSignatureResultDBClient<RandomnessResultCache> {
    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

#[async_trait]
impl SignatureResultCacheFetcher<RandomnessResultCache>
    for BaseSignatureResultDBClient<RandomnessResultCache>
{
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let model =
            BaseRandomnessResultQuery::select_by_request_id(self.get_connection(), task_request_id)
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
        let query_stmt = build_randomness_record_query(Some(
            Expr::col((
                BaseRandomnessResultTable::Table,
                BaseRandomnessResultTable::RequestId,
            ))
            .eq(task_request_id),
        ));
        if let Some(randomness_record) = self
            .db_client
            .query_one_statement::<RandomnessRecord>(&query_stmt)
            .await?
        {
            return Ok(randomness_record.into());
        }
        return Err(BLSTaskError::CommitterCacheNotExisted.into());
    }
}

#[async_trait]
impl SignatureResultCacheUpdater<RandomnessResultCache>
    for BaseSignatureResultDBClient<RandomnessResultCache>
{
    async fn get_ready_to_commit_signatures(
        &mut self,
        current_block_height: usize,
    ) -> DataAccessResult<Vec<RandomnessResultCache>> {
        let ready_to_commit_signatures = self
            .signature_results_cache
            .get_ready_to_commit_signatures(current_block_height)
            .await?;

        if ready_to_commit_signatures.is_empty() {
            return Ok(vec![]);
        }

        let request_ids = ready_to_commit_signatures
            .iter()
            .map(|s| s.request_id())
            .collect::<Vec<_>>();

        let update_stmt = Query::update()
            .table(BaseRandomnessResultTable::Table)
            .values([
                (
                    BaseRandomnessResultTable::State,
                    BLSResultCacheState::Committing.to_i32().into(),
                ),
                (
                    BaseRandomnessResultTable::UpdateAt,
                    format_now_date().into(),
                ),
            ])
            .and_where(Expr::col(BaseRandomnessResultTable::RequestId).is_in(request_ids))
            .to_owned();

        self.db_client
            .execute_update_statement(&update_stmt)
            .await?;

        Ok(ready_to_commit_signatures)
    }

    async fn update_commit_result(
        &mut self,
        task_request_id: &[u8],
        status: BLSResultCacheState,
    ) -> DataAccessResult<()> {
        let update_stmt = Query::update()
            .table(BaseRandomnessResultTable::Table)
            .values([
                (BaseRandomnessResultTable::State, status.to_i32().into()),
                (
                    BaseRandomnessResultTable::UpdateAt,
                    format_now_date().into(),
                ),
            ])
            .and_where(Expr::col(BaseRandomnessResultTable::RequestId).eq(task_request_id))
            .to_owned();

        self.db_client
            .execute_update_statement(&update_stmt)
            .await?;

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
        BaseRandomnessResultMutation::add(
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
        let txn = self.get_connection().begin().await.map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        let model = BaseRandomnessResultQuery::select_by_request_id(
            self.get_connection(),
            &task_request_id,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?
        .ok_or(BLSTaskError::CommitterCacheNotExisted)?;

        BaseRandomnessResultMutation::add_partial_signature(
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

        txn.commit().await.map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        self.signature_results_cache
            .add_partial_signature(task_request_id, member_address, partial_signature)
            .await?;

        Ok(true)
    }

    async fn incr_committed_times(&mut self, task_request_id: &[u8]) -> DataAccessResult<()> {
        let update_stmt = Query::update()
            .table(BaseRandomnessResultTable::Table)
            .values([
                (
                    BaseRandomnessResultTable::CommittedTimes,
                    Expr::col(BaseRandomnessResultTable::CommittedTimes).add(1),
                ),
                (
                    BaseRandomnessResultTable::UpdateAt,
                    format_now_date().into(),
                ),
            ])
            .and_where(Expr::col(BaseRandomnessResultTable::RequestId).eq(task_request_id))
            .to_owned();

        self.db_client
            .execute_update_statement(&update_stmt)
            .await?;

        self.signature_results_cache
            .incr_committed_times(task_request_id)
            .await?;

        Ok(())
    }
}

pub struct BaseRandomnessResultQuery;

impl BaseRandomnessResultQuery {
    pub async fn select_by_request_id(
        db: &DbConn,
        request_id: &[u8],
    ) -> Result<Option<base_randomness_result::Model>, DbErr> {
        BaseRandomnessResult::find()
            .filter(base_randomness_result::Column::RequestId.eq(request_id))
            .one(db)
            .await
    }
}

pub struct BaseRandomnessResultMutation;

impl BaseRandomnessResultMutation {
    pub async fn add(
        db: &DbConn,
        request_id: Vec<u8>,
        group_index: i32,
        message: Vec<u8>,
        threshold: i32,
    ) -> Result<base_randomness_result::ActiveModel, DbErr> {
        base_randomness_result::ActiveModel {
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
        model: base_randomness_result::Model,
        member_address: Address,
        partial_signature: Vec<u8>,
    ) -> Result<base_randomness_result::Model, DbErr> {
        let mut partial_signatures: BTreeMap<Address, Vec<u8>> =
            serde_json::from_str(&model.partial_signatures).unwrap();

        partial_signatures.insert(member_address, partial_signature);

        let mut randomness_result: base_randomness_result::ActiveModel = model.into();

        randomness_result.partial_signatures =
            Set(serde_json::to_string(&partial_signatures).unwrap());

        randomness_result.update_at = Set(format_now_date());

        randomness_result.update(db).await
    }
}

pub(crate) fn build_randomness_record_query(and_where: Option<SimpleExpr>) -> SelectStatement {
    Query::select()
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::RequestId,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::GroupIndex,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::Message,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::Threshold,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::PartialSignatures,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::CommittedTimes,
        ))
        .column((
            BaseRandomnessResultTable::Table,
            BaseRandomnessResultTable::State,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::SubscriptionId,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::RequestType,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::Params,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::Requester,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::Seed,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::RequestConfirmations,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::CallbackGasLimit,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::CallbackMaxGasPrice,
        ))
        .column((
            BaseRandomnessTaskTable::Table,
            BaseRandomnessTaskTable::AssignmentBlockHeight,
        ))
        .from(BaseRandomnessResultTable::Table)
        .inner_join(
            BaseRandomnessTaskTable::Table,
            Expr::col((
                BaseRandomnessResultTable::Table,
                BaseRandomnessResultTable::RequestId,
            ))
            .equals((
                BaseRandomnessTaskTable::Table,
                BaseRandomnessTaskTable::RequestId,
            )),
        )
        .conditions(
            and_where.is_some(),
            |x| {
                x.and_where(and_where.unwrap());
            },
            |_x| {},
        )
        .to_owned()
}
