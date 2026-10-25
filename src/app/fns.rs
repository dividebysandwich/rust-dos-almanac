#![cfg_attr(not(feature = "ssr"), allow(dead_code, unused_imports))]

//! Server functions for the web UI. Each one is a thin wrapper around the
//! service layer the JSON API uses too. Form-facing ones take strings, as a
//! browser posts them, and parse them here.

use crate::model::*;
use leptos::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "ssr")]
mod ssr {
    pub use crate::server::auth::{self, Actor};
    pub use crate::server::service::{games, moderation, reports, users};
    pub use crate::server::state::AppState;
    use leptos::prelude::*;

    pub async fn ctx() -> Result<(AppState, Actor), ServerFnError> {
        let state = expect_context::<AppState>();
        let actor: Actor = leptos_axum::extract_with_state(&state).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok((state, actor))
    }

    pub fn set_cookie(value: String) {
        if let (Some(opts), Ok(v)) = (use_context::<leptos_axum::ResponseOptions>(), value.parse()) {
            opts.append_header(axum::http::header::SET_COOKIE, v);
        }
    }
}
#[cfg(feature = "ssr")]
use ssr::*;

fn opt(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn parse_year(s: &str) -> Result<Option<i32>, ServerFnError> {
    match opt(s) {
        Some(y) => y.parse().map(Some).map_err(|_| ServerFnError::new("the year is a number like 1993")),
        None => Ok(None),
    }
}

fn parse_id(s: &str) -> Result<Option<i64>, ServerFnError> {
    match opt(s) {
        Some(v) => v.parse().map(Some).map_err(|_| ServerFnError::new("bad version")),
        None => Ok(None),
    }
}

fn parse_list(s: &str) -> Vec<String> {
    s.split([',', '\n']).map(str::trim).filter(|a| !a.is_empty()).map(String::from).collect()
}

/// `{"emulator.cpu": "486"}` as the form posts it, grouped by section.
fn settings_from_form(fields: HashMap<String, String>) -> SettingsMap {
    let mut out = SettingsMap::new();
    for (k, v) in fields {
        if let Some((section, key)) = k.split_once('.')
            && !v.trim().is_empty()
        {
            out.entry(section.to_string()).or_default().insert(key.to_string(), v);
        }
    }
    out
}

#[server(prefix = "/_sfn")]
pub async fn current_user() -> Result<Option<UserInfo>, ServerFnError> {
    let (_, actor) = ctx().await?;
    Ok(actor.user)
}

#[server(prefix = "/_sfn")]
pub async fn login(username: String, password: String, next: Option<String>) -> Result<UserInfo, ServerFnError> {
    let (state, actor) = ctx().await?;
    let (token, user) = users::login(&state, &actor, &username, &password).await?;
    set_cookie(auth::session_cookie(&token, state.shared.secure_cookies));
    // Only paths on this site, never an open redirect.
    let next = next.filter(|n| n.starts_with('/') && !n.starts_with("//")).unwrap_or_else(|| "/mod".into());
    leptos_axum::redirect(&next);
    Ok(user)
}

#[server(prefix = "/_sfn")]
pub async fn logout() -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    users::logout(&state, &actor).await?;
    set_cookie(auth::clear_cookie(state.shared.secure_cookies));
    leptos_axum::redirect("/");
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn search_games(q: String, page: u64) -> Result<Vec<GameSummary>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(games::search(&state.db, &actor, &q, page).await?)
}

#[server(prefix = "/_sfn")]
pub async fn get_game(slug: String) -> Result<GameDetail, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(games::get(&state.db, &actor, &slug).await?)
}

#[server(prefix = "/_sfn")]
pub async fn get_reports(slug: String) -> Result<Vec<ReportInfo>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(reports::for_game(&state.db, &actor, &slug).await?)
}

#[server(prefix = "/_sfn")]
pub async fn get_report(id: i64) -> Result<ReportInfo, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(reports::get(&state.db, &actor, id).await?)
}

#[server(prefix = "/_sfn")]
pub async fn get_config(slug: String, version: Option<i64>) -> Result<Recommendation, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(reports::recommendation(&state.db, &actor, &slug, version).await?)
}

#[server(prefix = "/_sfn")]
pub async fn create_game(
    name: String,
    year: String,
    publisher: String,
    aliases: String,
    label: String,
    files: String,
    ra_md5: String,
) -> Result<String, ServerFnError> {
    let (state, actor) = ctx().await?;
    let version = if opt(&label).is_some() || opt(&files).is_some() || opt(&ra_md5).is_some() {
        Some(NewVersion {
            label: opt(&label).unwrap_or_else(|| "Original".into()),
            files: parse_file_lines(&files).map_err(ServerFnError::new)?,
            ra_md5: opt(&ra_md5),
        })
    } else {
        None
    };
    let input = NewGame { name, year: parse_year(&year)?, publisher: opt(&publisher), aliases: parse_list(&aliases), version };
    let g = games::create(&state, &actor, input).await?;
    leptos_axum::redirect(&format!("/games/{}", g.slug));
    Ok(g.slug)
}

