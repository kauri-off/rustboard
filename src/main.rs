mod admin_auth;
mod config;
mod db;
mod error;
mod i18n;
mod models;
mod rate_limit;
mod routes;
mod templates;
mod utils;

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Instant};

use axum::http::{HeaderName, HeaderValue};
use axum::{Router, extract::ConnectInfo, routing::get};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};
use tower_http::{
    services::ServeDir,
    trace::{self, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use config::AppConfig;
use models::Board;
use rate_limit::{LoginRateLimiter, RateLimiter};

pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub config: RwLock<AppConfig>,
    pub config_path: std::path::PathBuf,
    pub boards: RwLock<Vec<Board>>,
    pub rate_limiter: RateLimiter,
    pub login_rate_limiter: LoginRateLimiter,
    pub css_hash: String,
    pub admin_sessions: Mutex<HashMap<String, Instant>>,
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (config, config_path) = AppConfig::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.server.log_level))
        .with_target(false)
        .compact()
        .init();

    let logging = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            let connect_ip = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let ip = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split(',').next())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    request
                        .headers()
                        .get("x-real-ip")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or(connect_ip);
            tracing::info_span!(
                "request",
                method = %request.method(),
                uri = %request.uri(),
                ip = %ip,
            )
        })
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO));

    tokio::fs::create_dir_all(&config.database.upload_dir).await?;

    let pool = db::create_pool(&config.database.url).await?;
    db::run_migrations(&pool).await?;

    let boards =
        sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards ORDER BY id")
            .fetch_all(&pool)
            .await?;

    tracing::info!("Loaded {} boards", boards.len());

    let css_bytes = tokio::fs::read("static/style.css")
        .await
        .unwrap_or_default();
    let css_hash = hex::encode(&Sha256::digest(&css_bytes)[..8]);
    error::set_css_hash(css_hash.clone());

    let cooldown = config.limits.post_cooldown_secs;
    let upload_dir = config.database.upload_dir.clone();
    let bind_addr = config.server.bind_addr.clone();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    let state = Arc::new(AppState {
        pool,
        config: RwLock::new(config),
        config_path,
        boards: RwLock::new(boards),
        rate_limiter: RateLimiter::new(cooldown),
        login_rate_limiter: LoginRateLimiter::new(),
        css_hash,
        admin_sessions: Mutex::new(HashMap::new()),
        shutdown_tx,
    });

    let app = Router::new()
        .route("/set-lang", axum::routing::post(routes::lang::set_lang))
        .route("/", get(routes::boards::board_list))
        .route(
            "/robots.txt",
            get(|| async { ([("content-type", "text/plain")], "User-agent: *\nAllow: /") }),
        )
        .route("/boards", get(routes::boards::board_list))
        .route(
            "/{slug}/",
            get(routes::board::board_get).post(routes::board::board_post),
        )
        .route(
            "/{slug}/{thread_id}",
            get(routes::thread::thread_get).post(routes::thread::thread_post),
        )
        // Admin routes
        .route("/admin", get(routes::admin::index))
        .route(
            "/admin/login",
            get(routes::admin::login_get).post(routes::admin::login_post),
        )
        .route("/admin/logout", axum::routing::post(routes::admin::logout))
        .route("/admin/dashboard", get(routes::admin::dashboard))
        .route(
            "/admin/boards",
            get(routes::admin::boards_get).post(routes::admin::boards_post),
        )
        .route(
            "/admin/boards/{id}/delete",
            axum::routing::post(routes::admin::board_delete),
        )
        .route("/admin/posts", get(routes::admin::posts_get))
        .route(
            "/admin/threads/{id}/delete",
            axum::routing::post(routes::admin::thread_delete),
        )
        .route(
            "/admin/posts/{id}/delete",
            axum::routing::post(routes::admin::post_delete),
        )
        .route(
            "/admin/settings",
            get(routes::admin::settings_get).post(routes::admin::settings_post),
        )
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .nest_service(
            "/static",
            tower_http::set_header::SetResponseHeader::overriding(
                ServeDir::new("static"),
                HeaderName::from_static("cache-control"),
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
        )
        .layer(logging)
        .with_state(Arc::clone(&state));

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Listening on http://{bind_addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        loop {
            shutdown_rx.changed().await.ok();
            if *shutdown_rx.borrow() {
                break;
            }
        }
    })
    .await?;

    // If restart was requested, re-execute the binary with the same arguments
    if *state.shutdown_tx.borrow() {
        let exe = std::env::current_exe()?;
        std::process::Command::new(&exe)
            .args(std::env::args().skip(1))
            .spawn()?;
    }

    Ok(())
}
