use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GroupInfo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GroupInfo::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GroupInfo::Index).integer().not_null())
                    .col(ColumnDef::new(GroupInfo::Epoch).integer().not_null())
                    .col(ColumnDef::new(GroupInfo::Size).integer().not_null())
                    .col(ColumnDef::new(GroupInfo::Threshold).integer().not_null())
                    .col(ColumnDef::new(GroupInfo::State).integer().not_null())
                    .col(ColumnDef::new(GroupInfo::PublicKey).blob(BlobSize::Medium))
                    .col(ColumnDef::new(GroupInfo::Members).text().not_null())
                    .col(ColumnDef::new(GroupInfo::Committers).text())
                    .col(ColumnDef::new(GroupInfo::Share).blob(BlobSize::Medium))
                    .col(ColumnDef::new(GroupInfo::DkgStatus).integer().not_null())
                    .col(
                        ColumnDef::new(GroupInfo::SelfMemberIndex)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GroupInfo::DkgStartBlockHeight)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GroupInfo::CreateAt).date_time().not_null())
                    .col(ColumnDef::new(GroupInfo::UpdateAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GroupInfo::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum GroupInfo {
    Table,
    Id,
    Index,
    Epoch,
    Size,
    Threshold,
    State,
    PublicKey,
    Members,
    Committers,
    Share,
    DkgStatus,
    SelfMemberIndex,
    DkgStartBlockHeight,
    CreateAt,
    UpdateAt,
}
