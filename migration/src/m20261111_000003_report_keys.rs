use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Reports {
    Table,
    OwnerKey,
    Notes,
}

#[derive(DeriveIden)]
enum Games {
    Table,
    OwnerKey,
}

const REPORT_KEY_INDEX: &str = "idx_reports_owner_key";
const GAME_KEY_INDEX: &str = "idx_games_owner_key";

/// Reports keep the SHA-256 of the key their sender gave (a Rust-DOS game
/// profile's UUID), so the same key replaces its report instead of adding
/// one, and the player's own notes. Games keep the key of whoever added
/// them, which may correct their year and publisher until staff edit it.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite takes one column per ALTER TABLE.
        manager.alter_table(Table::alter().table(Reports::Table).add_column(string_len_null(Reports::OwnerKey, 64)).to_owned()).await?;
        manager.alter_table(Table::alter().table(Reports::Table).add_column(text_null(Reports::Notes)).to_owned()).await?;
        manager.alter_table(Table::alter().table(Games::Table).add_column(string_len_null(Games::OwnerKey, 64)).to_owned()).await?;
        manager
            .create_index(Index::create().name(REPORT_KEY_INDEX).table(Reports::Table).col(Reports::OwnerKey).unique().to_owned())
            .await?;
        manager.create_index(Index::create().name(GAME_KEY_INDEX).table(Games::Table).col(Games::OwnerKey).to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_index(Index::drop().name(GAME_KEY_INDEX).table(Games::Table).to_owned()).await?;
        manager.drop_index(Index::drop().name(REPORT_KEY_INDEX).table(Reports::Table).to_owned()).await?;
        manager.alter_table(Table::alter().table(Games::Table).drop_column(Games::OwnerKey).to_owned()).await?;
        manager.alter_table(Table::alter().table(Reports::Table).drop_column(Reports::Notes).to_owned()).await?;
        manager.alter_table(Table::alter().table(Reports::Table).drop_column(Reports::OwnerKey).to_owned()).await
    }
}
