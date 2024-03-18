use sea_orm_migration::prelude::*;

use crate::m20240318_000027_create_taiko_randomness_result_table::TaikoRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessResult::Table)
                    .name("taiko_randomness_result_request_id")
                    .col(TaikoRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessResult::Table)
                    .name("taiko_randomness_result_group_index")
                    .col(TaikoRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(TaikoRandomnessResult::Table)
                    .name("taiko_randomness_result_state")
                    .col(TaikoRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("taiko_randomness_result_state")
                    .to_owned(),
            )
            .await
    }
}
