use sea_orm_migration::prelude::*;

use crate::m20250506_000031_create_b3_randomness_result_table::B3RandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessResult::Table)
                    .name("b3_randomness_result_request_id")
                    .col(B3RandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessResult::Table)
                    .name("b3_randomness_result_group_index")
                    .col(B3RandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(B3RandomnessResult::Table)
                    .name("b3_randomness_result_state")
                    .col(B3RandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("b3_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("b3_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("b3_randomness_result_state").to_owned())
            .await
    }
}
