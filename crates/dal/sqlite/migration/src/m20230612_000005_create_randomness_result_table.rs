use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RandomnessResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RandomnessResult::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::RequestId)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::GroupIndex)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::Message)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::Threshold)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::PartialSignatures)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::State)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessResult::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RandomnessResult::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum RandomnessResult {
    Table,
    Id,
    RequestId,
    GroupIndex,
    Message,
    Threshold,
    PartialSignatures,
    State,
    CreateAt,
    UpdateAt,
    CommittedTimes,
}
