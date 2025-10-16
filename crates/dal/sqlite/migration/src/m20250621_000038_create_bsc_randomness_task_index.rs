use sea_orm_migration::prelude::*;

use crate::m20250621_000037_create_bsc_randomness_task_table::BSCRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessTask::Table)
                    .name("bsc_randomness_task_request_id")
                    .col(BSCRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessTask::Table)
                    .name("bsc_randomness_task_group_index")
                    .col(BSCRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessTask::Table)
                    .name("bsc_randomness_task_assignment_block_height")
                    .col(BSCRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("bsc_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("bsc_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("bsc_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
