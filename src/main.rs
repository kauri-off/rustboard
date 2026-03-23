mod config;
mod db;
mod error;
mod i18n;
mod models;
mod rate_limit;
mod routes;
mod templates;
mod utils;

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::ConnectInfo, routing::get};
use sha2::{Digest, Sha256};
use tower_http::{services::ServeDir, trace::{self, TraceLayer}};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use config::AppConfig;
use models::Board;
use rate_limit::RateLimiter;

pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub config: AppConfig,
    pub boards: Vec<Board>,
    pub rate_limiter: RateLimiter,
    pub css_hash: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;

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

    let css_bytes = tokio::fs::read("static/style.css").await.unwrap_or_default();
    let css_hash = hex::encode(&Sha256::digest(&css_bytes)[..8]);
    error::set_css_hash(css_hash.clone());

    let cooldown = config.limits.post_cooldown_secs;
    let state = Arc::new(AppState {
        pool,
        config,
        boards,
        rate_limiter: RateLimiter::new(cooldown),
        css_hash,
    });

    let upload_dir = state.config.database.upload_dir.clone();
    let bind_addr = state.config.server.bind_addr.clone();

    let app = Router::new()
        .route("/set-lang", axum::routing::post(routes::lang::set_lang))
        .route("/", get(routes::boards::board_list))
        .route(
            "/robots.txt",
            get(|| async {
                (
                    [("content-type", "text/plain")],
                    "User-agent: *\nAllow: /$\nAllow: /boards$\nDisallow: /\n",
                )
            }),
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
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .nest_service("/static", ServeDir::new("static"))
        .layer(logging)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Listening on http://{bind_addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
