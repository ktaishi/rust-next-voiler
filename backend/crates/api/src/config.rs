use anyhow::Context;
use app_core::db::DbConfig;

/// API の設定 (すべて環境変数)。DB の接続設定はパスワードを含むため `Debug` を実装しない。
#[derive(Clone)]
pub struct Config {
    pub bind_addr: String,
    pub db: DbConfig,
    pub db_max_connections: u32,
    pub run_migrations: bool,
    pub cors_allowed_origins: Vec<String>,
    pub log_format: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            db: DbConfig::from_env()?,
            db_max_connections: env_or("DB_MAX_CONNECTIONS", "20")
                .parse()
                .context("DB_MAX_CONNECTIONS must be a number")?,
            // ローカル開発の利便のため既定 true。本番は false にして `api migrate` を別タスクで実行する
            run_migrations: env_or("RUN_MIGRATIONS", "true") == "true",
            cors_allowed_origins: env_or("CORS_ALLOWED_ORIGINS", "http://localhost:3000")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            log_format: env_or("LOG_FORMAT", "pretty"),
        })
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
