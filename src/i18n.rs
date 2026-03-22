pub struct Translations {
    pub lang_code: &'static str,
    pub powered_by: &'static str,
    // board_list
    pub boards: &'static str,
    pub board_col: &'static str,
    pub name_col: &'static str,
    pub description_col: &'static str,
    // board
    pub new_thread: &'static str,
    pub subject: &'static str,
    pub comment: &'static str,
    pub image: &'static str,
    pub post_btn: &'static str,
    pub no_threads: &'static str,
    // thread
    pub return_link: &'static str,
    pub reply_heading: &'static str,
    pub post_reply_btn: &'static str,
    pub optional: &'static str,
    // error
    pub back_to_boards: &'static str,
}

pub static EN: Translations = Translations {
    lang_code: "en",
    powered_by: "Powered by",
    boards: "Boards",
    board_col: "Board",
    name_col: "Name",
    description_col: "Description",
    new_thread: "New Thread",
    subject: "Subject",
    comment: "Comment",
    image: "Image",
    post_btn: "Post",
    no_threads: "No threads yet. Be the first to post!",
    return_link: "[Return]",
    reply_heading: "Reply",
    post_reply_btn: "Post Reply",
    optional: "Optional",
    back_to_boards: "← Back to boards",
};

pub static RU: Translations = Translations {
    lang_code: "ru",
    powered_by: "Работает на",
    boards: "Доски",
    board_col: "Доска",
    name_col: "Название",
    description_col: "Описание",
    new_thread: "Новая тема",
    subject: "Тема",
    comment: "Комментарий",
    image: "Изображение",
    post_btn: "Отправить",
    no_threads: "Тем ещё нет. Будьте первым!",
    return_link: "[Назад]",
    reply_heading: "Ответить",
    post_reply_btn: "Отправить ответ",
    optional: "Необязательно",
    back_to_boards: "← К доскам",
};

/// Cookie -> Accept-Language -> EN
pub fn lang_from_headers(headers: &axum::http::HeaderMap) -> &'static Translations {
    let cookie_lang = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').find_map(|p| p.trim().strip_prefix("lang=")));
    let accept_lang = headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok());
    match cookie_lang {
        Some("ru") => &RU,
        Some(_) => &EN,
        None => match accept_lang {
            Some(h) if h.split(',').next().unwrap_or("").trim().starts_with("ru") => &RU,
            _ => &EN,
        },
    }
}

/// Russian post count pluralization
pub fn ru_posts(n: i64) -> &'static str {
    let abs = n.unsigned_abs() % 100;
    let last = abs % 10;
    if abs >= 11 && abs <= 19 {
        "сообщений"
    } else if last == 1 {
        "сообщение"
    } else if last >= 2 && last <= 4 {
        "сообщения"
    } else {
        "сообщений"
    }
}
