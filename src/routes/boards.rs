use askama::Template;
use axum::{extract::State, http::HeaderMap, response::Html};
use std::sync::Arc;

use crate::{AppState, error::AppError, templates::BoardListTemplate};

pub async fn board_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let t = crate::i18n::lang_from_headers(&headers);
    let html = BoardListTemplate {
        boards: state.boards.clone(),
        site_name: state.config.site_name.clone(),
        site_url: state.config.site_url.clone(),
        css_hash: state.css_hash.clone(),
        t,
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;

    Ok(Html(html))
}
