use sea_orm_migration::prelude::*;

use crate::m20230815_000009_create_op_randomness_result_table::OPRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessResult::Table)
                    .name("op_randomness_result_request_id")
                    .col(OPRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessResult::Table)
                    .name("op_randomness_result_group_index")
                    .col(OPRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(OPRandomnessResult::Table)
                    .name("op_randomness_result_state")
                    .col(OPRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("op_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("op_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(Index::drop().name("op_randomness_result_state").to_owned())
            .await
    }
}
