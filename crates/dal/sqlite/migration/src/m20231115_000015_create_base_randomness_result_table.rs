use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BaseRandomnessResult::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BaseRandomnessResult::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::RequestId)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::GroupIndex)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::Message)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::Threshold)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::PartialSignatures)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::CommittedTimes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::State)
                            .tiny_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BaseRandomnessResult::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BaseRandomnessResult::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum BaseRandomnessResult {
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
