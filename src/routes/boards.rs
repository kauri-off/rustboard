use askama::Template;
use axum::{extract::State, http::HeaderMap, response::Html};
use std::sync::Arc;

use crate::{AppState, error::AppError, templates::BoardListTemplate};

pub async fn board_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Html<String>, AppError> {
    let t = crate::i18n::lang_from_headers(&headers);
    let (site_name, site_url) = {
        let cfg = state.config.read().await;
        (cfg.site.name.clone(), cfg.site.url.clone())
    };
    let boards = state.boards.read().await.clone();
    let html = BoardListTemplate {
        boards,
        site_name,
        site_url,
        css_hash: state.css_hash.clone(),
        t,
    }
    .render()
    .map_err(|e: askama::Error| AppError::Internal(e.into()))?;

    Ok(Html(html))
}
