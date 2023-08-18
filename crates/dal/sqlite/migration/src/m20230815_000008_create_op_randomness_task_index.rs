use sea_orm_migration::prelude::*;

use crate::m20230815_000007_create_op_randomness_task_table::OPRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessTask::Table)
                    .name("op_randomness_task_request_id")
                    .col(OPRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessTask::Table)
                    .name("op_randomness_task_group_index")
                    .col(OPRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessTask::Table)
                    .name("op_randomness_task_assignment_block_height")
                    .col(OPRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("op_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("op_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("op_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
