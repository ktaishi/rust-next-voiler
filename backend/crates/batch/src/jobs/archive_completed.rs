use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::{
    retry::{retry, RetryPolicy},
    runner::{Job, JobContext},
};

/// 完了済みかつ `updated_at < cutoff` の Todo を `todos_archive` へ移動する。
///
/// 1 チャンク = 1 トランザクション (SELECT ... FOR UPDATE SKIP LOCKED → INSERT IGNORE → DELETE)。
/// 処理済みの行は `todos` から消えるので、毎回先頭から取ればよく、途中で失敗しても再実行で続きから進む。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveCompletedJob {
    pub cutoff: DateTime<Utc>,
    pub chunk_size: u32,
}

impl ArchiveCompletedJob {
    pub const NAME: &'static str = "archive-completed";

    /// 境界日時は開始時に 1 度だけ決め、params として保存する (再開時も同じ境界を使う)。
    pub fn new(older_than_days: u32, chunk_size: u32) -> Self {
        Self {
            cutoff: Utc::now() - Duration::days(i64::from(older_than_days)),
            chunk_size: chunk_size.max(1),
        }
    }
}

fn push_id_list(qb: &mut QueryBuilder<'_, MySql>, ids: &[Uuid]) {
    qb.push(" WHERE id IN (");
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    sep.push_unseparated(")");
}

async fn archive_chunk(
    pool: &MySqlPool,
    cutoff: DateTime<Utc>,
    limit: u32,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM todos WHERE completed = TRUE AND updated_at < ? \
         ORDER BY updated_at, id LIMIT ? FOR UPDATE SKIP LOCKED",
    )
    .bind(cutoff)
    .bind(limit)
    .fetch_all(&mut *tx)
    .await?;
    if ids.is_empty() {
        tx.commit().await?;
        return Ok(0);
    }

    let mut ins = QueryBuilder::<MySql>::new(
        "INSERT IGNORE INTO todos_archive (id, title, completed, created_at, updated_at, archived_at) \
         SELECT id, title, completed, created_at, updated_at, NOW(6) FROM todos",
    );
    push_id_list(&mut ins, &ids);
    ins.build().execute(&mut *tx).await?;

    let mut del = QueryBuilder::<MySql>::new("DELETE FROM todos");
    push_id_list(&mut del, &ids);
    let deleted = del.build().execute(&mut *tx).await?.rows_affected();

    tx.commit().await?;
    Ok(deleted)
}

#[async_trait]
impl Job for ArchiveCompletedJob {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn params(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    async fn run(&self, ctx: &mut JobContext) -> anyhow::Result<()> {
        if ctx.is_dry_run() {
            let n: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM todos WHERE completed = TRUE AND updated_at < ?",
            )
            .bind(self.cutoff)
            .fetch_one(ctx.pool())
            .await?;
            tracing::info!(targets = n, cutoff = %self.cutoff, "dry-run: would archive");
            return Ok(());
        }
        let pool = ctx.pool().clone();
        let policy = RetryPolicy::default();
        loop {
            let moved = retry(policy, "archive.chunk", || {
                archive_chunk(&pool, self.cutoff, self.chunk_size)
            })
            .await?;
            if moved == 0 {
                break;
            }
            ctx.commit_progress(moved, None).await?;
        }
        Ok(())
    }
}
