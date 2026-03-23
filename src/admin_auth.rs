use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::http::HeaderMap;
use uuid::Uuid;

use crate::AppState;

const SESSION_TTL: Duration = Duration::from_secs(24 * 3600);

pub async fn create_session(state: &Arc<AppState>) -> String {
    let token = Uuid::new_v4().to_string();
    let expiry = Instant::now() + SESSION_TTL;
    state
        .admin_sessions
        .lock()
        .await
        .insert(token.clone(), expiry);
    token
}

pub async fn destroy_session(state: &Arc<AppState>, token: &str) {
    state.admin_sessions.lock().await.remove(token);
}

pub async fn is_valid_session(state: &Arc<AppState>, token: &str) -> bool {
    let mut map = state.admin_sessions.lock().await;
    match map.get(token) {
        Some(&expiry) if Instant::now() < expiry => true,
        Some(_) => {
            map.remove(token);
            false
        }
        None => false,
    }
}

pub fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get("cookie")?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("admin_session=") {
            return Some(val.to_string());
        }
    }
    None
}
