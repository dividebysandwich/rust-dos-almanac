use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum GameVersions {
    Table,
    Platform,
}

/// The system a version runs on: `dos`, `win16` or `win32`; null when unknown.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(GameVersions::Table).add_column(string_len_null(GameVersions::Platform, 8)).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(Table::alter().table(GameVersions::Table).drop_column(GameVersions::Platform).to_owned()).await
    }
}
