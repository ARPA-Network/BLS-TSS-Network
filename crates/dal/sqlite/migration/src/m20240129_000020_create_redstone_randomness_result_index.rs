use sea_orm_migration::prelude::*;

use crate::m20240129_000019_create_redstone_randomness_result_table::RedstoneRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessResult::Table)
                    .name("redstone_randomness_result_request_id")
                    .col(RedstoneRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessResult::Table)
                    .name("redstone_randomness_result_group_index")
                    .col(RedstoneRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RedstoneRandomnessResult::Table)
                    .name("redstone_randomness_result_state")
                    .col(RedstoneRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("redstone_randomness_result_state")
                    .to_owned(),
            )
            .await
    }
}
