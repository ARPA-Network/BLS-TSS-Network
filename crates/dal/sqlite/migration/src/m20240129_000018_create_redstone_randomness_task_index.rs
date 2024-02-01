use sea_orm_migration::prelude::*;

use crate::m20240129_000017_create_redstone_randomness_task_table::RedstoneRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessTask::Table)
                    .name("redstone_randomness_task_request_id")
                    .col(RedstoneRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessTask::Table)
                    .name("redstone_randomness_task_group_index")
                    .col(RedstoneRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessTask::Table)
                    .name("redstone_randomness_task_assignment_block_height")
                    .col(RedstoneRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
