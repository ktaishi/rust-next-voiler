use std::time::Duration;

use api::{config::Config, healthcheck};
use clap::{Parser, Subcommand};

/// API サーバ。サブコマンド無しで起動すると HTTP サーバになる。
/// 設定はすべて環境変数 (12-factor)。一覧は deploy/README.md。
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// HTTP サーバを起動する (既定)
    Serve,
    /// マイグレーションを適用して終了する (デプロイ前の 1 回限りのタスク)
    Migrate,
    /// 自プロセスの /healthz を確認する (コンテナのヘルスチェック用。0 = healthy)
    Healthcheck,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command.unwrap_or(Command::Serve) {
        Command::Serve => {
            let config = Config::from_env()?;
            app_core::telemetry::init(&config.log_format);
            api::run(config).await
        }
        Command::Migrate => {
            let config = Config::from_env()?;
            app_core::telemetry::init(&config.log_format);
            api::migrate(&config).await
        }
        Command::Healthcheck => {
            let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
            healthcheck::check(&healthcheck::local_target(&bind)?, Duration::from_secs(3)).await
        }
    }
}
