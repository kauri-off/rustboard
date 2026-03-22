use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Board {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Thread {
    pub id: i64,
    pub board_id: i64,
    pub subject: String,
    pub content: String,
    pub image_path: String,
    pub ip_hash: String,
    pub created_at: String,
    pub bump_at: String,
    pub post_count: i64,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Post {
    pub id: i64,
    pub thread_id: i64,
    pub content: String,
    pub image_path: Option<String>,
    pub ip_hash: String,
    pub created_at: String,
}
