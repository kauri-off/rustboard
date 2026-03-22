use askama::Template;
use axum::{
    extract::{ConnectInfo, Multipart, Path, State},
    response::{Html, IntoResponse, Redirect, Response},
};
use std::{net::SocketAddr, sync::Arc};

use crate::{
    AppState,
    error::AppError,
    models::{Board, Thread},
    templates::BoardTemplate,
    utils::{hash_ip, process_image},
};

pub async fn board_get(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Html<String>, AppError> {
    let board = fetch_board(&state, &slug).await?;
    let threads = fetch_threads(&state, board.id).await?;

    let html = BoardTemplate {
        board,
        boards: state.boards.clone(),
        threads,
        site_name: state.config.site_name.clone(),
        error: None,
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;

    Ok(Html(html))
}

pub async fn board_post(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let board = fetch_board(&state, &slug).await?;

    if !state.rate_limiter.check_and_record(&addr.ip().to_string()) {
        return render_board_error(
            &state,
            board,
            "You are posting too fast. Please wait before trying again.",
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
                            "Invalid file type. Allowed: jpg, png, gif, webp".to_string(),
                        ));
                        continue;
                    }
                };

                if bytes.len() > state.config.max_image_bytes {
                    image_result = Some(Err(format!(
                        "Image too large. Max {} MB",
                        state.config.max_image_bytes / 1024 / 1024
                    )));
                    continue;
                }

                match process_image(
                    &bytes,
                    &ext,
                    state.config.max_image_width,
                    state.config.max_image_height,
                ) {
                    Ok(processed) => {
                        let save_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
                        let save_path = state.config.upload_dir.join(&save_name);
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
            return render_board_error(&state, board, &err_msg).await;
        }
        None => {
            return render_board_error(&state, board, "Thread requires an image").await;
        }
    };

    if subject.chars().count() > state.config.max_subject_chars {
        return render_board_error(
            &state,
            board,
            &format!("Subject too long (max {} characters)", state.config.max_subject_chars),
        )
        .await;
    }
    if content.chars().count() > state.config.max_content_chars {
        return render_board_error(
            &state,
            board,
            &format!("Comment too long (max {} characters)", state.config.max_content_chars),
        )
        .await;
    }

    if content.trim().is_empty() && subject.trim().is_empty() {
        return render_board_error(&state, board, "Thread must have a subject or comment").await;
    }

    let ip_hash = hash_ip(&addr.ip().to_string(), &state.config.ip_salt);

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
) -> Result<Response, AppError> {
    let threads = fetch_threads(state, board.id).await?;
    let html = BoardTemplate {
        board,
        boards: state.boards.clone(),
        threads,
        site_name: state.config.site_name.clone(),
        error: Some(error_msg.to_string()),
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

async fn fetch_threads(state: &AppState, board_id: i64) -> Result<Vec<Thread>, AppError> {
    sqlx::query_as::<_, Thread>(
        "SELECT id, board_id, subject, content, image_path, ip_hash, created_at, bump_at, post_count
         FROM threads WHERE board_id = ? ORDER BY bump_at DESC LIMIT 100",
    )
    .bind(board_id)
    .fetch_all(&state.pool)
    .await
    .map_err(Into::into)
}
