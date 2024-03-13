use sea_orm_migration::prelude::*;

use crate::m20240312_000021_create_loot_randomness_task_table::LootRandomnessTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessTask::Table)
                    .name("loot_randomness_task_request_id")
                    .col(LootRandomnessTask::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessTask::Table)
                    .name("loot_randomness_task_group_index")
                    .col(LootRandomnessTask::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessTask::Table)
                    .name("loot_randomness_task_assignment_block_height")
                    .col(LootRandomnessTask::AssignmentBlockHeight)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_task_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_task_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_task_assignment_block_height")
                    .to_owned(),
            )
            .await
    }
}
