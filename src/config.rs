use std::path::PathBuf;

pub struct AppConfig {
    pub database_url: String,
    pub upload_dir: PathBuf,
    pub bind_addr: String,
    pub site_name: String,
    pub ip_salt: String,
    pub max_image_bytes: usize,
    pub max_image_width: u32,
    pub max_image_height: u32,
    pub post_cooldown_secs: u64,
    pub max_subject_chars: usize,
    pub max_content_chars: usize,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:rustboard.db".to_string()),
            upload_dir: PathBuf::from(
                std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            ),
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            site_name: std::env::var("SITE_NAME").unwrap_or_else(|_| "Rustboard".to_string()),
            ip_salt: std::env::var("IP_SALT")
                .unwrap_or_else(|_| "default-salt-change-me".to_string()),
            max_image_bytes: std::env::var("MAX_IMAGE_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5 * 1024 * 1024),
            max_image_width: std::env::var("MAX_IMAGE_WIDTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
            max_image_height: std::env::var("MAX_IMAGE_HEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
            post_cooldown_secs: std::env::var("POST_COOLDOWN_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_subject_chars: std::env::var("MAX_SUBJECT_CHARS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(200),
            max_content_chars: std::env::var("MAX_CONTENT_CHARS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2000),
        })
    }
}
