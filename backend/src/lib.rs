//! API サーバのエントリポイント。
//!
//! レイヤ構成: `handlers`(HTTP) -> `domain`(ビジネスルール) -> `repository`(永続化)

pub mod config;
pub mod domain;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod repository;
pub mod routes;
pub mod state;
pub mod telemetry;

use std::sync::Arc;

use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;

use crate::{config::Config, repository::MySqlTodoRepository, state::AppState};

pub async fn run(config: Config) -> anyhow::Result<()> {
    let pool = MySqlPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    if config.run_migrations {
        sqlx::migrate!("./migrations").run(&pool).await?;
        tracing::info!("migrations applied");
    }

    let state = AppState {
        todos: Arc::new(MySqlTodoRepository::new(pool.clone())),
        db: Some(pool),
    };
    let app = routes::router(state, &config.cors_allowed_origins);

    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutdown signal received");
}
