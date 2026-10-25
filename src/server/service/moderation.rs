use super::reports::infos;
use super::{id32, log_action};
use crate::model::{ModLogEntry, ModReport};
use crate::server::auth::Actor;
use crate::server::entities::{game, game_version, moderation_log, report, user};
use crate::server::error::{Error, Result};
use crate::server::state::AppState;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait};
use std::collections::HashMap;

pub const PAGE_SIZE: u64 = 50;

/// The newest reports, hidden ones included, optionally only hidden ones or
/// only those from one source.
pub async fn reports(db: &DatabaseConnection, actor: &Actor, hidden_only: bool, source: Option<&str>, page: u64) -> Result<Vec<ModReport>> {
    actor.staff()?;
    let mut q = report::Entity::find();
    if hidden_only {
        q = q.filter(report::Column::Hidden.eq(true));
    }
    if let Some(s) = source.map(str::trim).filter(|s| !s.is_empty()) {
        q = q.filter(report::Column::SourceHash.eq(s));
    }
    let rows = q.order_by_desc(report::Column::CreatedAt).limit(PAGE_SIZE).offset(page.min(10_000) * PAGE_SIZE).all(db).await?;
    let game_ids: Vec<i32> = rows.iter().map(|r| r.game_id).collect();
    let games: HashMap<i32, game::Model> =
        game::Entity::find().filter(game::Column::Id.is_in(game_ids)).all(db).await?.into_iter().map(|g| (g.id, g)).collect();
    Ok(infos(db, actor, rows)
        .await?
        .into_iter()
        .filter_map(|r| {
            let g = games.get(&id32(r.game_id).ok()?)?;
            Some(ModReport { game_slug: g.slug.clone(), game_name: g.name.clone(), report: r })
        })
        .collect())
}

/// Hide everything one source submitted: the answer to a spam run.
/// Returns how many games, versions and reports were hidden.
pub async fn hide_source(state: &AppState, actor: &Actor, source: &str) -> Result<u64> {
    let staff = actor.staff()?.clone();
    let source = source.trim();
    if source.is_empty() {
        return Err(Error::invalid("no source given"));
    }
    let db = &state.db;
    let txn = db.begin().await?;
    let mut n = 0;
    n += report::Entity::update_many()
        .col_expr(report::Column::Hidden, Expr::value(true))
        .filter(report::Column::SourceHash.eq(source))
        .filter(report::Column::UserId.is_null())
        .exec(&txn)
        .await?
        .rows_affected;
    n += game_version::Entity::update_many()
        .col_expr(game_version::Column::Hidden, Expr::value(true))
        .filter(game_version::Column::SourceHash.eq(source))
        .exec(&txn)
        .await?
        .rows_affected;
    n += game::Entity::update_many()
        .col_expr(game::Column::Hidden, Expr::value(true))
        .filter(game::Column::SourceHash.eq(source))
        .exec(&txn)
        .await?
        .rows_affected;
    txn.commit().await?;
    log_action(db, id32(staff.id)?, "hide-source", "source", 0, Some(format!("{source}: {n} entries"))).await?;
    Ok(n)
}

pub async fn log(db: &DatabaseConnection, actor: &Actor, page: u64) -> Result<Vec<ModLogEntry>> {
    actor.staff()?;
    let rows = moderation_log::Entity::find()
        .order_by_desc(moderation_log::Column::Id)
        .limit(PAGE_SIZE)
        .offset(page.min(10_000) * PAGE_SIZE)
        .all(db)
        .await?;
    let uids: Vec<i32> = rows.iter().filter_map(|r| r.user_id).collect();
    let names: HashMap<i32, String> =
        user::Entity::find().filter(user::Column::Id.is_in(uids)).all(db).await?.into_iter().map(|u| (u.id, u.username)).collect();
    Ok(rows
        .into_iter()
        .map(|r| ModLogEntry {
            id: r.id.into(),
            user: r.user_id.and_then(|u| names.get(&u).cloned()).unwrap_or_else(|| "(deleted)".into()),
            action: r.action,
            target_type: r.target_type,
            target_id: r.target_id,
            detail: r.detail,
            at: r.at,
        })
        .collect())
}
