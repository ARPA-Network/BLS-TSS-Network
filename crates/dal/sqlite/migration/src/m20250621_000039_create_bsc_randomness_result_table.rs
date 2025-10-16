use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BSCRandomnessResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BSCRandomnessResult::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::RequestId)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::GroupIndex)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::Message)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::Threshold)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::PartialSignatures)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::CommittedTimes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::State)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BSCRandomnessResult::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BSCRandomnessResult::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum BSCRandomnessResult {
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
