pub mod api;
pub mod auth;
pub mod client_ip;
pub mod config;
pub mod db;
pub mod entities;
pub mod error;
pub mod ratelimit;
pub mod recommend;
pub mod service;
pub mod state;

use crate::app::{App, shell};
use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::Request;
use axum::response::IntoResponse;
use axum::routing::post;
use clap::Parser;
use config::{Cli, Command, Config};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};
use state::AppState;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

/// Where the web UI's server functions live, apart from `/api/v1`.
pub const SERVER_FN_PREFIX: &str = "/_sfn";

async fn server_fn_handler(State(state): State<AppState>, req: Request<Body>) -> impl IntoResponse {
    handle_server_fns_with_context(move || provide_context(state.clone()), req).await
}

pub fn router(state: AppState) -> Router {
    let routes = generate_route_list(App);
    Router::new()
        .nest("/api/v1", api::router(state.clone()))
        .route(&format!("{SERVER_FN_PREFIX}/{{*fn_name}}"), post(server_fn_handler).get(server_fn_handler))
        .leptos_routes_with_context(
            &state,
            routes,
            {
                let state = state.clone();
                move || provide_context(state.clone())
            },
            {
                let options = state.leptos.clone();
                move || shell(options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(DefaultBodyLimit::max(512 * 1024))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// A connected, migrated state, for `serve` and for tests.
pub async fn build_state(config: &Config, leptos: LeptosOptions) -> anyhow::Result<AppState> {
    let db = db::connect(&config.database_url).await?;
    db::migrate(&db).await?;
    let secret = db::secret(&db, config.secret.as_deref()).await?;
    Ok(AppState::new(db, leptos, config, secret))
}

fn read_password(prompt: &str) -> anyhow::Result<String> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        let first = rpassword::prompt_password(prompt)?;
        let second = rpassword::prompt_password("Repeat: ")?;
        anyhow::ensure!(first == second, "the passwords differ");
        Ok(first)
    } else {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn,sea_orm=warn".into()))
        .init();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Serve) {
        Command::Migrate => {
            let db = db::connect(&cli.config.database_url).await?;
            db::migrate(&db).await?;
            println!("database is up to date");
        }
        Command::CreateUser { username, role } => {
            let db = db::connect(&cli.config.database_url).await?;
            db::migrate(&db).await?;
            let password = read_password(&format!("Password for {username}: "))?;
            let role = crate::model::Role::parse(&role).expect("clap checked the role");
            let u = service::users::create_direct(&db, &username, &password, role).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("created {} ({})", u.username, u.role.as_str());
        }
        Command::SetPassword { username } => {
            let db = db::connect(&cli.config.database_url).await?;
            let password = read_password(&format!("New password for {username}: "))?;
            service::users::set_password_direct(&db, &username, &password).await.map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("password changed; {username}'s sessions were signed out");
        }
        Command::Serve => serve(cli.config).await?,
    }
    Ok(())
}

async fn serve(config: Config) -> anyhow::Result<()> {
    let leptos = get_configuration(None)?.leptos_options;
    let addr = leptos.site_addr;
    let state = build_state(&config, leptos).await?;

    // Forget idle rate-limit entries and expired sessions now and then.
    let housekeeping = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(600));
        loop {
            tick.tick().await;
            housekeeping.limits().prune();
            if let Err(e) = auth::delete_expired_sessions(&housekeeping.db).await {
                tracing::warn!("session cleanup: {e}");
            }
        }
    });

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
