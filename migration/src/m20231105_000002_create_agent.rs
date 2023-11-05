use sea_orm_migration::prelude::*;

use crate::m20231105_000001_create_user::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Agent::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Agent::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Agent::Name).string().not_null())
                    .col(ColumnDef::new(Agent::Language).string().not_null())
                    .col(ColumnDef::new(Agent::Directory).string().not_null())
                    .col(ColumnDef::new(Agent::Rating).float().not_null().default(1000.0))
                    .col(ColumnDef::new(Agent::TotalScore).float().not_null().default(0.0))
                    .col(ColumnDef::new(Agent::NumGames).integer().not_null().default(0))
                    .col(ColumnDef::new(Agent::Removed).boolean().not_null().default(false))
                    .col(ColumnDef::new(Agent::ErrorFile).string())
                    .col(ColumnDef::new(Agent::SourceFile).string())
                    .col(ColumnDef::new(Agent::InGame).boolean().not_null().default(false))
                    .col(ColumnDef::new(Agent::OwnerId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Agent::Table, Agent::OwnerId)
                            .to(User::Table, User::Id)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Agent::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Agent {
    Table,

    Id,
    Name,
    Language,
    Directory,

    InGame,
    
    Rating,
    TotalScore,
    NumGames,

    Removed,
    ErrorFile,
    SourceFile,

    OwnerId
}