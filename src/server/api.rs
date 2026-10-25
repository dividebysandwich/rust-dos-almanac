//! The JSON API under `/api/v1`, for Rust-DOS and other clients.

use super::auth::Actor;
use super::error::{Error, Result};
use super::service::{games, moderation, reports, users};
use super::state::AppState;
use crate::model::*;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, Method, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};

pub fn router(state: AppState) -> Router<AppState> {
    // Any site may read and submit anonymously; credentials are never
    // allowed cross-origin, so staff cookies can't be borrowed this way.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);
    Router::new()
        .route("/schema", get(schema))
        .route("/games", get(search).post(create_game))
        .route("/games/lookup", post(lookup))
        .route("/games/{slug}", get(game).patch(update_game))
        .route("/games/{slug}/reports", get(game_reports))
        .route("/games/{slug}/config", get(config))
        .route("/games/{slug}/versions", post(add_version))
        .route("/games/{slug}/blessing", axum::routing::delete(unbless))
        .route("/versions/{id}", patch(update_version))
        .route("/reports", post(create_report))
        .route("/reports/{id}", get(report).patch(update_report))
        .route("/reports/{id}/bless", post(bless))
        .route("/moderation/reports", get(mod_reports))
        .route("/moderation/hide-source", post(hide_source))
        .route("/moderation/log", get(mod_log))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", patch(update_user))
        .layer(middleware::from_fn_with_state(state, read_limit))
        .layer(cors)
}

/// Every API request spends from the address's read budget; staff are exempt.
async fn read_limit(State(state): State<AppState>, actor: Actor, req: Request, next: Next) -> Response {
    if !actor.is_staff()
        && let Err(e) = state.limits().read(actor.key)
    {
        return e.into_response();
    }
    next.run(req).await
}

async fn schema() -> Json<SchemaInfo> {
    Json(schema_info())
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    page: u64,
}

async fn search(State(s): State<AppState>, actor: Actor, Query(q): Query<SearchQuery>) -> Result<Json<Vec<GameSummary>>> {
    Ok(Json(games::search(&s.db, &actor, &q.q, q.page).await?))
}

async fn create_game(State(s): State<AppState>, actor: Actor, Json(input): Json<NewGame>) -> Result<Json<GameDetail>> {
    Ok(Json(games::create(&s, &actor, input).await?))
}

async fn lookup(State(s): State<AppState>, actor: Actor, Json(req): Json<LookupRequest>) -> Result<Json<Vec<LookupMatch>>> {
    Ok(Json(games::lookup(&s.db, &actor, req).await?))
}

async fn game(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>) -> Result<Json<GameDetail>> {
    Ok(Json(games::get(&s.db, &actor, &slug).await?))
}

async fn update_game(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>, Json(p): Json<GamePatch>) -> Result<Json<GameDetail>> {
    Ok(Json(games::update(&s, &actor, &slug, p).await?))
}

async fn game_reports(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>) -> Result<Json<Vec<ReportInfo>>> {
    Ok(Json(reports::for_game(&s.db, &actor, &slug).await?))
}

#[derive(Deserialize)]
struct ConfigQuery {
    version: Option<i64>,
    #[serde(default)]
    format: Option<String>,
}

/// The recommended settings, as JSON or (`format=ini`) as a config fragment.
async fn config(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>, Query(q): Query<ConfigQuery>) -> Result<Response> {
    let rec = reports::recommendation(&s.db, &actor, &slug, q.version).await?;
    match q.format.as_deref() {
        None | Some("json") => Ok(Json(rec).into_response()),
        Some("ini") => {
            let mut res = rec.ini.into_response();
            res.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
            Ok(res)
        }
        Some(_) => Err(Error::invalid("format is json or ini")),
    }
}

async fn add_version(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>, Json(v): Json<NewVersion>) -> Result<Json<VersionInfo>> {
    Ok(Json(games::add_version(&s, &actor, &slug, v).await?))
}

#[derive(Deserialize)]
struct VersionQuery {
    version: Option<i64>,
}

