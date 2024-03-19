use sea_orm_migration::prelude::*;

use crate::m20240318_000025_create_taiko_randomness_task_table::TaikoRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessTask::Table)
                    .name("taiko_randomness_task_request_id")
                    .col(TaikoRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessTask::Table)
                    .name("taiko_randomness_task_group_index")
                    .col(TaikoRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessTask::Table)
                    .name("taiko_randomness_task_assignment_block_height")
                    .col(TaikoRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
