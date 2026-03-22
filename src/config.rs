use std::path::PathBuf;

pub struct AppConfig {
    pub database_url: String,
    pub upload_dir: PathBuf,
    pub bind_addr: String,
    pub site_name: String,
    pub site_url: String,
    pub ip_salt: String,
    pub max_image_bytes: usize,
    pub max_image_width: u32,
    pub max_image_height: u32,
    pub post_cooldown_secs: u64,
    pub max_subject_chars: usize,
    pub max_content_chars: usize,
}

const DEFAULT_SALTS: &[&str] = &[
    "default-salt-change-me",
    "change-this-to-a-random-string",
    "change-me-to-a-random-secret",
];

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let ip_salt = std::env::var("IP_SALT")
            .unwrap_or_else(|_| "default-salt-change-me".to_string());

        if DEFAULT_SALTS.contains(&ip_salt.as_str()) {
            eprintln!();
            eprintln!("ERROR: IP_SALT has not been changed from the default value.");
            eprintln!();
            eprintln!("IP_SALT is used to hash poster IP addresses. Using a known default");
            eprintln!("value means anyone could reverse-engineer which posts share an IP.");
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  1. Generate a strong random secret, e.g.:");
            eprintln!("       openssl rand -hex 32");
            eprintln!("  2. Set it in your .env file:");
            eprintln!("       IP_SALT=<your-generated-secret>");
            eprintln!();
            std::process::exit(1);
        }

        Ok(Self {
            ip_salt,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:rustboard.db".to_string()),
            upload_dir: PathBuf::from(
                std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            ),
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            site_name: std::env::var("SITE_NAME").unwrap_or_else(|_| "Rustboard".to_string()),
            site_url: std::env::var("SITE_URL").unwrap_or_default(),
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
