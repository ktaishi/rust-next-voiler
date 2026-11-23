use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub run_migrations: bool,
    pub cors_allowed_origins: Vec<String>,
    pub log_format: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            db_max_connections: env_or("DB_MAX_CONNECTIONS", "20")
                .parse()
                .context("DB_MAX_CONNECTIONS must be a number")?,
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
