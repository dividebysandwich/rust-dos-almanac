use super::entities::server_secret;
use migration::{Migrator, MigratorTrait};
use rand::RngCore;
use sea_orm::sqlx::sqlite::{SqliteJournalMode, SqliteSynchronous};
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, DbErr, EntityTrait, Set};
use std::time::Duration;

pub async fn connect(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(url);
    opt.sqlx_logging(false);
    if url.starts_with("sqlite:") {
        if url.contains(":memory:") {
            // Every pooled connection would get a database of its own.
            opt.max_connections(1).min_connections(1);
        }
        opt.map_sqlx_sqlite_opts(|o| {
            o.journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true)
                .busy_timeout(Duration::from_secs(5))
        });
    }
    Database::connect(opt).await
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

/// The HMAC key for client addresses: the configured one, else one made up
/// on first start and kept in the database, so pseudonyms survive restarts.
pub async fn secret(db: &DatabaseConnection, configured: Option<&str>) -> Result<Vec<u8>, DbErr> {
    if let Some(s) = configured.filter(|s| !s.is_empty()) {
        return Ok(s.as_bytes().to_vec());
    }
    const NAME: &str = "source_hmac";
    if let Some(row) = server_secret::Entity::find_by_id(NAME.to_string()).one(db).await? {
        return Ok(row.value.into_bytes());
    }
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let value = hex::encode(bytes);
    server_secret::ActiveModel { name: Set(NAME.into()), value: Set(value.clone()) }.insert(db).await?;
    Ok(value.into_bytes())
}
