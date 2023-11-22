use sea_orm_migration::prelude::*;

use crate::m20230815_000009_create_op_randomness_result_table::OPRandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(OPRandomnessResult::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(OPRandomnessResultNewColumn::CommittedTimes)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(OPRandomnessResult::Table)
                    .drop_column(OPRandomnessResultNewColumn::CommittedTimes)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum OPRandomnessResultNewColumn {
    CommittedTimes,
}
