use sea_orm_migration::prelude::*;

use crate::m20240312_000023_create_loot_randomness_result_table::LootRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessResult::Table)
                    .name("loot_randomness_result_request_id")
                    .col(LootRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessResult::Table)
                    .name("loot_randomness_result_group_index")
                    .col(LootRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(LootRandomnessResult::Table)
                    .name("loot_randomness_result_state")
                    .col(LootRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("loot_randomness_result_state")
                    .to_owned(),
            )
            .await
    }
}
