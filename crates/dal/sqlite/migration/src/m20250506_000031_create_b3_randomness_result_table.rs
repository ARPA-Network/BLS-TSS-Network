use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(B3RandomnessResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(B3RandomnessResult::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::RequestId)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::GroupIndex)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::Message)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::Threshold)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::PartialSignatures)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::CommittedTimes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::State)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(B3RandomnessResult::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(B3RandomnessResult::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum B3RandomnessResult {
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
