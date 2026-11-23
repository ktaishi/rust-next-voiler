use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use futures::{stream, StreamExt, TryStreamExt};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::{NoContext, Timestamp, Uuid};

use crate::{
    chunk::rows_per_statement,
    retry::{retry, RetryPolicy},
    runner::{Job, JobContext},
};

/// 性能試験用のテストデータを `todos` にバルク INSERT する。
///
/// チェックポイント = 投入済み件数。並列 INSERT の結果は順序どおりに受け取り (`buffered`)、
/// 連続して完了した件数だけを記録する。失敗時、並列実行中だった後続バッチ (最大 concurrency-1 個)
/// は投入済みでもカウントされないため、再開すると最大でその分だけ多く投入される (at-least-once)。
#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct SeedJob {
    /// 投入件数
    #[arg(long)]
    pub count: u64,
    /// 1 文あたりの行数 (プレースホルダ上限で自動的に丸める)
    #[arg(long, default_value_t = 1_000)]
    pub chunk_size: usize,
    /// 同時に実行する INSERT 文の数
    #[arg(long, default_value_t = 4)]
    pub concurrency: usize,
    /// 完了済みにする割合 (0.0〜1.0)
    #[arg(long, default_value_t = 0.5)]
    pub completed_ratio: f64,
    /// updated_at を現在から最大何日前まで散らすか
    #[arg(long, default_value_t = 365)]
    pub max_age_days: u32,
}

const COLUMNS: usize = 5;

struct Row {
    id: Uuid,
    title: String,
    completed: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SeedJob {
    pub const NAME: &'static str = "seed";

    fn generate(&self, start: u64, n: usize, now: DateTime<Utc>) -> Vec<Row> {
        let mut rng = rand::rng();
        let max_secs = i64::from(self.max_age_days) * 86_400;
        (0..n as u64)
            .map(|i| {
                let updated_at = now - Duration::seconds(rng.random_range(0..=max_secs));
                let created_at = updated_at - Duration::seconds(rng.random_range(0..=86_400));
                let ts = Timestamp::from_unix(
                    NoContext,
                    created_at.timestamp() as u64,
                    created_at.timestamp_subsec_nanos(),
                );
                Row {
                    id: Uuid::new_v7(ts),
                    title: format!("seed-{}", start + i),
                    completed: rng.random_bool(self.completed_ratio.clamp(0.0, 1.0)),
                    created_at,
                    updated_at,
                }
            })
            .collect()
    }
}

async fn insert_rows(pool: &MySqlPool, rows: &[Row]) -> Result<u64, sqlx::Error> {
    let mut qb: QueryBuilder<MySql> =
        QueryBuilder::new("INSERT INTO todos (id, title, completed, created_at, updated_at) ");
    qb.push_values(rows, |mut b, r| {
        b.push_bind(r.id)
            .push_bind(&r.title)
            .push_bind(r.completed)
            .push_bind(r.created_at)
            .push_bind(r.updated_at);
    });
    Ok(qb.build().execute(pool).await?.rows_affected())
}

#[async_trait]
impl Job for SeedJob {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn params(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    async fn run(&self, ctx: &mut JobContext) -> anyhow::Result<()> {
        let mut done: u64 = match ctx.checkpoint() {
            Some(c) => c.parse().context("invalid seed checkpoint")?,
            None => 0,
        };
        if ctx.is_dry_run() {
            tracing::info!(
                remaining = self.count.saturating_sub(done),
                "dry-run: would insert"
            );
            return Ok(());
        }
        let per_stmt = rows_per_statement(self.chunk_size, COLUMNS) as u64;
        let now = Utc::now();
        let pool = ctx.pool().clone();
        let policy = RetryPolicy::default();

        let batches = (done..self.count)
            .step_by(per_stmt as usize)
            .map(|start| (start, per_stmt.min(self.count - start) as usize));

        let mut results = stream::iter(batches)
            .map(|(start, n)| {
                let rows = self.generate(start, n, now);
                let pool = pool.clone();
                async move { retry(policy, "seed.insert", || insert_rows(&pool, &rows)).await }
            })
            .buffered(self.concurrency.max(1));

        while let Some(inserted) = results.try_next().await? {
            done += inserted;
            ctx.commit_progress(inserted, Some(done.to_string()))
                .await?;
        }
        Ok(())
    }
}
