use sea_orm_migration::prelude::*;

use crate::m20230612_000005_create_randomness_result_table::RandomnessResult;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(RandomnessResult::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(RandomnessResultNewColumn::CommittedTimes)
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
                    .table(RandomnessResult::Table)
                    .drop_column(RandomnessResultNewColumn::CommittedTimes)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum RandomnessResultNewColumn {
    CommittedTimes,
}
