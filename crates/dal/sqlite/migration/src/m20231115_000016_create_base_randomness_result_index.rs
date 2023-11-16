use sea_orm_migration::prelude::*;

use crate::m20231115_000015_create_base_randomness_result_table::BaseRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessResult::Table)
                    .name("base_randomness_result_request_id")
                    .col(BaseRandomnessResult::RequestId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessResult::Table)
                    .name("base_randomness_result_group_index")
                    .col(BaseRandomnessResult::GroupIndex)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(BaseRandomnessResult::Table)
                    .name("base_randomness_result_state")
                    .col(BaseRandomnessResult::State)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_result_request_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_result_group_index")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("base_randomness_result_state")
                    .to_owned(),
            )
            .await
    }
}
