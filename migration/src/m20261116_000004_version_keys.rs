use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum GameVersions {
    Table,
    OwnerKey,
}

const VERSION_KEY_INDEX: &str = "idx_game_versions_owner_key";

/// Versions keep the SHA-256 of the key of whoever added them, who may
/// correct their label, platform and files until staff edit them, as a
/// game's adder may correct the game. A game's first version, added with
/// it, takes the game's key.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(GameVersions::Table).add_column(string_len_null(GameVersions::OwnerKey, 64)).to_owned())
            .await?;
        manager
            .create_index(Index::create().name(VERSION_KEY_INDEX).table(GameVersions::Table).col(GameVersions::OwnerKey).to_owned())
            .await?;
        // `POST /games` adds the game and its first version within the same request.
        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE game_versions SET owner_key = (SELECT g.owner_key FROM games g \
                 WHERE g.id = game_versions.game_id AND ABS(g.created_at - game_versions.created_at) <= 5) \
                 WHERE owner_key IS NULL",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_index(Index::drop().name(VERSION_KEY_INDEX).table(GameVersions::Table).to_owned()).await?;
        manager.alter_table(Table::alter().table(GameVersions::Table).drop_column(GameVersions::OwnerKey).to_owned()).await
    }
}
