use sea_orm_migration::prelude::*;

use crate::m20220920_000003_create_randomness_task_table::RandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(RandomnessTask::Table)
                    .name("request_id")
                    .col(RandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RandomnessTask::Table)
                    .name("group_index")
                    .col(RandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RandomnessTask::Table)
                    .name("assignment_block_height")
                    .col(RandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("request_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("group_index").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("assignment_block_height").to_owned())
            .await
    }
}
