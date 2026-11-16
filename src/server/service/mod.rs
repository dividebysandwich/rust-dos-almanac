//! What the API and the web UI both do. Every function takes the actor and
//! checks what it may do itself, so neither front end can forget a check.

pub mod games;
pub mod moderation;
pub mod reports;
pub mod users;

use super::auth::{Actor, now};
use super::entities::{moderation_log, report};
use super::error::{Error, Result};
use crate::model::{Rating, RatingSummary};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

/// The SHA-256 of a sender's key (`model::valid_key`), as reports and
/// games keep it; None without one.
pub fn key_hash(key: Option<&str>) -> Result<Option<String>> {
    use sha2::{Digest, Sha256};
    let Some(key) = key.map(str::trim).filter(|k| !k.is_empty()) else { return Ok(None) };
    if !crate::model::valid_key(key) {
        return Err(Error::invalid("the key is 16 to 64 letters, digits and dashes, such as a UUID"));
    }
    Ok(Some(hex::encode(Sha256::digest(key.to_ascii_lowercase().as_bytes()))))
}

/// Who changes a game or version: staff, or whoever added it, by the key
/// they added it with (`owner_key`, hashed).
pub(super) enum Editor {
    Staff(i32),
    Owner,
}

pub(super) fn editor(actor: &Actor, key: Option<&str>, owner_key: Option<&str>) -> Result<Editor> {
    if let Some(u) = &actor.user {
        return Ok(Editor::Staff(id32(u.id)?));
    }
    match key_hash(key)? {
        Some(k) if owner_key == Some(k.as_str()) => Ok(Editor::Owner),
        _ => Err(Error::WrongKey),
    }
}

/// Log a change its adder made with their key, for staff to see: no user,
/// and the start of the source pseudonym, to follow it up.
pub(super) async fn log_owner_edit(db: &impl ConnectionTrait, actor: &Actor, target_type: &str, target_id: i32, changes: &[&str]) -> Result<()> {
    let source: String = actor.source.chars().take(8).collect();
    moderation_log::ActiveModel {
        user_id: Set(None),
        action: Set("owner-edit".into()),
        target_type: Set(target_type.into()),
        target_id: Set(target_id.into()),
        detail: Set(Some(format!("{} · src {source}", changes.join(", ")))),
        at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub fn id32(id: i64) -> Result<i32> {
    i32::try_from(id).map_err(|_| Error::NotFound)
}

pub async fn log_action(db: &impl ConnectionTrait, user_id: i32, action: &str, target_type: &str, target_id: i32, detail: Option<String>) -> Result<()> {
    moderation_log::ActiveModel {
        user_id: Set(Some(user_id)),
        action: Set(action.into()),
        target_type: Set(target_type.into()),
        target_id: Set(target_id.into()),
        detail: Set(detail.filter(|d| !d.is_empty())),
        at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Rating counts of public reports matching `filter`, grouped by `key`.
pub async fn ratings_by(
    db: &impl ConnectionTrait,
    filter: SimpleExpr,
    key: impl Fn(&report::Model) -> Option<i32>,
) -> Result<HashMap<i32, RatingSummary>> {
    let rows = report::Entity::find().filter(filter).filter(report::Column::Hidden.eq(false)).all(db).await?;
    let mut grouped: HashMap<i32, Vec<Rating>> = HashMap::new();
    for r in &rows {
        if let (Some(k), Some(rating)) = (key(r), Rating::parse(&r.rating)) {
            grouped.entry(k).or_default().push(rating);
        }
    }
    Ok(grouped.into_iter().map(|(k, v)| (k, RatingSummary::from_ratings(v))).collect())
}