async fn unbless(State(s): State<AppState>, actor: Actor, Path(slug): Path<String>, Query(q): Query<VersionQuery>) -> Result<Json<serde_json::Value>> {
    reports::unbless(&s, &actor, &slug, q.version).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn update_version(State(s): State<AppState>, actor: Actor, Path(id): Path<i64>, Json(p): Json<VersionPatch>) -> Result<Json<VersionInfo>> {
    Ok(Json(games::update_version(&s, &actor, id, p).await?))
}

async fn create_report(State(s): State<AppState>, actor: Actor, Json(r): Json<NewReport>) -> Result<Json<ReportInfo>> {
    Ok(Json(reports::create(&s, &actor, r).await?))
}

async fn report(State(s): State<AppState>, actor: Actor, Path(id): Path<i64>) -> Result<Json<ReportInfo>> {
    Ok(Json(reports::get(&s.db, &actor, id).await?))
}

async fn update_report(State(s): State<AppState>, actor: Actor, Path(id): Path<i64>, Json(p): Json<ReportPatch>) -> Result<Json<ReportInfo>> {
    Ok(Json(reports::update(&s, &actor, id, p).await?))
}

async fn bless(State(s): State<AppState>, actor: Actor, Path(id): Path<i64>) -> Result<Json<ReportInfo>> {
    Ok(Json(reports::bless(&s, &actor, id).await?))
}

#[derive(Deserialize)]
struct ModQuery {
    #[serde(default)]
    hidden: bool,
    source: Option<String>,
    #[serde(default)]
    page: u64,
}

async fn mod_reports(State(s): State<AppState>, actor: Actor, Query(q): Query<ModQuery>) -> Result<Json<Vec<ModReport>>> {
    Ok(Json(moderation::reports(&s.db, &actor, q.hidden, q.source.as_deref(), q.page).await?))
}

#[derive(Deserialize)]
struct HideSource {
    source: String,
}

async fn hide_source(State(s): State<AppState>, actor: Actor, Json(b): Json<HideSource>) -> Result<Json<serde_json::Value>> {
    let hidden = moderation::hide_source(&s, &actor, &b.source).await?;
    Ok(Json(serde_json::json!({ "hidden": hidden })))
}

#[derive(Deserialize)]
struct PageQuery {
    #[serde(default)]
    page: u64,
}

async fn mod_log(State(s): State<AppState>, actor: Actor, Query(q): Query<PageQuery>) -> Result<Json<Vec<ModLogEntry>>> {
    Ok(Json(moderation::log(&s.db, &actor, q.page).await?))
}

#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
}

/// Returns a bearer token for `Authorization: Bearer <token>`.
async fn login(State(s): State<AppState>, actor: Actor, Json(b): Json<Login>) -> Result<Json<serde_json::Value>> {
    let (token, user) = users::login(&s, &actor, &b.username, &b.password).await?;
    Ok(Json(serde_json::json!({ "token": token, "user": user })))
}

async fn logout(State(s): State<AppState>, actor: Actor) -> Result<Json<serde_json::Value>> {
    users::logout(&s, &actor).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn me(actor: Actor) -> Json<Option<UserInfo>> {
    Json(actor.user)
}

async fn list_users(State(s): State<AppState>, actor: Actor) -> Result<Json<Vec<UserAdminInfo>>> {
    Ok(Json(users::list(&s.db, &actor).await?))
}

#[derive(Deserialize)]
struct NewUser {
    username: String,
    password: String,
    role: Role,
}

async fn create_user(State(s): State<AppState>, actor: Actor, Json(b): Json<NewUser>) -> Result<Json<UserAdminInfo>> {
    Ok(Json(users::create(&s, &actor, &b.username, &b.password, b.role).await?))
}

#[derive(Deserialize)]
struct UserPatch {
    role: Option<Role>,
    disabled: Option<bool>,
    password: Option<String>,
}

async fn update_user(State(s): State<AppState>, actor: Actor, Path(id): Path<i64>, Json(b): Json<UserPatch>) -> Result<Json<UserAdminInfo>> {
    Ok(Json(users::update(&s, &actor, id, b.role, b.disabled, b.password).await?))
}