#[server(prefix = "/_sfn")]
pub async fn add_version(slug: String, label: String, files: String, ra_md5: String) -> Result<i64, ServerFnError> {
    let (state, actor) = ctx().await?;
    let input = NewVersion { label, files: parse_file_lines(&files).map_err(ServerFnError::new)?, ra_md5: opt(&ra_md5) };
    let v = games::add_version(&state, &actor, &slug, input).await?;
    leptos_axum::redirect(&format!("/games/{slug}?v={}", v.id));
    Ok(v.id)
}

#[server(prefix = "/_sfn")]
pub async fn submit_report(
    game: String,
    version: String,
    rating: String,
    rust_dos_version: String,
    #[server(default)] description: String,
    #[server(default)] settings: HashMap<String, String>,
) -> Result<i64, ServerFnError> {
    let (state, actor) = ctx().await?;
    let rating = Rating::parse(&rating).ok_or_else(|| ServerFnError::new("pick how well the game runs"))?;
    let input = NewReport {
        game: game.clone(),
        version_id: parse_id(&version)?,
        rating,
        settings: settings_from_form(settings),
        rust_dos_version: opt(&rust_dos_version),
        description: opt(&description),
    };
    let r = reports::create(&state, &actor, input).await?;
    let v = r.version_id.map(|v| format!("?v={v}")).unwrap_or_default();
    leptos_axum::redirect(&format!("/games/{game}{v}#report-{}", r.id));
    Ok(r.id)
}

#[server(prefix = "/_sfn")]
pub async fn edit_report(
    id: i64,
    game: String,
    version: String,
    rating: String,
    rust_dos_version: String,
    #[server(default)] description: String,
    #[server(default)] settings: HashMap<String, String>,
) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    let rating = Rating::parse(&rating).ok_or_else(|| ServerFnError::new("pick how well the game runs"))?;
    let patch = ReportPatch {
        rating: Some(rating),
        version_id: Some(parse_id(&version)?),
        description: Some(opt(&description)),
        settings: Some(settings_from_form(settings)),
        rust_dos_version: Some(opt(&rust_dos_version)),
        hidden: None,
    };
    reports::update(&state, &actor, id, patch).await?;
    leptos_axum::redirect(&format!("/games/{game}#report-{id}"));
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn set_report_hidden(id: i64, hidden: bool) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    reports::update(&state, &actor, id, ReportPatch { hidden: Some(hidden), ..Default::default() }).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn bless_report(id: i64) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    reports::bless(&state, &actor, id).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn unbless(slug: String, version: Option<i64>) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    reports::unbless(&state, &actor, &slug, version).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn edit_game(slug: String, name: String, year: String, publisher: String, aliases: String) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    let patch = GamePatch {
        name: Some(name),
        year: Some(parse_year(&year)?),
        publisher: Some(opt(&publisher)),
        aliases: Some(parse_list(&aliases)),
        hidden: None,
    };
    games::update(&state, &actor, &slug, patch).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn set_game_hidden(slug: String, hidden: bool) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    games::update(&state, &actor, &slug, GamePatch { hidden: Some(hidden), ..Default::default() }).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn edit_version(id: i64, label: String, files: String, ra_md5: String) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    let patch = VersionPatch {
        label: Some(label),
        hidden: None,
        files: Some(parse_file_lines(&files).map_err(ServerFnError::new)?),
        ra_md5: Some(parse_list(&ra_md5)),
    };
    games::update_version(&state, &actor, id, patch).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn set_version_hidden(id: i64, hidden: bool) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    games::update_version(&state, &actor, id, VersionPatch { hidden: Some(hidden), ..Default::default() }).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn mod_reports(hidden_only: bool, source: String, page: u64) -> Result<Vec<ModReport>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(moderation::reports(&state.db, &actor, hidden_only, opt(&source).as_deref(), page).await?)
}

#[server(prefix = "/_sfn")]
pub async fn hidden_games() -> Result<Vec<GameSummary>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(games::hidden_games(&state.db, &actor).await?)
}

#[server(prefix = "/_sfn")]
pub async fn hide_source(source: String) -> Result<u64, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(moderation::hide_source(&state, &actor, &source).await?)
}

#[server(prefix = "/_sfn")]
pub async fn mod_log(page: u64) -> Result<Vec<ModLogEntry>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(moderation::log(&state.db, &actor, page).await?)
}

#[server(prefix = "/_sfn")]
pub async fn list_users() -> Result<Vec<UserAdminInfo>, ServerFnError> {
    let (state, actor) = ctx().await?;
    Ok(users::list(&state.db, &actor).await?)
}

#[server(prefix = "/_sfn")]
pub async fn create_user(username: String, password: String, role: String) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    let role = Role::parse(&role).ok_or_else(|| ServerFnError::new("unknown role"))?;
    users::create(&state, &actor, &username, &password, role).await?;
    Ok(())
}

#[server(prefix = "/_sfn")]
pub async fn update_user(id: i64, role: Option<Role>, disabled: Option<bool>, password: Option<String>) -> Result<(), ServerFnError> {
    let (state, actor) = ctx().await?;
    users::update(&state, &actor, id, role, disabled, password.and_then(|p| opt(&p))).await?;
    Ok(())
}
