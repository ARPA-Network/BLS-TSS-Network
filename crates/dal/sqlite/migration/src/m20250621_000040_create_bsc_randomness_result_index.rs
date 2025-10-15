use sea_orm_migration::prelude::*;

use crate::m20250621_000039_create_bsc_randomness_result_table::BSCRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessResult::Table)
                    .name("bsc_randomness_result_request_id")
                    .col(BSCRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessResult::Table)
                    .name("bsc_randomness_result_group_index")
                    .col(BSCRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BSCRandomnessResult::Table)
                    .name("bsc_randomness_result_state")
                    .col(BSCRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("bsc_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("bsc_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("bsc_randomness_result_state").to_owned())
            .await
    }
}
