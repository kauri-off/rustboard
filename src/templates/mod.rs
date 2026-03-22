use askama::Template;

use crate::i18n::Translations;
use crate::models::{Board, Post, Thread};

#[derive(Template)]
#[template(path = "board_list.html")]
pub struct BoardListTemplate {
    pub boards: Vec<Board>,
    pub site_name: String,
    pub site_url: String,
    pub css_hash: String,
    pub t: &'static Translations,
}

#[derive(Template)]
#[template(path = "board.html")]
pub struct BoardTemplate {
    pub board: Board,
    pub boards: Vec<Board>,
    pub threads: Vec<Thread>,
    pub site_name: String,
    pub site_url: String,
    pub css_hash: String,
    pub error: Option<String>,
    pub t: &'static Translations,
}

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub board: Board,
    pub boards: Vec<Board>,
    pub thread: Thread,
    pub posts: Vec<Post>,
    pub site_name: String,
    pub site_url: String,
    pub css_hash: String,
    pub error: Option<String>,
    pub t: &'static Translations,
}

pub mod filters {
    pub fn ru_post_form(n: &i64) -> askama::Result<&'static str> {
        Ok(crate::i18n::ru_posts(*n))
    }
}
