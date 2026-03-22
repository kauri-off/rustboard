mod config;
mod db;
mod error;
mod models;
mod rate_limit;
mod routes;
mod templates;
mod utils;

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::ConnectInfo, routing::get};
use tower_http::{services::ServeDir, trace::{self, TraceLayer}};
use tracing::Level;

use config::AppConfig;
use models::Board;
use rate_limit::RateLimiter;

pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub config: AppConfig,
    pub boards: Vec<Board>,
    pub rate_limiter: RateLimiter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
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

    let config = AppConfig::from_env()?;

    tokio::fs::create_dir_all(&config.upload_dir).await?;

    let pool = db::create_pool(&config.database_url).await?;
    db::run_migrations(&pool).await?;

    let boards =
        sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards ORDER BY id")
            .fetch_all(&pool)
            .await?;

    tracing::info!("Loaded {} boards", boards.len());

    let cooldown = config.post_cooldown_secs;
    let state = Arc::new(AppState {
        pool,
        config,
        boards,
        rate_limiter: RateLimiter::new(cooldown),
    });

    let upload_dir = state.config.upload_dir.clone();
    let bind_addr = state.config.bind_addr.clone();

    let app = Router::new()
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
