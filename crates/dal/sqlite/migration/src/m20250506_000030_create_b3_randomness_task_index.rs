use sea_orm_migration::prelude::*;

use crate::m20250506_000029_create_b3_randomness_task_table::B3RandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessTask::Table)
                    .name("b3_randomness_task_request_id")
                    .col(B3RandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessTask::Table)
                    .name("b3_randomness_task_group_index")
                    .col(B3RandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessTask::Table)
                    .name("b3_randomness_task_assignment_block_height")
                    .col(B3RandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("b3_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("b3_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("b3_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
