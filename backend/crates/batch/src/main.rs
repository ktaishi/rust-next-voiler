use batch::{
    jobs::{ArchiveCompletedJob, SeedJob},
    runner::{Job, JobRunner},
};
use clap::{Parser, Subcommand};
use uuid::Uuid;

/// バッチ処理 CLI。終了コード 0 = 成功, 1 = 失敗 (多重起動・ジョブ失敗を含む)。
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,
    /// コネクションプールの上限。seed の --concurrency + 2 以上にする
    #[arg(long, env = "DB_MAX_CONNECTIONS", default_value_t = 10)]
    db_max_connections: u32,
    #[arg(long, env = "LOG_FORMAT", default_value = "pretty")]
    log_format: String,
    /// 対象を数えるだけで変更しない (履歴にも残さない)
    #[arg(long, global = true)]
    dry_run: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// テストデータをバルク投入する
    Seed(SeedJob),
    /// 完了済みの古い Todo を todos_archive へ移動する
    ArchiveCompleted {
        /// updated_at がこの日数より前のものを対象にする
        #[arg(long)]
        older_than_days: u32,
        #[arg(long, default_value_t = 1_000)]
        chunk_size: u32,
    },
    /// 失敗した実行をチェックポイントから再開する
    Resume { run_id: Uuid },
    /// 直近の実行履歴を表示する
    Status {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    app_core::telemetry::init(&cli.log_format);
    let pool = app_core::db::connect(&cli.database_url, cli.db_max_connections).await?;
    let runner = JobRunner::new(pool);

    let job: Box<dyn Job> = match cli.command {
        Command::Seed(j) => Box::new(j),
        Command::ArchiveCompleted {
            older_than_days,
            chunk_size,
        } => Box::new(ArchiveCompletedJob::new(older_than_days, chunk_size)),
        Command::Resume { run_id } => {
            let s = runner.resume(run_id).await?;
            println!(
                "{}",
                serde_json::json!({ "runId": s.run_id, "processed": s.processed, "elapsedMs": s.elapsed_ms })
            );
            return Ok(());
        }
        Command::Status { limit } => {
            println!(
                "{:<36}  {:<18}  {:<9}  {:>10}  {:<26}  ERROR",
                "RUN_ID", "JOB", "STATUS", "PROCESSED", "STARTED_AT"
            );
            for r in runner.recent(limit).await? {
                println!(
                    "{:<36}  {:<18}  {:<9}  {:>10}  {:<26}  {}",
                    r.id,
                    r.job_name,
                    r.status,
                    r.processed,
                    r.started_at.to_rfc3339(),
                    r.error.unwrap_or_default()
                );
            }
            return Ok(());
        }
    };

    let s = runner.run(job.as_ref(), cli.dry_run).await?;
    println!(
        "{}",
        serde_json::json!({ "runId": s.run_id, "job": s.job_name, "processed": s.processed, "elapsedMs": s.elapsed_ms })
    );
    Ok(())
}
