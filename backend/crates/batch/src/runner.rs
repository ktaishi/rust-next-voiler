use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{pool::PoolConnection, MySql, MySqlPool};
use uuid::Uuid;

/// バッチジョブ。構造体自身が実行パラメータを持ち、`params()` で JSON 化して履歴に保存する
/// (再開時は `jobs::build` が JSON から復元する)。
#[async_trait]
pub trait Job: Send + Sync {
    fn name(&self) -> &'static str;
    fn params(&self) -> serde_json::Value;
    /// チャンクごとに `ctx.commit_progress` を呼ぶこと。`ctx.checkpoint()` があればそこから再開する。
    async fn run(&self, ctx: &mut JobContext) -> anyhow::Result<()>;
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("job '{0}' is already running (lock not acquired)")]
    AlreadyRunning(String),
    #[error("run {0} not found")]
    RunNotFound(Uuid),
    #[error("run {0} already succeeded; nothing to resume")]
    AlreadySucceeded(Uuid),
    #[error("job failed (run_id={run_id}): {source:#}")]
    Failed { run_id: Uuid, source: anyhow::Error },
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    pub run_id: Uuid,
    pub job_name: String,
    pub processed: u64,
    pub elapsed_ms: u128,
}

/// 実行中のジョブに渡すコンテキスト。
pub struct JobContext {
    pool: MySqlPool,
    run_id: Uuid,
    processed: u64,
    checkpoint: Option<String>,
    dry_run: bool,
    last_log: Instant,
}

impl JobContext {
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
    pub fn run_id(&self) -> Uuid {
        self.run_id
    }
    pub fn processed(&self) -> u64 {
        self.processed
    }
    pub fn checkpoint(&self) -> Option<&str> {
        self.checkpoint.as_deref()
    }
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// 進捗とチェックポイントを永続化する。チャンクのコミット後に呼ぶ。
    pub async fn commit_progress(
        &mut self,
        delta: u64,
        checkpoint: Option<String>,
    ) -> Result<(), sqlx::Error> {
        self.processed += delta;
        if checkpoint.is_some() {
            self.checkpoint = checkpoint;
        }
        if !self.dry_run {
            sqlx::query(
                "UPDATE batch_jobs SET processed = ?, checkpoint = COALESCE(?, checkpoint) WHERE id = ?",
            )
            .bind(self.processed)
            .bind(self.checkpoint.as_deref())
            .bind(self.run_id)
            .execute(&self.pool)
            .await?;
        }
        if self.last_log.elapsed().as_secs() >= 5 {
            tracing::info!(run_id = %self.run_id, processed = self.processed, "progress");
            self.last_log = Instant::now();
        }
        Ok(())
    }
}

/// 実行履歴の 1 行。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobRun {
    pub id: Uuid,
    pub job_name: String,
    pub status: String,
    pub params: serde_json::Value,
    pub processed: u64,
    pub checkpoint: Option<String>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

pub struct JobRunner {
    pool: MySqlPool,
}

impl JobRunner {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// 新規実行。
    pub async fn run(&self, job: &dyn Job, dry_run: bool) -> Result<RunSummary, RunError> {
        let lock = self.lock(job.name()).await?;
        let run_id = Uuid::now_v7();
        if !dry_run {
            sqlx::query(
                "INSERT INTO batch_jobs (id, job_name, status, params) VALUES (?, ?, 'running', ?)",
            )
            .bind(run_id)
            .bind(job.name())
            .bind(job.params())
            .execute(&self.pool)
            .await?;
        }
        self.execute(job, lock, run_id, 0, None, dry_run).await
    }

    /// 失敗 (または異常終了で running のまま) の実行を、チェックポイントから再開する。
    pub async fn resume(&self, run_id: Uuid) -> Result<RunSummary, RunError> {
        let run = self
            .find(run_id)
            .await?
            .ok_or(RunError::RunNotFound(run_id))?;
        if run.status == "succeeded" {
            return Err(RunError::AlreadySucceeded(run_id));
        }
        let job = crate::jobs::build(&run.job_name, run.params.clone())?;
        // ロックが取れた = 同名ジョブは誰も実行していない = running のままの行は異常終了の残骸
        let lock = self.lock(job.name()).await?;
        sqlx::query(
            "UPDATE batch_jobs SET status = 'running', error = NULL, finished_at = NULL WHERE id = ?",
        )
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        tracing::info!(%run_id, processed = run.processed, checkpoint = ?run.checkpoint, "resuming");
        self.execute(
            job.as_ref(),
            lock,
            run_id,
            run.processed,
            run.checkpoint,
            false,
        )
        .await
    }

