use super::{id32, log_action};
use crate::model::{Role, UserAdminInfo, UserInfo};
use crate::server::auth::{self, Actor, now};
use crate::server::entities::{session, user};
use crate::server::error::{Error, Result};
use crate::server::state::AppState;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set};

fn username(name: &str) -> Result<String> {
    let n = name.trim().to_lowercase();
    if !(2..=32).contains(&n.len()) || !n.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
        return Err(Error::invalid("usernames are 2 to 32 letters, digits, '.', '_' or '-'"));
    }
    Ok(n)
}

fn admin_info(u: user::Model) -> UserAdminInfo {
    UserAdminInfo { id: u.id.into(), role: Role::parse(&u.role).unwrap_or(Role::Moderator), username: u.username, disabled: u.disabled, created_at: u.created_at }
}

/// Check a password and start a session; returns the session token.
pub async fn login(state: &AppState, actor: &Actor, name: &str, password: &str) -> Result<(String, UserInfo)> {
    state.limits().login(actor.key)?;
    let db = &state.db;
    let found = auth::find_user(db, name).await?.filter(|u| !u.disabled);
    let ok = auth::verify_password(password.to_string(), found.as_ref().map(|u| u.password_hash.clone())).await;
    let Some(u) = found.filter(|_| ok) else {
        tracing::info!(source = %actor.source, "failed login for {name:?}");
        return Err(Error::invalid("wrong username or password"));
    };
    let token = auth::create_session(db, u.id).await?;
    Ok((token, auth::user_info(&u)))
}

pub async fn logout(state: &AppState, actor: &Actor) -> Result<()> {
    if let Some(t) = &actor.token {
        auth::delete_session(&state.db, t).await?;
    }
    Ok(())
}

/// For the command line: no actor, no rate limit.
pub async fn create_direct(db: &DatabaseConnection, name: &str, password: &str, role: Role) -> Result<UserAdminInfo> {
    let name = username(name)?;
    auth::validate_password(password)?;
    if auth::find_user(db, &name).await?.is_some() {
        return Err(Error::Conflict(format!("user {name} exists")));
    }
    let hash = auth::hash_password(password.to_string()).await?;
    let u = user::ActiveModel {
        username: Set(name),
        password_hash: Set(hash),
        role: Set(role.as_str().into()),
        disabled: Set(false),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(admin_info(u))
}

pub async fn set_password_direct(db: &DatabaseConnection, name: &str, password: &str) -> Result<()> {
    auth::validate_password(password)?;
    let u = auth::find_user(db, name).await?.ok_or(Error::NotFound)?;
    let hash = auth::hash_password(password.to_string()).await?;
    let id = u.id;
    let mut m = u.into_active_model();
    m.password_hash = Set(hash);
    m.update(db).await?;
    session::Entity::delete_many().filter(session::Column::UserId.eq(id)).exec(db).await?;
    Ok(())
}

pub async fn list(db: &DatabaseConnection, actor: &Actor) -> Result<Vec<UserAdminInfo>> {
    actor.admin()?;
    Ok(user::Entity::find().order_by_asc(user::Column::Username).all(db).await?.into_iter().map(admin_info).collect())
}

pub async fn create(state: &AppState, actor: &Actor, name: &str, password: &str, role: Role) -> Result<UserAdminInfo> {
    let admin = actor.admin()?.clone();
    let u = create_direct(&state.db, name, password, role).await?;
    log_action(&state.db, id32(admin.id)?, "create", "user", id32(u.id)?, Some(format!("{} as {}", u.username, role.as_str()))).await?;
    Ok(u)
}

pub async fn update(state: &AppState, actor: &Actor, id: i64, role: Option<Role>, disabled: Option<bool>, password: Option<String>) -> Result<UserAdminInfo> {
    let admin = actor.admin()?.clone();
    let db = &state.db;
    let u = user::Entity::find_by_id(id32(id)?).one(db).await?.ok_or(Error::NotFound)?;
    if admin.id == i64::from(u.id) && (disabled == Some(true) || role.is_some_and(|r| r != Role::Admin)) {
        return Err(Error::invalid("you can't disable or demote yourself"));
    }
    let mut changes = Vec::new();
    let mut m = u.clone().into_active_model();
    if let Some(r) = role {
        m.role = Set(r.as_str().into());
        changes.push(format!("role {}", r.as_str()));
    }
    if let Some(d) = disabled {
        m.disabled = Set(d);
        changes.push(if d { "disabled".into() } else { "enabled".into() });
    }
    if let Some(p) = &password {
        auth::validate_password(p)?;
        m.password_hash = Set(auth::hash_password(p.clone()).await?);
        changes.push("password".into());
    }
    let u = m.update(db).await?;
    if disabled == Some(true) || password.is_some() {
        session::Entity::delete_many().filter(session::Column::UserId.eq(u.id)).exec(db).await?;
    }
    log_action(db, id32(admin.id)?, "edit", "user", u.id, Some(changes.join(", "))).await?;
    Ok(admin_info(u))
}
