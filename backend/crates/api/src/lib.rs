//! HTTP API サーバ。
//!
//! レイヤ構成: `handlers`(HTTP) -> `app_core::domain`(ビジネスルール) -> `app_core::repository`(永続化)

pub mod config;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod healthcheck;
pub mod routes;
pub mod state;

use std::sync::Arc;

use app_core::{db, repository::MySqlTodoRepository};
use tokio::net::TcpListener;

use crate::{config::Config, state::AppState};

/// マイグレーションだけを適用する (`api migrate`)。
pub async fn migrate(config: &Config) -> anyhow::Result<()> {
    let pool = db::connect(&config.db, 1).await?;
    db::MIGRATOR.run(&pool).await?;
    tracing::info!("migrations applied");
    Ok(())
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let pool = db::connect(&config.db, config.db_max_connections).await?;

    if config.run_migrations {
        db::MIGRATOR.run(&pool).await?;
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
