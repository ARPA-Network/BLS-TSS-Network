use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArpaChainRandomnessResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::RequestId)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::GroupIndex)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::Message)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::Threshold)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::PartialSignatures)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::CommittedTimes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::State)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ArpaChainRandomnessResult::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(ArpaChainRandomnessResult::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum ArpaChainRandomnessResult {
    Table,
    Id,
    RequestId,
    GroupIndex,
    Message,
    Threshold,
    PartialSignatures,
    CommittedTimes,
    State,
    CreateAt,
    UpdateAt,
}
