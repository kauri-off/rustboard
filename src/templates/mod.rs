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
    pub bind_addr: String,
    pub log_level: String,
    pub site_name: String,
    pub site_url: String,
    pub admin_username: String,
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
    #[askama::filter_fn]
    pub fn ru_post_form(n: &i64, _: &dyn askama::Values) -> askama::Result<&'static str> {
        Ok(crate::i18n::ru_posts(*n))
    }

    fn html_escape_str(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#x27;"),
                _ => out.push(c),
            }
        }
        out
    }

    fn linkify_quotelinks(s: &str) -> String {
        let needle = "&gt;&gt;";
        let mut out = String::with_capacity(s.len());
        let mut rest = s;
        while let Some(pos) = rest.find(needle) {
            out.push_str(&rest[..pos]);
            let after = &rest[pos + needle.len()..];
            let digit_end = after
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after.len());
            if digit_end > 0 {
                let id = &after[..digit_end];
                out.push_str("<a href=\"#post-");
                out.push_str(id);
                out.push_str("\" class=\"quotelink\">&gt;&gt;");
                out.push_str(id);
                out.push_str("</a>");
                rest = &after[digit_end..];
            } else {
                out.push_str(needle);
                rest = after;
            }
        }
        out.push_str(rest);
        out
    }

    #[askama::filter_fn]
    pub fn format_post_content(s: &str, _: &dyn askama::Values) -> askama::Result<String> {
        let mut out = String::with_capacity(s.len() * 2);
        let mut first = true;
        for line in s.lines() {
            if !first {
                out.push('\n');
            }
            first = false;
            let escaped = html_escape_str(line);
            let processed = linkify_quotelinks(&escaped);
            if escaped.starts_with("&gt;") {
                out.push_str("<span class=\"greentext\">");
                out.push_str(&processed);
                out.push_str("</span>");
            } else {
                out.push_str(&processed);
            }
        }
        Ok(out)
    }
}
