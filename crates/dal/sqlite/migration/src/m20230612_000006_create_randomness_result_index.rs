use sea_orm_migration::prelude::*;

use crate::m20230612_000005_create_randomness_result_table::RandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(RandomnessResult::Table)
                    .name("randomness_result_request_id")
                    .col(RandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RandomnessResult::Table)
                    .name("randomness_result_group_index")
                    .col(RandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(RandomnessResult::Table)
                    .name("randomness_result_state")
                    .col(RandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("randomness_result_state").to_owned())
            .await
    }
}
