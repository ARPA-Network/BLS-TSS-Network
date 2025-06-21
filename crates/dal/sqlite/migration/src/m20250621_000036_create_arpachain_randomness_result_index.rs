use sea_orm_migration::prelude::*;

use crate::m20250621_000035_create_arpachain_randomness_result_table::ArpaChainRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(ArpaChainRandomnessResult::Table)
                    .name("arpachain_randomness_result_request_id")
                    .col(ArpaChainRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ArpaChainRandomnessResult::Table)
                    .name("arpachain_randomness_result_group_index")
                    .col(ArpaChainRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ArpaChainRandomnessResult::Table)
                    .name("arpachain_randomness_result_state")
                    .col(ArpaChainRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("arpachain_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("arpachain_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("arpachain_randomness_result_state")
                    .to_owned(),
            )
            .await
    }
}
