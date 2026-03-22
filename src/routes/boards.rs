use askama::Template;
use axum::{extract::State, response::Html};
use std::sync::Arc;

use crate::{AppState, error::AppError, templates::BoardListTemplate};

pub async fn board_list(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let html = BoardListTemplate {
        boards: state.boards.clone(),
        site_name: state.config.site_name.clone(),
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;

    Ok(Html(html))
}
