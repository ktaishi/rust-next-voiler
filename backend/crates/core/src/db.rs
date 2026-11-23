use std::time::Duration;

use sqlx::{migrate::Migrator, mysql::MySqlPoolOptions, MySqlPool};

/// `backend/migrations` を埋め込んだマイグレータ。API とバッチで共有する。
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub async fn connect(database_url: &str, max_connections: u32) -> anyhow::Result<MySqlPool> {
    Ok(MySqlPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?)
}
