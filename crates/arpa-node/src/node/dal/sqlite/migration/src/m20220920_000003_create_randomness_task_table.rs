use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RandomnessTask::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RandomnessTask::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RandomnessTask::Index).integer().not_null())
                    .col(
                        ColumnDef::new(RandomnessTask::GroupIndex)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessTask::AssignmentBlockHeight)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RandomnessTask::Message).text().not_null())
                    .col(ColumnDef::new(RandomnessTask::State).integer().not_null())
                    .col(
                        ColumnDef::new(RandomnessTask::CreateAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RandomnessTask::UpdateAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RandomnessTask::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum RandomnessTask {
    Table,
    Id,
    Index,
    GroupIndex,
    AssignmentBlockHeight,
    Message,
    State,
    CreateAt,
    UpdateAt,
}
