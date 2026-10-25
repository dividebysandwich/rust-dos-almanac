//! Staff accounts and who is making a request.

use super::client_ip;
use super::entities::{session, user};
use super::error::{Error, Result};
use super::state::AppState;
use crate::model::{Role, UserInfo};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::{HeaderMap, Method, header, request::Parts};
use rand::RngCore;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub const SESSION_COOKIE: &str = "almanac_session";
const SESSION_SECS: i64 = 14 * 86_400;
pub const MIN_PASSWORD_LEN: usize = 10;

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn hash_blocking(password: &str) -> Result<String> {
    let mut salt = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).map_err(|e| Error::Internal(e.to_string()))?;
    Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|e| Error::Internal(e.to_string()))
}

pub async fn hash_password(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || hash_blocking(&password)).await.map_err(|e| Error::Internal(e.to_string()))?
}

/// Verify against `hash`, or against a throwaway hash when there's no
/// account, so a wrong username takes as long as a wrong password.
pub async fn verify_password(password: String, hash: Option<String>) -> bool {
    static DUMMY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    tokio::task::spawn_blocking(move || {
        let dummy = DUMMY.get_or_init(|| hash_blocking("no such account").unwrap_or_default());
        let checked = hash.as_deref().unwrap_or(dummy);
        let ok = PasswordHash::new(checked).is_ok_and(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok());
        ok && hash.is_some()
    })
    .await
    .unwrap_or(false)
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(Error::invalid(format!("password must be at least {MIN_PASSWORD_LEN} characters")));
    }
    Ok(())
}

fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// A new session; returns the token to hand to the client.
pub async fn create_session(db: &DatabaseConnection, user_id: i32) -> Result<String> {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    let now = now();
    session::ActiveModel {
        token_hash: Set(token_hash(&token)),
        user_id: Set(user_id),
        created_at: Set(now),
        expires_at: Set(now + SESSION_SECS),
    }
    .insert(db)
    .await?;
    Ok(token)
}

pub async fn delete_session(db: &DatabaseConnection, token: &str) -> Result<()> {
    session::Entity::delete_by_id(token_hash(token)).exec(db).await?;
    Ok(())
}

pub async fn delete_expired_sessions(db: &DatabaseConnection) -> Result<()> {
    session::Entity::delete_many().filter(session::Column::ExpiresAt.lt(now())).exec(db).await?;
    Ok(())
}

pub fn session_cookie(token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_SECS}{secure}")
}

pub fn clear_cookie(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure}")
}

pub fn user_info(u: &user::Model) -> UserInfo {
    UserInfo { id: u.id.into(), username: u.username.clone(), role: Role::parse(&u.role).unwrap_or(Role::Moderator) }
}

async fn user_for_token(db: &DatabaseConnection, token: &str) -> Result<Option<user::Model>> {
    let Some(s) = session::Entity::find_by_id(token_hash(token)).one(db).await? else {
        return Ok(None);
    };
    if s.expires_at < now() {
        return Ok(None);
    }
    Ok(user::Entity::find_by_id(s.user_id).one(db).await?.filter(|u| !u.disabled))
}

/// Who is making a request: a staff member or not, and from where.
#[derive(Clone, Debug)]
pub struct Actor {
    pub user: Option<UserInfo>,
    /// The client address, as resolved through trusted proxies.
    pub ip: IpAddr,
    /// The rate-limit key (the address, or its IPv6 /64).
    pub key: IpAddr,
    /// `AppState::source_hash` of the key.
    pub source: String,
    /// The session token the request carried, if any.
    pub token: Option<String>,
}

impl Actor {
    pub fn is_staff(&self) -> bool {
        self.user.is_some()
    }

    pub fn staff(&self) -> Result<&UserInfo> {
        self.user.as_ref().ok_or(Error::Unauthorized)
    }

    pub fn admin(&self) -> Result<&UserInfo> {
        let u = self.staff()?;
        if u.role == Role::Admin { Ok(u) } else { Err(Error::Forbidden) }
    }

    pub fn user_id(&self) -> Option<i32> {
        self.user.as_ref().and_then(|u| i32::try_from(u.id).ok())
    }

    /// Anonymous writes spend from the address's budget; staff writes don't.
    pub fn spend_write(&self, state: &AppState) -> Result<()> {
        if self.is_staff() { Ok(()) } else { state.limits().write(self.key) }
    }
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == SESSION_COOKIE && !value.is_empty()).then(|| value.to_string())
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let v = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    v.strip_prefix("Bearer ").map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// For cookie-authenticated requests that change things: the Origin, when
/// the browser sends one, must be this site. SameSite=Lax already keeps the
/// cookie off cross-site POSTs; this also covers sibling subdomains.
fn same_origin(parts: &Parts) -> bool {
    if matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return true;
    }
    let Some(origin) = parts.headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let host = parts.headers.get(header::HOST).and_then(|v| v.to_str().ok()).unwrap_or("");
    let origin_host = origin.split_once("://").map(|(_, rest)| rest).unwrap_or(origin);
    !host.is_empty() && origin_host.eq_ignore_ascii_case(host)
}

impl FromRequestParts<AppState> for Actor {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        if let Some(actor) = parts.extensions.get::<Actor>() {
            return Ok(actor.clone());
        }
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let ip = client_ip::resolve(peer, &parts.headers, &state.shared.trusted_proxies);
        let key = client_ip::limit_key(ip);

        let token = match bearer_token(&parts.headers) {
            Some(t) => Some(t),
            None => cookie_token(&parts.headers).filter(|_| same_origin(parts)),
        };
        let user = match &token {
            Some(t) => user_for_token(&state.db, t).await?.map(|u| user_info(&u)),
            None => None,
        };
        let actor = Actor { user, ip, key, source: state.source_hash(key), token };
        parts.extensions.insert(actor.clone());
        Ok(actor)
    }
}

/// Look a user up by name, case-insensitively.
pub async fn find_user(db: &DatabaseConnection, username: &str) -> Result<Option<user::Model>> {
    Ok(user::Entity::find().filter(user::Column::Username.eq(username.trim().to_lowercase())).one(db).await?)
}
