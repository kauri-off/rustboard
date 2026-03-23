use askama::Template;

use crate::i18n::Translations;
use crate::models::{Board, Post, PostWithBoard, Thread, ThreadWithBoard, ThreadWithPreviews};

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
    pub threads: Vec<ThreadWithPreviews>,
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

// ── Admin templates ───────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct AdminLoginTemplate {
    pub error: Option<String>,
    pub css_hash: String,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
pub struct AdminDashboardTemplate {
    pub board_count: i64,
    pub thread_count: i64,
    pub post_count: i64,
    pub css_hash: String,
}

#[derive(Template)]
#[template(path = "admin/boards.html")]
pub struct AdminBoardsTemplate {
    pub boards: Vec<Board>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub css_hash: String,
}

#[derive(Template)]
#[template(path = "admin/posts.html")]
pub struct AdminPostsTemplate {
    pub threads: Vec<ThreadWithBoard>,
    pub posts: Vec<PostWithBoard>,
    pub css_hash: String,
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct AdminSettingsTemplate {
    pub site_name: String,
    pub threads_per_board: u32,
    pub post_cooldown_secs: u64,
    pub max_image_bytes: usize,
    pub max_subject_chars: usize,
    pub max_content_chars: usize,
    pub error: Option<String>,
    pub success: Option<String>,
    pub css_hash: String,
}

pub mod filters {
    pub fn ru_post_form(n: &i64) -> askama::Result<&'static str> {
        Ok(crate::i18n::ru_posts(*n))
    }
}
