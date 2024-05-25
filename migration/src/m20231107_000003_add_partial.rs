use sea_orm_migration::prelude::*;

use crate::m20231105_000002_create_agent::Agent;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Agent::Table)
                    .add_column(
                        ColumnDef::new(Columns::Partial)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Agent::Table)
                    .drop_column(Columns::Partial)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Columns {
    Partial,
}
