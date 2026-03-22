use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    status: u16,
    message: String,
    site_name: String,
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
