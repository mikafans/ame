use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub production: bool,
    pub cors_origins: String,
    pub log_format: String,
    pub database_url: Option<String>,
    pub valkey_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub free: TierConfig,
    pub premium: TierConfig,
    pub public: PublicConfig,
    pub export: ExportConfig,
    pub cost: CostConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TierConfig {
    pub burst: u32,
    pub rate: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublicConfig {
    pub burst: u32,
    pub period_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportConfig {
    pub burst: u32,
    pub period_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CostConfig {
    pub read: u32,
    pub write: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchConfig {
    pub free: usize,
    pub premium: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub ratelimit: RateLimitConfig,
    pub batch: BatchConfig,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load() -> anyhow::Result<Self> {
        let path_str = std::env::var("AME_CONFIG_PATH").unwrap_or_else(|_| {
            if std::path::Path::new("ame.toml").exists() {
                "ame.toml".to_string()
            } else if std::path::Path::new("../ame.toml").exists() {
                "../ame.toml".to_string()
            } else {
                "ame.toml".to_string()
            }
        });
        let mut config = Self::load_from_file(path_str)?;

        if let Some(port) = std::env::var("AME_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
        {
            config.server.port = port;
        }
        if let Ok(prod_str) = std::env::var("AME_PRODUCTION") {
            config.server.production = prod_str == "true" || prod_str == "1";
        }
        if let Ok(cors) = std::env::var("AME_CORS_ORIGINS") {
            config.server.cors_origins = cors;
        }
        if let Ok(log) = std::env::var("AME_LOG_FORMAT") {
            config.server.log_format = log;
        }
        if let Ok(db) = std::env::var("AME_DATABASE_URL") {
            config.server.database_url = Some(db);
        }
        if let Ok(vk) = std::env::var("AME_VALKEY_URL") {
            config.server.valkey_url = Some(vk);
        }

        Ok(config)
    }
}

pub fn create_valkey_pool(config: &Config) -> anyhow::Result<deadpool_redis::Pool> {
    let valkey_url = config
        .server
        .valkey_url
        .clone()
        .or_else(|| std::env::var("AME_VALKEY_URL").ok())
        .unwrap_or_else(|| "redis://127.0.0.1:6379/".to_string());
    let cfg = deadpool_redis::Config::from_url(valkey_url);
    let pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
    Ok(pool)
}