    pub async fn find(&self, run_id: Uuid) -> Result<Option<JobRun>, sqlx::Error> {
        sqlx::query_as(&format!("{SELECT_RUN} WHERE id = ?"))
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn recent(&self, limit: u32) -> Result<Vec<JobRun>, sqlx::Error> {
        sqlx::query_as(&format!("{SELECT_RUN} ORDER BY id DESC LIMIT ?"))
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    async fn execute(
        &self,
        job: &dyn Job,
        lock: JobLock,
        run_id: Uuid,
        processed: u64,
        checkpoint: Option<String>,
        dry_run: bool,
    ) -> Result<RunSummary, RunError> {
        let started = Instant::now();
        let mut ctx = JobContext {
            pool: self.pool.clone(),
            run_id,
            processed,
            checkpoint,
            dry_run,
            last_log: Instant::now(),
        };
        tracing::info!(job = job.name(), %run_id, dry_run, params = %job.params(), "job started");
        let result = job.run(&mut ctx).await;

        if !dry_run {
            let (status, error) = match &result {
                Ok(()) => ("succeeded", None),
                Err(e) => ("failed", Some(format!("{e:#}"))),
            };
            sqlx::query(
                "UPDATE batch_jobs SET status = ?, processed = ?, error = ?, finished_at = NOW(6) WHERE id = ?",
            )
            .bind(status)
            .bind(ctx.processed)
            .bind(error)
            .bind(run_id)
            .execute(&self.pool)
            .await?;
        }
        lock.release().await;

        let elapsed_ms = started.elapsed().as_millis();
        match result {
            Ok(()) => {
                tracing::info!(job = job.name(), %run_id, processed = ctx.processed, elapsed_ms, "job succeeded");
                Ok(RunSummary {
                    run_id,
                    job_name: job.name().to_string(),
                    processed: ctx.processed,
                    elapsed_ms,
                })
            }
            Err(source) => {
                tracing::error!(job = job.name(), %run_id, error = %format!("{source:#}"), "job failed");
                Err(RunError::Failed { run_id, source })
            }
        }
    }

    /// `GET_LOCK` による多重起動防止。ロックは専用コネクションに紐づき、
    /// プロセスが落ちて接続が切れれば MySQL が自動的に解放する。
    /// ロック名には DB 名を含め、同一サーバ上の別 DB (テスト用 DB 等) と干渉しないようにする。
    async fn lock(&self, job_name: &str) -> Result<JobLock, RunError> {
        let mut conn = self.pool.acquire().await?;
        let name = lock_name(&mut conn, job_name).await?;
        let got: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
            .bind(&name)
            .fetch_one(&mut *conn)
            .await?;
        if got != Some(1) {
            return Err(RunError::AlreadyRunning(job_name.to_string()));
        }
        Ok(JobLock { conn, name })
    }
}

/// `GET_LOCK` のロック名 (MySQL の上限 64 文字に収まる)。
/// `batch:<ジョブ名先頭16文字>:<SHA2(DB名:ジョブ名) の先頭32桁>`
pub async fn lock_name(
    conn: &mut sqlx::MySqlConnection,
    job_name: &str,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT CONCAT('batch:', LEFT(?, 16), ':', LEFT(SHA2(CONCAT(DATABASE(), ':', ?), 256), 32))",
    )
    .bind(job_name)
    .bind(job_name)
    .fetch_one(conn)
    .await
}

const SELECT_RUN: &str = "SELECT id, job_name, CAST(status AS CHAR) AS status, params, processed, checkpoint, error, started_at, finished_at FROM batch_jobs";

struct JobLock {
    conn: PoolConnection<MySql>,
    name: String,
}

impl JobLock {
    async fn release(mut self) {
        if let Err(e) = sqlx::query("SELECT RELEASE_LOCK(?)")
            .bind(&self.name)
            .execute(&mut *self.conn)
            .await
        {
            // 解放に失敗しても接続を閉じればロックは消える
            tracing::warn!(error = %e, "RELEASE_LOCK failed; closing connection");
            let _ = self.conn.detach();
        }
    }
}
