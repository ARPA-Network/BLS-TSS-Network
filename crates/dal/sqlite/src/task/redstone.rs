use crate::types::redstone_model_to_randomness_task;
use crate::types::DBError;
use crate::types::SqliteDB;
use arpa_core::format_now_date;
use arpa_core::u256_to_vec;
use arpa_core::{address_to_string, RandomnessTask, Task};
use arpa_dal::error::DataAccessResult;
use arpa_dal::error::RandomnessTaskError;
use arpa_dal::{BLSTasksFetcher, BLSTasksUpdater};
use async_trait::async_trait;
use entity::prelude::RedstoneRandomnessTask;
use entity::redstone_randomness_task;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbBackend, DbConn, DbErr, EntityTrait, FromQueryResult,
    QueryFilter, Set, Statement,
};
use std::{marker::PhantomData, sync::Arc};

impl SqliteDB {
    pub fn get_redstone_bls_tasks_client<T: Task>(&self) -> RedstoneBLSTasksDBClient<T> {
        RedstoneBLSTasksDBClient {
            db_client: Arc::new(self.clone()),
            bls_tasks: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RedstoneBLSTasksDBClient<T: Task> {
    db_client: Arc<SqliteDB>,
    bls_tasks: PhantomData<T>,
}

impl RedstoneBLSTasksDBClient<RandomnessTask> {
    pub fn get_connection(&self) -> &DbConn {
        &self.db_client.connection
    }
}

#[async_trait]
impl BLSTasksFetcher<RandomnessTask> for RedstoneBLSTasksDBClient<RandomnessTask> {
    async fn contains(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let conn = &self.db_client.connection;
        let task = RedstoneRandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;
        Ok(task.is_some())
    }

    async fn get(&self, task_request_id: &[u8]) -> DataAccessResult<RandomnessTask> {
        let conn = &self.db_client.connection;
        let task = RedstoneRandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        task.map(redstone_model_to_randomness_task).ok_or_else(|| {
            RandomnessTaskError::NoRandomnessTask(format!("{:?}", task_request_id)).into()
        })
    }

    async fn is_handled(&self, task_request_id: &[u8]) -> DataAccessResult<bool> {
        let conn = &self.db_client.connection;
        let task = RedstoneRandomnessTaskQuery::select_by_request_id(conn, task_request_id)
            .await
            .map_err(|e| {
                let e: DBError = e.into();
                e
            })?;

        Ok(task.is_some() && task.unwrap().state == 1)
    }
}

#[async_trait]
impl BLSTasksUpdater<RandomnessTask> for RedstoneBLSTasksDBClient<RandomnessTask> {
    async fn add(&mut self, task: RandomnessTask) -> DataAccessResult<()> {
        let seed_bytes = u256_to_vec(&task.seed);

        RedstoneRandomnessTaskMutation::add_task(
            self.get_connection(),
            task.request_id,
            task.subscription_id as i32,
            task.group_index as i32,
            task.request_type as i32,
            task.params,
            address_to_string(task.requester),
            seed_bytes,
            task.request_confirmations as i32,
            task.callback_gas_limit as i32,
            u256_to_vec(&task.callback_max_gas_price),
            task.assignment_block_height as i32,
        )
        .await
        .map_err(|e| {
            let e: DBError = e.into();
            e
        })?;

        Ok(())
    }

    async fn check_and_get_available_tasks(
        &mut self,
        current_block_height: usize,
        current_group_index: usize,
        randomness_task_exclusive_window: usize,
    ) -> DataAccessResult<Vec<RandomnessTask>> {
        let before_assignment_block_height =
            if current_block_height > randomness_task_exclusive_window {
                current_block_height - randomness_task_exclusive_window
            } else {
                0
            };
        RedstoneRandomnessTaskMutation::fetch_available_tasks(
            self.get_connection(),
            current_group_index as i32,
            before_assignment_block_height as i32,
        )
        .await
        .map(|models| {
            models
                .into_iter()
                .map(redstone_model_to_randomness_task)
                .collect::<Vec<_>>()
        })
        .map_err(|e| {
            let e: DBError = e.into();
            e.into()
        })
    }
}

pub struct RedstoneRandomnessTaskQuery;

impl RedstoneRandomnessTaskQuery {
    pub async fn select_by_request_id(
        db: &DbConn,
        request_id: &[u8],
    ) -> Result<Option<redstone_randomness_task::Model>, DbErr> {
        RedstoneRandomnessTask::find()
            .filter(redstone_randomness_task::Column::RequestId.eq(request_id))
            .one(db)
            .await
    }
}

pub struct RedstoneRandomnessTaskMutation;

impl RedstoneRandomnessTaskMutation {
    #[allow(clippy::too_many_arguments)]
    pub async fn add_task(
        db: &DbConn,
        request_id: Vec<u8>,
        subscription_id: i32,
        group_index: i32,
        request_type: i32,
        params: Vec<u8>,
        requester: String,
        seed: Vec<u8>,
        request_confirmations: i32,
        callback_gas_limit: i32,
        callback_max_gas_price: Vec<u8>,
        assignment_block_height: i32,
    ) -> Result<redstone_randomness_task::ActiveModel, DbErr> {
        redstone_randomness_task::ActiveModel {
            request_id: Set(request_id),
            subscription_id: Set(subscription_id),
            group_index: Set(group_index),
            request_type: Set(request_type),
            params: Set(params),
            requester: Set(requester),
            seed: Set(seed),
            request_confirmations: Set(request_confirmations),
            callback_gas_limit: Set(callback_gas_limit),
            callback_max_gas_price: Set(callback_max_gas_price),
            assignment_block_height: Set(assignment_block_height),
            create_at: Set(format_now_date()),
            update_at: Set(format_now_date()),
            state: Set(0),
            ..Default::default()
        }
        .save(db)
        .await
    }

    pub async fn fetch_available_tasks(
        db: &DbConn,
        group_index: i32,
        assignment_block_height: i32,
    ) -> Result<Vec<redstone_randomness_task::Model>, DbErr> {
        redstone_randomness_task::Model::find_by_statement(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                r#"update redstone_randomness_task set state = 1, update_at = $1 where state = 0 and (group_index = $2 or assignment_block_height < $3) 
                returning *"#,
                vec![format_now_date().into(), group_index.into(), assignment_block_height.into()],
            ))
            .all(db).await
    }
}
