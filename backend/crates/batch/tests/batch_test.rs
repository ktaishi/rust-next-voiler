//! バッチの内部結合テスト (実 MySQL)。
//! 実行: DATABASE_URL=mysql://root:root@127.0.0.1:3306/app cargo test --all-features
#![cfg(feature = "integration")]

use batch::{
    jobs::{ArchiveCompletedJob, SeedJob},
    runner::{lock_name, JobRunner, RunError},
};
use chrono::{Duration, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;

async fn insert_todo(pool: &MySqlPool, title: &str, completed: bool, age_days: i64) {
    let ts = Utc::now() - Duration::days(age_days);
    sqlx::query(
        "INSERT INTO todos (id, title, completed, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7())
    .bind(title)
    .bind(completed)
    .bind(ts)
    .bind(ts)
    .execute(pool)
    .await
    .unwrap();
}

async fn titles(pool: &MySqlPool, table: &str) -> Vec<String> {
    sqlx::query_scalar(&format!("SELECT title FROM {table} ORDER BY title"))
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn count(pool: &MySqlPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

/// 4 件: 対象 2 件 (完了・40日前/35日前)、対象外 2 件 (未完了・古い / 完了・最近)
async fn fixture(pool: &MySqlPool) {
    insert_todo(pool, "done-old-1", true, 40).await;
    insert_todo(pool, "done-old-2", true, 35).await;
    insert_todo(pool, "open-old", false, 40).await;
    insert_todo(pool, "done-new", true, 1).await;
}

// AC-1
#[sqlx::test(migrations = "../../migrations")]
async fn archive_moves_only_eligible_rows(pool: MySqlPool) {
    fixture(&pool).await;
    let runner = JobRunner::new(pool.clone());

    let s = runner
        .run(&ArchiveCompletedJob::new(30, 1), false)
        .await
        .unwrap();

    assert_eq!(s.processed, 2);
    assert_eq!(
        titles(&pool, "todos_archive").await,
        ["done-old-1", "done-old-2"]
    );
    assert_eq!(titles(&pool, "todos").await, ["done-new", "open-old"]);
}

// AC-3
#[sqlx::test(migrations = "../../migrations")]
async fn archive_is_idempotent(pool: MySqlPool) {
    fixture(&pool).await;
    let runner = JobRunner::new(pool.clone());
    runner
        .run(&ArchiveCompletedJob::new(30, 10), false)
        .await
        .unwrap();
    let again = runner
        .run(&ArchiveCompletedJob::new(30, 10), false)
        .await
        .unwrap();

    assert_eq!(again.processed, 0);
    assert_eq!(count(&pool, "todos_archive").await, 2);
    assert_eq!(count(&pool, "todos").await, 2);
}

// AC-9
#[sqlx::test(migrations = "../../migrations")]
async fn archive_dry_run_changes_nothing(pool: MySqlPool) {
    fixture(&pool).await;
    JobRunner::new(pool.clone())
        .run(&ArchiveCompletedJob::new(30, 10), true)
        .await
        .unwrap();

    assert_eq!(count(&pool, "todos").await, 4);
    assert_eq!(count(&pool, "todos_archive").await, 0);
    assert_eq!(count(&pool, "batch_jobs").await, 0);
}

// AC-2, AC-6, AC-8 (恒久エラーは再試行せず failed)
#[sqlx::test(migrations = "../../migrations")]
async fn archive_is_atomic_per_chunk_and_resumable(pool: MySqlPool) {
    // 古い順: a(40日) b(39) c(38) boom(37)。chunk=2 → [a,b] は成功、[c,boom] は DELETE で失敗させる
    for (t, age) in [("a", 40), ("b", 39), ("c", 38), ("boom", 37)] {
        insert_todo(&pool, t, true, age).await;
    }
    sqlx::raw_sql(
        "CREATE TRIGGER fail_on_boom BEFORE DELETE ON todos FOR EACH ROW \
         BEGIN IF OLD.title = 'boom' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'injected'; END IF; END",
    )
    .execute(&pool)
    .await
    .unwrap();
    let runner = JobRunner::new(pool.clone());

    let err = runner
        .run(&ArchiveCompletedJob::new(30, 2), false)
        .await
        .unwrap_err();
    let RunError::Failed { run_id, .. } = err else {
        panic!("unexpected: {err}")
    };

    // 1 チャンク目はコミット済み、2 チャンク目はロールバック: 両方にある / どちらにもない行は無い
    assert_eq!(titles(&pool, "todos_archive").await, ["a", "b"]);
    assert_eq!(titles(&pool, "todos").await, ["boom", "c"]);
    let run = runner.find(run_id).await.unwrap().unwrap();
    assert_eq!(run.status, "failed");
    assert_eq!(run.processed, 2);
    assert!(run.error.unwrap().contains("injected"));

    // 原因を取り除いて再開 → 同じ cutoff で続きから処理される
    sqlx::raw_sql("DROP TRIGGER fail_on_boom")
        .execute(&pool)
        .await
        .unwrap();
    let s = runner.resume(run_id).await.unwrap();
    assert_eq!(s.run_id, run_id);
    assert_eq!(s.processed, 4);
    assert_eq!(count(&pool, "todos").await, 0);
    assert_eq!(
        runner.find(run_id).await.unwrap().unwrap().status,
        "succeeded"
    );
    assert!(matches!(
        runner.resume(run_id).await,
        Err(RunError::AlreadySucceeded(_))
    ));
}

// AC-4
#[sqlx::test(migrations = "../../migrations")]
async fn second_run_fails_while_locked(pool: MySqlPool) {
    // 別コネクションが同名ジョブのロックを保持している状態を作る
    let mut holder = pool.acquire().await.unwrap();
    let name = lock_name(&mut holder, "archive-completed").await.unwrap();
    assert!(name.len() <= 64);
    let got: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, 0)")
        .bind(&name)
        .fetch_one(&mut *holder)
        .await
        .unwrap();
    assert_eq!(got, Some(1));

    let r = JobRunner::new(pool.clone())
        .run(&ArchiveCompletedJob::new(30, 10), false)
        .await;
    assert!(matches!(r, Err(RunError::AlreadyRunning(_))));
    assert_eq!(count(&pool, "batch_jobs").await, 0);

    // 保持側の接続が解放すれば実行できる
    sqlx::query("SELECT RELEASE_LOCK(?)")
        .bind(&name)
        .execute(&mut *holder)
        .await
        .unwrap();
    assert!(JobRunner::new(pool)
        .run(&ArchiveCompletedJob::new(30, 10), false)
        .await
        .is_ok());
}

// AC-5
#[sqlx::test(migrations = "../../migrations")]
async fn run_records_history(pool: MySqlPool) {
    fixture(&pool).await;
    let runner = JobRunner::new(pool.clone());
    let s = runner
        .run(&ArchiveCompletedJob::new(30, 10), false)
        .await
        .unwrap();

    let run = runner.find(s.run_id).await.unwrap().unwrap();
    assert_eq!(run.job_name, "archive-completed");
    assert_eq!(run.status, "succeeded");
    assert_eq!(run.processed, 2);
    assert!(run.finished_at.is_some());
    assert_eq!(run.params["chunk_size"], 10);
    assert_eq!(runner.recent(10).await.unwrap().len(), 1);
}

fn seed(count: u64) -> SeedJob {
    SeedJob {
        count,
        chunk_size: 100,
        concurrency: 3,
        completed_ratio: 0.5,
        max_age_days: 30,
    }
}

// AC-7
#[sqlx::test(migrations = "../../migrations")]
async fn seed_inserts_requested_count(pool: MySqlPool) {
    let s = JobRunner::new(pool.clone())
        .run(&seed(1_050), false)
        .await
        .unwrap();
    assert_eq!(s.processed, 1_050);
    assert_eq!(count(&pool, "todos").await, 1_050);
    // 一意な UUID v7 が採番されている
    let distinct: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT id) FROM todos")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(distinct, 1_050);
}

// AC-6
#[sqlx::test(migrations = "../../migrations")]
async fn seed_resumes_from_checkpoint(pool: MySqlPool) {
    // 300 件投入済みで失敗した実行を再現
    let job = seed(1_000);
    let partial = SeedJob {
        count: 300,
        ..job.clone()
    };
    JobRunner::new(pool.clone())
        .run(&partial, false)
        .await
        .unwrap();
    let run_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO batch_jobs (id, job_name, status, params, processed, checkpoint, error) \
         VALUES (?, 'seed', 'failed', ?, 300, '300', 'simulated crash')",
    )
    .bind(run_id)
    .bind(serde_json::to_value(&job).unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let s = JobRunner::new(pool.clone()).resume(run_id).await.unwrap();

    assert_eq!(s.processed, 1_000);
    assert_eq!(count(&pool, "todos").await, 1_000);
}
