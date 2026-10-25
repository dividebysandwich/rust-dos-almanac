//! What the API and the web UI both do. Every function takes the actor and
//! checks what it may do itself, so neither front end can forget a check.

pub mod games;
pub mod moderation;
pub mod reports;
pub mod users;

use super::auth::now;
use super::entities::{moderation_log, report};
use super::error::{Error, Result};
use crate::model::{Rating, RatingSummary};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

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
