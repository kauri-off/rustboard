use askama::Template;
use axum::{
    Form,
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::{
    AppState,
    admin_auth::{create_session, destroy_session, extract_session_token, is_valid_session},
    error::AppError,
    models::{Board, PostWithBoard, ThreadWithBoard},
    templates::{
        AdminBoardsTemplate, AdminDashboardTemplate, AdminLoginTemplate, AdminPostsTemplate,
        AdminSettingsTemplate,
    },
    utils::real_ip,
};

// ── Auth check ──────────────────────────────────────────────────────────────

async fn check_auth(state: &Arc<AppState>, headers: &HeaderMap) -> Option<Response> {
    let token = extract_session_token(headers);
    let valid = match &token {
        Some(t) => is_valid_session(state, t).await,
        None => false,
    };
    if valid {
        None
    } else {
        Some(Redirect::to("/admin/login").into_response())
    }
}

// ── Form types ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct CreateBoardForm {
    slug: String,
    name: String,
    description: String,
}

#[derive(Deserialize)]
pub struct SettingsForm {
    bind_addr: String,
    log_level: String,
    site_name: String,
    site_url: String,
    admin_username: String,
    admin_password: String, // empty = keep existing
    threads_per_board: u32,
    post_cooldown_secs: u64,
    max_image_bytes: usize,
    max_subject_chars: usize,
    max_content_chars: usize,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn index() -> Response {
    Redirect::to("/admin/dashboard").into_response()
}

pub async fn login_get(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    // If already logged in, redirect to dashboard
    if check_auth(&state, &headers).await.is_none() {
        return Redirect::to("/admin/dashboard").into_response();
    }
    render_login(None, &state.css_hash)
}

pub async fn login_post(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Response {
    let client_ip = real_ip(&headers, &ConnectInfo(addr));

    // Check lockout before doing anything else
    if state.login_rate_limiter.is_locked(&client_ip) {
        let secs = state.login_rate_limiter.lockout_secs_remaining(&client_ip);
        let msg = format!(
            "Too many failed attempts. Try again in {} minutes.",
            (secs + 59) / 60
        );
        return render_login(Some(&msg), &state.css_hash);
    }

    let (username, password) = {
        let cfg = state.config.read().await;
        (cfg.admin.username.clone(), cfg.admin.password.clone())
    };

    if form.username == username && form.password == password {
        state.login_rate_limiter.record_success(&client_ip);
        let token = create_session(&state).await;
        let cookie = format!(
            "admin_session={}; HttpOnly; SameSite=Strict; Path=/admin; Max-Age=86400",
            token
        );
        (
            [(axum::http::header::SET_COOKIE, cookie)],
            Redirect::to("/admin/dashboard"),
        )
            .into_response()
    } else {
        state.login_rate_limiter.record_failure(&client_ip);
        let secs = state.login_rate_limiter.lockout_secs_remaining(&client_ip);
        let msg = if secs > 0 {
            format!(
                "Too many failed attempts. Try again in {} minutes.",
                (secs + 59) / 60
            )
        } else {
            "Invalid username or password.".to_string()
        };
        render_login(Some(&msg), &state.css_hash)
    }
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(token) = extract_session_token(&headers) {
        destroy_session(&state, &token).await;
    }
    let clear_cookie = "admin_session=; HttpOnly; SameSite=Strict; Path=/admin; Max-Age=0";
    (
        [(axum::http::header::SET_COOKIE, clear_cookie)],
        Redirect::to("/admin/login"),
    )
        .into_response()
}

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let board_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM boards")
        .fetch_one(&state.pool)
        .await?;
    let thread_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM threads")
        .fetch_one(&state.pool)
        .await?;
    let post_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.pool)
        .await?;

