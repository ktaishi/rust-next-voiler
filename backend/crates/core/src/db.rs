//! DB 接続設定とプール。API / バッチ / マイグレーションで共通。
//!
//! 接続先は環境変数で与える (どちらか一方):
//! - `DATABASE_URL` … `mysql://user:pass@host:3306/db` (ローカル開発・CI 向け)
//! - `DB_HOST` / `DB_PORT` / `DB_USER` / `DB_PASSWORD` / `DB_NAME` … 個別指定 (本番向け)。
//!   パスワードをシークレットストア (AWS Secrets Manager, k8s Secret 等) から単独で注入でき、
//!   記号を含むパスワードを URL エンコードする必要もない。
//!
//! 共通オプション: `DB_SSL_MODE` (disabled|preferred|required|verify_ca|verify_identity), `DB_SSL_CA` (CA 証明書パス)

use std::{str::FromStr, time::Duration};

use anyhow::Context;
use sqlx::{
    migrate::Migrator,
    mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode},
    MySqlPool,
};

/// `backend/migrations` を埋め込んだマイグレータ。API とバッチで共有する。
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// 接続設定。パスワードを含むため `Debug` は実装しない。
#[derive(Clone)]
pub struct DbConfig {
    options: MySqlConnectOptions,
}

impl DbConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
    }

    /// テスト可能にするため、環境変数の読み取りを関数で受け取る。
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let mut options = match get("DATABASE_URL") {
            Some(url) => MySqlConnectOptions::from_str(&url).context("invalid DATABASE_URL")?,
            None => {
                let host = get("DB_HOST").context("DATABASE_URL or DB_HOST is required")?;
                let port = match get("DB_PORT") {
                    Some(p) => p.parse().context("DB_PORT must be a number")?,
                    None => 3306,
                };
                MySqlConnectOptions::new()
                    .host(&host)
                    .port(port)
                    .username(&get("DB_USER").context("DB_USER is required")?)
                    .password(&get("DB_PASSWORD").unwrap_or_default())
                    .database(&get("DB_NAME").context("DB_NAME is required")?)
            }
        };
        if let Some(mode) = get("DB_SSL_MODE") {
            let mode = MySqlSslMode::from_str(&mode)
                .map_err(|e| anyhow::anyhow!("invalid DB_SSL_MODE: {e}"))?;
            options = options.ssl_mode(mode);
        }
        if let Some(ca) = get("DB_SSL_CA") {
            options = options.ssl_ca(ca);
        }
        Ok(Self { options })
    }

    pub fn options(&self) -> &MySqlConnectOptions {
        &self.options
    }
}

pub async fn connect(config: &DbConfig, max_connections: u32) -> anyhow::Result<MySqlPool> {
    Ok(MySqlPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(config.options.clone())
        .await?)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn cfg(pairs: &[(&str, &str)]) -> anyhow::Result<DbConfig> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        DbConfig::from_lookup(|k| m.get(k).cloned())
    }

    #[test]
    fn database_url_takes_precedence() {
        let c = cfg(&[
            ("DATABASE_URL", "mysql://u:p@db1:3307/app"),
            ("DB_HOST", "ignored"),
        ])
        .unwrap();
        assert_eq!(c.options().get_host(), "db1");
        assert_eq!(c.options().get_port(), 3307);
        assert_eq!(c.options().get_database(), Some("app"));
    }

    #[test]
    fn discrete_variables_allow_symbols_in_password() {
        let c = cfg(&[
            ("DB_HOST", "db.internal"),
            ("DB_USER", "app"),
            ("DB_PASSWORD", "p@ss:/#?word"),
            ("DB_NAME", "app"),
            ("DB_SSL_MODE", "required"),
        ])
        .unwrap();
        assert_eq!(c.options().get_host(), "db.internal");
        assert_eq!(c.options().get_port(), 3306);
        assert_eq!(c.options().get_username(), "app");
    }

    #[test]
    fn missing_or_invalid_settings_are_errors() {
        assert!(cfg(&[]).is_err());
        assert!(cfg(&[("DB_HOST", "h"), ("DB_NAME", "app")]).is_err()); // DB_USER なし
        assert!(cfg(&[
            ("DATABASE_URL", "mysql://u@h/app"),
            ("DB_SSL_MODE", "bogus")
        ])
        .is_err());
        assert!(cfg(&[
            ("DB_HOST", "h"),
            ("DB_USER", "u"),
            ("DB_NAME", "d"),
            ("DB_PORT", "x")
        ])
        .is_err());
    }
}
