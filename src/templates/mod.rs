use askama::Template;

use crate::models::{Board, Post, Thread};

#[derive(Template)]
#[template(path = "board_list.html")]
pub struct BoardListTemplate {
    pub boards: Vec<Board>,
    pub site_name: String,
}

#[derive(Template)]
#[template(path = "board.html")]
pub struct BoardTemplate {
    pub board: Board,
    pub boards: Vec<Board>,
    pub threads: Vec<Thread>,
    pub site_name: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub board: Board,
    pub boards: Vec<Board>,
    pub thread: Thread,
    pub posts: Vec<Post>,
    pub site_name: String,
    pub error: Option<String>,
}