    let html = AdminDashboardTemplate {
        board_count,
        thread_count,
        post_count,
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

pub async fn boards_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let boards = state.boards.read().await.clone();
    let html = AdminBoardsTemplate {
        boards,
        error: None,
        success: None,
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

pub async fn boards_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<CreateBoardForm>,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let slug = form.slug.trim().to_lowercase();
    let name = form.name.trim().to_string();
    let description = form.description.trim().to_string();

    let render_error =
        |msg: &str, state: &AppState, boards: Vec<Board>| -> Result<Response, AppError> {
            let html = AdminBoardsTemplate {
                boards,
                error: Some(msg.to_string()),
                success: None,
                css_hash: state.css_hash.clone(),
            }
            .render()
            .map_err(|e| AppError::Internal(e.into()))?;
            Ok(Html(html).into_response())
        };

    let boards = state.boards.read().await.clone();

    if slug.is_empty() || name.is_empty() {
        return render_error("Slug and name are required.", &state, boards);
    }
    if !slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return render_error(
            "Slug may only contain letters, numbers, and hyphens.",
            &state,
            boards,
        );
    }
    if boards.iter().any(|b| b.slug == slug) {
        return render_error("A board with that slug already exists.", &state, boards);
    }

    sqlx::query("INSERT INTO boards (slug, name, description) VALUES (?, ?, ?)")
        .bind(&slug)
        .bind(&name)
        .bind(&description)
        .execute(&state.pool)
        .await?;

    // Refresh boards in state
    let updated_boards =
        sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards ORDER BY id")
            .fetch_all(&state.pool)
            .await?;
    *state.boards.write().await = updated_boards.clone();

    let html = AdminBoardsTemplate {
        boards: updated_boards,
        error: None,
        success: Some(format!("Board /{slug}/ created.")),
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

pub async fn board_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(board_id): Path<i64>,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    // Collect image paths before deletion (FK cascade will remove threads/posts)
    let thread_images: Vec<Option<String>> =
        sqlx::query_scalar("SELECT image_path FROM threads WHERE board_id = ?")
            .bind(board_id)
            .fetch_all(&state.pool)
            .await?;

    let post_images: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT p.image_path FROM posts p JOIN threads t ON p.thread_id = t.id WHERE t.board_id = ?",
    )
    .bind(board_id)
    .fetch_all(&state.pool)
    .await?;

    sqlx::query("DELETE FROM boards WHERE id = ?")
        .bind(board_id)
        .execute(&state.pool)
        .await?;

    // Clean up uploaded files
    for path in thread_images.into_iter().flatten() {
        let _ = tokio::fs::remove_file(&path).await;
    }
    for path in post_images.into_iter().flatten() {
        let _ = tokio::fs::remove_file(&path).await;
    }

    // Refresh boards in state
    let updated_boards =
        sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards ORDER BY id")
            .fetch_all(&state.pool)
            .await?;
    *state.boards.write().await = updated_boards;

    Ok(Redirect::to("/admin/boards").into_response())
}

pub async fn posts_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let threads: Vec<ThreadWithBoard> = sqlx::query_as(
        "SELECT t.id, t.board_id, t.subject, t.content, t.image_path, t.ip_hash,
                t.created_at, t.bump_at, t.post_count, b.slug as board_slug, b.name as board_name
         FROM threads t JOIN boards b ON t.board_id = b.id
         ORDER BY t.created_at DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await?;

    let posts: Vec<PostWithBoard> = sqlx::query_as(
        "SELECT p.id, p.thread_id, p.content, p.image_path, p.ip_hash, p.created_at,
                b.slug as board_slug
         FROM posts p
         JOIN threads t ON p.thread_id = t.id
         JOIN boards b ON t.board_id = b.id
         ORDER BY p.created_at DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await?;

    let html = AdminPostsTemplate {
        threads,
        posts,
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

pub async fn thread_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(thread_id): Path<i64>,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    // Collect images before deletion
    let thread_image: Option<Option<String>> =
        sqlx::query_scalar("SELECT image_path FROM threads WHERE id = ?")
            .bind(thread_id)
            .fetch_optional(&state.pool)
            .await?;

    let post_images: Vec<Option<String>> =
        sqlx::query_scalar("SELECT image_path FROM posts WHERE thread_id = ?")
            .bind(thread_id)
            .fetch_all(&state.pool)
            .await?;

    sqlx::query("DELETE FROM threads WHERE id = ?")
        .bind(thread_id)
        .execute(&state.pool)
        .await?;

    if let Some(Some(path)) = thread_image {
        let _ = tokio::fs::remove_file(&path).await;
    }
    for path in post_images.into_iter().flatten() {
        let _ = tokio::fs::remove_file(&path).await;
    }

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn post_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(post_id): Path<i64>,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let image_path: Option<Option<String>> =
        sqlx::query_scalar("SELECT image_path FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(&state.pool)
            .await?;

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.pool)
        .await?;

    if let Some(Some(path)) = image_path {
        let _ = tokio::fs::remove_file(&path).await;
    }

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn settings_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let cfg = state.config.read().await;
    let html = AdminSettingsTemplate {
        bind_addr: cfg.server.bind_addr.clone(),
        log_level: cfg.server.log_level.clone(),
        site_name: cfg.site.name.clone(),
        site_url: cfg.site.url.clone(),
        admin_username: cfg.admin.username.clone(),
        threads_per_board: cfg.limits.threads_per_board,
        post_cooldown_secs: cfg.limits.post_cooldown_secs,
        max_image_bytes: cfg.limits.max_image_bytes,
        max_subject_chars: cfg.limits.max_subject_chars,
        max_content_chars: cfg.limits.max_content_chars,
        error: None,
        success: None,
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

pub async fn settings_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<SettingsForm>,
) -> Result<Response, AppError> {
    if let Some(r) = check_auth(&state, &headers).await {
        return Ok(r);
    }

    let bind_addr = form.bind_addr.trim().to_string();
    let site_name = form.site_name.trim().to_string();
    let admin_username = form.admin_username.trim().to_string();

    if bind_addr.parse::<std::net::SocketAddr>().is_err() {
        let html = AdminSettingsTemplate {
            bind_addr: bind_addr.clone(),
            log_level: form.log_level.clone(),
            site_name: site_name.clone(),
            site_url: form.site_url.clone(),
            admin_username: admin_username.clone(),
            threads_per_board: form.threads_per_board,
            post_cooldown_secs: form.post_cooldown_secs,
            max_image_bytes: form.max_image_bytes,
            max_subject_chars: form.max_subject_chars,
            max_content_chars: form.max_content_chars,
            error: Some("Invalid bind address (e.g. 0.0.0.0:3000).".to_string()),
            success: None,
            css_hash: state.css_hash.clone(),
        }
        .render()
        .map_err(|e| AppError::Internal(e.into()))?;
        return Ok(Html(html).into_response());
    }

    if site_name.is_empty() {
        let html = AdminSettingsTemplate {
            bind_addr: bind_addr.clone(),
            log_level: form.log_level.clone(),
            site_name: site_name.clone(),
            site_url: form.site_url.clone(),
            admin_username: admin_username.clone(),
            threads_per_board: form.threads_per_board,
            post_cooldown_secs: form.post_cooldown_secs,
            max_image_bytes: form.max_image_bytes,
            max_subject_chars: form.max_subject_chars,
            max_content_chars: form.max_content_chars,
            error: Some("Site name cannot be empty.".to_string()),
            success: None,
            css_hash: state.css_hash.clone(),
        }
        .render()
        .map_err(|e| AppError::Internal(e.into()))?;
        return Ok(Html(html).into_response());
    }

    if admin_username.is_empty() {
        let html = AdminSettingsTemplate {
            bind_addr: bind_addr.clone(),
            log_level: form.log_level.clone(),
            site_name: site_name.clone(),
            site_url: form.site_url.clone(),
            admin_username: admin_username.clone(),
            threads_per_board: form.threads_per_board,
            post_cooldown_secs: form.post_cooldown_secs,
            max_image_bytes: form.max_image_bytes,
            max_subject_chars: form.max_subject_chars,
            max_content_chars: form.max_content_chars,
            error: Some("Admin username cannot be empty.".to_string()),
            success: None,
            css_hash: state.css_hash.clone(),
        }
        .render()
        .map_err(|e| AppError::Internal(e.into()))?;
        return Ok(Html(html).into_response());
    }

    // Detect if restart-required fields changed
    let needs_restart = {
        let cfg = state.config.read().await;
        bind_addr != cfg.server.bind_addr || form.log_level != cfg.server.log_level
    };

    // Apply all changes and write TOML
    {
        let mut cfg = state.config.write().await;
        cfg.server.bind_addr = bind_addr.clone();
        cfg.server.log_level = form.log_level.clone();
        cfg.site.name = site_name.clone();
        cfg.site.url = form.site_url.clone();
        cfg.limits.threads_per_board = form.threads_per_board;
        cfg.limits.post_cooldown_secs = form.post_cooldown_secs;
        cfg.limits.max_image_bytes = form.max_image_bytes;
        cfg.limits.max_subject_chars = form.max_subject_chars;
        cfg.limits.max_content_chars = form.max_content_chars;
        cfg.admin.username = admin_username.clone();
        if !form.admin_password.is_empty() {
            cfg.admin.password = form.admin_password.clone();
        }
        cfg.save(&state.config_path)
            .map_err(|e| AppError::Internal(e))?;
    }

    // Update rate limiter cooldown in-place (no restart needed)
    state.rate_limiter.set_cooldown(form.post_cooldown_secs);

    if needs_restart {
        state.shutdown_tx.send(true).ok();
        let html = AdminSettingsTemplate {
            bind_addr: bind_addr.clone(),
            log_level: form.log_level.clone(),
            site_name: site_name.clone(),
            site_url: form.site_url.clone(),
            admin_username: admin_username.clone(),
            threads_per_board: form.threads_per_board,
            post_cooldown_secs: form.post_cooldown_secs,
            max_image_bytes: form.max_image_bytes,
            max_subject_chars: form.max_subject_chars,
            max_content_chars: form.max_content_chars,
            error: None,
            success: Some("Settings saved. Server is restarting...".to_string()),
            css_hash: state.css_hash.clone(),
        }
        .render()
        .map_err(|e| AppError::Internal(e.into()))?;
        return Ok(Html(html).into_response());
    }

    let html = AdminSettingsTemplate {
        bind_addr: bind_addr.clone(),
        log_level: form.log_level.clone(),
        site_name: site_name.clone(),
        site_url: form.site_url.clone(),
        admin_username: admin_username.clone(),
        threads_per_board: form.threads_per_board,
        post_cooldown_secs: form.post_cooldown_secs,
        max_image_bytes: form.max_image_bytes,
        max_subject_chars: form.max_subject_chars,
        max_content_chars: form.max_content_chars,
        error: None,
        success: Some("Settings saved.".to_string()),
        css_hash: state.css_hash.clone(),
    }
    .render()
    .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Html(html).into_response())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render_login(error: Option<&str>, css_hash: &str) -> Response {
    let html = AdminLoginTemplate {
        error: error.map(|s| s.to_string()),
        css_hash: css_hash.to_string(),
    }
    .render()
    .unwrap_or_else(|_| "Login error".to_string());
    Html(html).into_response()
}
