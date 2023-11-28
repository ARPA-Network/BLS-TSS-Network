use sea_orm_migration::prelude::*;

use crate::m20231115_000013_create_base_randomness_task_table::BaseRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessTask::Table)
                    .name("base_randomness_task_request_id")
                    .col(BaseRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessTask::Table)
                    .name("base_randomness_task_group_index")
                    .col(BaseRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessTask::Table)
                    .name("base_randomness_task_assignment_block_height")
                    .col(BaseRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
