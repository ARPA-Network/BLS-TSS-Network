use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NodeInfo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NodeInfo::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NodeInfo::IdAddress).text().not_null())
                    .col(ColumnDef::new(NodeInfo::NodeRpcEndpoint).text().not_null())
                    .col(
                        ColumnDef::new(NodeInfo::DkgPrivateKey)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NodeInfo::DkgPublicKey)
                            .blob(BlobSize::Medium)
                            .not_null(),
                    )
                    .col(ColumnDef::new(NodeInfo::CreateAt).date_time().not_null())
                    .col(ColumnDef::new(NodeInfo::UpdateAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NodeInfo::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum NodeInfo {
    Table,
    Id,
    IdAddress,
    NodeRpcEndpoint,
    DkgPrivateKey,
    DkgPublicKey,
    CreateAt,
    UpdateAt,
}
