use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::sync::OnceLock;

use crate::i18n::Translations;

static CSS_HASH: OnceLock<String> = OnceLock::new();

pub fn set_css_hash(hash: String) {
    CSS_HASH.set(hash).ok();
}

fn css_hash() -> &'static str {
    CSS_HASH.get().map(|s| s.as_str()).unwrap_or("")
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    status: u16,
    message: String,
    site_name: String,
    css_hash: &'static str,
    t: &'static Translations,
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, status_code, message) = match self {
            AppError::NotFound(msg) => (404u16, StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (400u16, StatusCode::BAD_REQUEST, msg),
            AppError::Internal(e) => (
                500u16,
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {e}"),
            ),
        };

        let tmpl = ErrorTemplate {
            status,
            message,
            site_name: "Rustboard".to_string(),
            css_hash: css_hash(),
            t: &crate::i18n::EN,
        };

        let body = tmpl
            .render()
            .unwrap_or_else(|_| "An error occurred".to_string());
        (status_code, Html(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(e.into())
    }
}

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(e: axum::extract::multipart::MultipartError) -> Self {
        AppError::BadRequest(e.to_string())
    }
}
