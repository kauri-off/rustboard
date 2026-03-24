use askama::Template;
use axum::{
    extract::{ConnectInfo, Multipart, Path, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
};
use std::{net::SocketAddr, sync::Arc};

use crate::{
    AppState,
    error::AppError,
    i18n::Translations,
    models::{Board, Post, Thread},
    templates::ThreadTemplate,
    utils::{hash_ip, process_image, real_ip},
};

pub async fn thread_get(
    State(state): State<Arc<AppState>>,
    Path((slug, thread_id)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let t = crate::i18n::lang_from_headers(&headers);
    let (site_name, site_url) = {
        let cfg = state.config.read().await;
        (cfg.site.name.clone(), cfg.site.url.clone())
    };
    let (board, thread, posts) = fetch_thread_data(&state, &slug, thread_id).await?;
    let boards = state.boards.read().await.clone();

    let html = ThreadTemplate {
        board,
        boards,
        thread,
        posts,
        site_name,
        site_url,
        css_hash: state.css_hash.clone(),
        error: None,
        t,
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;

    Ok(Html(html))
}

pub async fn thread_post(
    State(state): State<Arc<AppState>>,
    Path((slug, thread_id)): Path<(String, i64)>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let t = crate::i18n::lang_from_headers(&headers);
    let (
        site_name,
        site_url,
        max_image_bytes,
        max_image_width,
        max_image_height,
        upload_dir,
        max_content_chars,
        ip_salt,
    ) = {
        let cfg = state.config.read().await;
        (
            cfg.site.name.clone(),
            cfg.site.url.clone(),
            cfg.limits.max_image_bytes,
            cfg.limits.max_image_width,
            cfg.limits.max_image_height,
            cfg.database.upload_dir.clone(),
            cfg.limits.max_content_chars,
            cfg.site.ip_salt.clone(),
        )
    };
    let (board, thread, posts) = fetch_thread_data(&state, &slug, thread_id).await?;
    let client_ip = real_ip(&headers, &ConnectInfo(addr));

    if !state.rate_limiter.check_and_record(&client_ip) {
        return render_thread_error(
            &state,
            board,
            thread,
            posts,
            "You are posting too fast. Please wait before trying again.",
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    let mut content = String::new();
    let mut image_path: Option<String> = None;
    let mut form_error: Option<String> = None;

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("content") => content = field.text().await?,
            Some("image") => {
                let filename = field.file_name().unwrap_or("").to_string();
                let bytes = field.bytes().await?;

                if bytes.is_empty() {
                    continue;
                }

                let ext = std::path::Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .filter(|e| ["jpg", "jpeg", "png", "gif", "webp"].contains(&e.as_str()));

                let ext = match ext {
                    Some(e) => e,
                    None => {
                        form_error = Some(
                            "Invalid file type. Allowed: jpg, jpeg, png, gif, webp".to_string(),
                        );
                        continue;
                    }
                };

                if bytes.len() > max_image_bytes {
                    form_error = Some(format!(
                        "Image too large. Max {} MB",
                        max_image_bytes / 1024 / 1024
                    ));
                    continue;
                }

                match process_image(&bytes, &ext, max_image_width, max_image_height) {
                    Ok(processed) => {
                        let save_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
                        let save_path = upload_dir.join(&save_name);
                        tokio::fs::write(&save_path, &processed).await?;
                        image_path = Some(format!("uploads/{}", save_name));
                    }
                    Err(e) => {
                        form_error = Some(e);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(err) = form_error {
        return render_thread_error(&state, board, thread, posts, &err, t, &site_name, &site_url)
            .await;
    }

    if content.chars().count() > max_content_chars {
        return render_thread_error(
            &state,
            board,
            thread,
            posts,
            &format!("Comment too long (max {} characters)", max_content_chars),
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    if content.trim().is_empty() && image_path.is_none() {
        return render_thread_error(
            &state,
            board,
            thread,
            posts,
            "Reply must have content or an image",
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    let ip_hash = hash_ip(&client_ip, &ip_salt);

    let result = sqlx::query(
        "INSERT INTO posts (thread_id, content, image_path, ip_hash) VALUES (?, ?, ?, ?)",
    )
    .bind(thread_id)
    .bind(&content)
    .bind(&image_path)
    .bind(&ip_hash)
    .execute(&state.pool)
    .await?;

    let post_id = result.last_insert_rowid();

    sqlx::query(
        "UPDATE threads SET bump_at = datetime('now'), post_count = post_count + 1 WHERE id = ?",
    )
    .bind(thread_id)
    .execute(&state.pool)
    .await?;

    Ok(Redirect::to(&format!("/{}/{}#post-{}", slug, thread_id, post_id)).into_response())
}

async fn render_thread_error(
    state: &AppState,
    board: Board,
    thread: Thread,
    posts: Vec<Post>,
    error_msg: &str,
    t: &'static Translations,
    site_name: &str,
    site_url: &str,
) -> Result<Response, AppError> {
    let boards = state.boards.read().await.clone();
    let html = ThreadTemplate {
        board,
        boards,
        thread,
        posts,
        site_name: site_name.to_string(),
        site_url: site_url.to_string(),
        css_hash: state.css_hash.clone(),
        error: Some(error_msg.to_string()),
        t,
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;
    Ok(Html(html).into_response())
}

async fn fetch_thread_data(
    state: &AppState,
    slug: &str,
    thread_id: i64,
) -> Result<(Board, Thread, Vec<Post>), AppError> {
    let board =
        sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Board /{slug}/ not found")))?;

    let thread = sqlx::query_as::<_, Thread>(
        "SELECT id, board_id, subject, content, image_path, ip_hash, created_at, bump_at, post_count
         FROM threads WHERE id = ? AND board_id = ?",
    )
    .bind(thread_id)
    .bind(board.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Thread not found".to_string()))?;

    let posts = sqlx::query_as::<_, Post>(
        "SELECT id, thread_id, content, image_path, ip_hash, created_at
         FROM posts WHERE thread_id = ? ORDER BY id ASC",
    )
    .bind(thread_id)
    .fetch_all(&state.pool)
    .await?;

    Ok((board, thread, posts))
}
