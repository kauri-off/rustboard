use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
    #[serde(default = "default_upload_dir")]
    pub upload_dir: PathBuf,
}

#[derive(Deserialize)]
pub struct SiteConfig {
    #[serde(default = "default_site_name")]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_ip_salt")]
    pub ip_salt: String,
}

#[derive(Deserialize)]
pub struct LimitsConfig {
    #[serde(default = "default_max_image_bytes")]
    pub max_image_bytes: usize,
    #[serde(default = "default_max_image_width")]
    pub max_image_width: u32,
    #[serde(default = "default_max_image_height")]
    pub max_image_height: u32,
    #[serde(default = "default_post_cooldown_secs")]
    pub post_cooldown_secs: u64,
    #[serde(default = "default_max_subject_chars")]
    pub max_subject_chars: usize,
    #[serde(default = "default_max_content_chars")]
    pub max_content_chars: usize,
    #[serde(default = "default_threads_per_board")]
    pub threads_per_board: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: default_bind_addr(),
            log_level: default_log_level(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            upload_dir: default_upload_dir(),
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            name: default_site_name(),
            url: String::new(),
            ip_salt: default_ip_salt(),
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_image_bytes: default_max_image_bytes(),
            max_image_width: default_max_image_width(),
            max_image_height: default_max_image_height(),
            post_cooldown_secs: default_post_cooldown_secs(),
            max_subject_chars: default_max_subject_chars(),
            max_content_chars: default_max_content_chars(),
            threads_per_board: default_threads_per_board(),
        }
    }
}

fn default_bind_addr() -> String { "0.0.0.0:3000".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_database_url() -> String { "sqlite:rustboard.db".to_string() }
fn default_upload_dir() -> PathBuf { PathBuf::from("./uploads") }
fn default_site_name() -> String { "Rustboard".to_string() }
fn default_ip_salt() -> String { "change-me-to-a-random-secret".to_string() }
fn default_max_image_bytes() -> usize { 5 * 1024 * 1024 }
fn default_max_image_width() -> u32 { 10_000 }
fn default_max_image_height() -> u32 { 10_000 }
fn default_post_cooldown_secs() -> u64 { 30 }
fn default_max_subject_chars() -> usize { 200 }
fn default_max_content_chars() -> usize { 2000 }
fn default_threads_per_board() -> u32 { 100 }

fn config_path_from_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == "--config")?;
    args.get(pos + 1).cloned()
}

const DEFAULT_SALTS: &[&str] = &[
    "change-me-to-a-random-secret",
    "change-this-to-a-random-string",
    "default-salt-change-me",
];

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path_from_args().unwrap_or_else(|| "config.toml".to_string());
        let text = std::fs::read_to_string(&path)
            .map_err(|_| anyhow::anyhow!(
                "Could not read {path}. Create one in the working directory.\n\
                 See the README for a full example."
            ))?;

        let config: AppConfig = toml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Invalid config.toml: {e}"))?;

        if DEFAULT_SALTS.contains(&config.site.ip_salt.as_str()) {
            eprintln!();
            eprintln!("ERROR: site.ip_salt has not been changed from the default value.");
            eprintln!();
            eprintln!("ip_salt is used to hash poster IP addresses. Using a known default");
            eprintln!("value means anyone could reverse-engineer which posts share an IP.");
            eprintln!();
            eprintln!("To fix this:");
            eprintln!("  1. Generate a strong random secret, e.g.:");
            eprintln!("       openssl rand -hex 32");
            eprintln!("  2. Set it in config.toml:");
            eprintln!("       [site]");
            eprintln!("       ip_salt = \"<your-generated-secret>\"");
            eprintln!();
            std::process::exit(1);
        }

        Ok(config)
    }
}
