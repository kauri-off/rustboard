mod config;
mod db;
mod error;
mod models;
mod rate_limit;
mod routes;
mod templates;
mod utils;

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

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
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("rustboard=info".parse()?)
                .add_directive("tower_http=debug".parse()?),
        )
        .init();

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
        .route(
            "/",
            get(|| async { axum::response::Redirect::permanent("/boards") }),
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
        .layer(TraceLayer::new_for_http())
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
