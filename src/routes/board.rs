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
    models::{Board, Post, Thread, ThreadWithPreviews},
    templates::BoardTemplate,
    utils::{hash_ip, process_image, real_ip},
};

pub async fn board_get(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let t = crate::i18n::lang_from_headers(&headers);
    let (site_name, site_url) = {
        let cfg = state.config.read().await;
        (cfg.site.name.clone(), cfg.site.url.clone())
    };
    let board = fetch_board(&state, &slug).await?;
    let threads = fetch_threads_with_previews(&state, board.id).await?;
    let boards = state.boards.read().await.clone();

    let html = BoardTemplate {
        board,
        boards,
        threads,
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

pub async fn board_post(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
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
        max_subject_chars,
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
            cfg.limits.max_subject_chars,
            cfg.limits.max_content_chars,
            cfg.site.ip_salt.clone(),
        )
    };
    let board = fetch_board(&state, &slug).await?;
    let client_ip = real_ip(&headers, &ConnectInfo(addr));

    if !state.rate_limiter.check_and_record(&client_ip) {
        return render_board_error(
            &state,
            board,
            "You are posting too fast. Please wait before trying again.",
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    let mut subject = String::new();
    let mut content = String::new();
    let mut image_result: Option<Result<String, String>> = None;

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("subject") => subject = field.text().await?,
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
                        image_result = Some(Err(
                            "Invalid file type. Allowed: jpg, jpeg, png, gif, webp".to_string(),
                        ));
                        continue;
                    }
                };

                if bytes.len() > max_image_bytes {
                    image_result = Some(Err(format!(
                        "Image too large. Max {} MB",
                        max_image_bytes / 1024 / 1024
                    )));
                    continue;
                }

                match process_image(&bytes, &ext, max_image_width, max_image_height) {
                    Ok(processed) => {
                        let save_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
                        let save_path = upload_dir.join(&save_name);
                        tokio::fs::write(&save_path, &processed).await?;
                        image_result = Some(Ok(format!("uploads/{}", save_name)));
                    }
                    Err(e) => {
                        image_result = Some(Err(e));
                    }
                }
            }
            _ => {}
        }
    }

    let image_path = match image_result {
        Some(Ok(path)) => path,
        Some(Err(err_msg)) => {
            return render_board_error(&state, board, &err_msg, t, &site_name, &site_url).await;
        }
        None => {
            return render_board_error(
                &state,
                board,
                "Thread requires an image",
                t,
                &site_name,
                &site_url,
            )
            .await;
        }
    };

    if subject.chars().count() > max_subject_chars {
        return render_board_error(
            &state,
            board,
            &format!("Subject too long (max {} characters)", max_subject_chars),
            t,
            &site_name,
            &site_url,
        )
        .await;
    }
    if content.chars().count() > max_content_chars {
        return render_board_error(
            &state,
            board,
            &format!("Comment too long (max {} characters)", max_content_chars),
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    if content.trim().is_empty() && subject.trim().is_empty() {
        return render_board_error(
            &state,
            board,
            "Thread must have a subject or comment",
            t,
            &site_name,
            &site_url,
        )
        .await;
    }

    let ip_hash = hash_ip(&client_ip, &ip_salt);

    let result = sqlx::query(
        "INSERT INTO threads (board_id, subject, content, image_path, ip_hash) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(board.id)
    .bind(&subject)
    .bind(&content)
    .bind(&image_path)
    .bind(&ip_hash)
    .execute(&state.pool)
    .await?;

    let thread_id = result.last_insert_rowid();
    Ok(Redirect::to(&format!("/{}/{}", slug, thread_id)).into_response())
}

async fn render_board_error(
    state: &AppState,
    board: Board,
    error_msg: &str,
    t: &'static Translations,
    site_name: &str,
    site_url: &str,
) -> Result<Response, AppError> {
    let threads = fetch_threads_with_previews(state, board.id).await?;
    let boards = state.boards.read().await.clone();
    let html = BoardTemplate {
        board,
        boards,
        threads,
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

async fn fetch_board(state: &AppState, slug: &str) -> Result<Board, AppError> {
    sqlx::query_as::<_, Board>("SELECT id, slug, name, description FROM boards WHERE slug = ?")
        .bind(slug)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Board /{slug}/ not found")))
}

async fn fetch_threads_with_previews(
    state: &AppState,
    board_id: i64,
) -> Result<Vec<ThreadWithPreviews>, AppError> {
    let threads_per_board = state.config.read().await.limits.threads_per_board;
    let threads = sqlx::query_as::<_, Thread>(
        "SELECT id, board_id, subject, content, image_path, ip_hash, created_at, bump_at, post_count
         FROM threads WHERE board_id = ? ORDER BY bump_at DESC LIMIT ?",
    )
    .bind(board_id)
    .bind(threads_per_board)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::with_capacity(threads.len());
    for thread in threads {
        let preview_posts = sqlx::query_as::<_, Post>(
            "SELECT id, thread_id, content, image_path, ip_hash, created_at
             FROM posts WHERE thread_id = ? ORDER BY id DESC LIMIT 5",
        )
        .bind(thread.id)
        .fetch_all(&state.pool)
        .await?;
        let preview_posts: Vec<Post> = preview_posts.into_iter().rev().collect();
        result.push(ThreadWithPreviews {
            thread,
            preview_posts,
        });
    }
    Ok(result)
}
