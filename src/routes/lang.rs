use axum::{
    Form,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SetLangForm {
    pub lang: String,
}

pub async fn set_lang(headers: HeaderMap, Form(form): Form<SetLangForm>) -> Response {
    let lang = if form.lang == "ru" { "ru" } else { "en" };
    let redirect_to = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("/boards")
        .to_string();
    let cookie = format!("lang={}; Path=/; SameSite=Lax; Max-Age=31536000", lang);
    (
        StatusCode::SEE_OTHER,
        [
            ("set-cookie", cookie.as_str()),
            ("location", redirect_to.as_str()),
        ],
    )
        .into_response()
}
