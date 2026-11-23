# 0002: バッチ処理基盤 設計

- ステータス: Approved
- 要件: ./requirements.md

## 1. 概要
backend を Cargo workspace に分割し、`app-core` (domain / repository / DB 接続) を `api` と `batch` が共有する。
バッチは CLI (clap) で起動し、共通の `JobRunner` がロック・実行履歴・チェックポイント・再試行を担う。
個々のジョブは `Job` trait を実装するだけでよい。

```
backend/
  migrations/            共有マイグレーション
  crates/core/           app-core: domain, repository, error, db, telemetry
  crates/api/            HTTP サーバ (axum)
  crates/batch/          バッチ CLI (clap) + JobRunner + jobs/
```

## 2. 方式検討
| 案 | 概要 | 長所 | 短所 |
|---|---|---|---|
| A (採用) | workspace + 独立バイナリ | API と独立にスケール・デプロイ。タイムアウトの制約なし。依存が最小 | 初期の分割コスト |
| B | 同一クレートに `src/bin/batch.rs` | 変更が小さい | axum 等の不要な依存を抱え、境界が曖昧になる |
| C | API に「バッチ実行」エンドポイント | 起動が簡単 | HTTP タイムアウト、API のリソースを食う、多重起動制御が困難 |

## 3. DB 設計
新規マイグレーション `20260201000000_batch_foundation.sql` (追加のみ)。

- `todos` に `idx_todos_completed_updated (completed, updated_at, id)` を追加: アーカイブ対象の抽出をインデックス範囲検索にする
- `todos_archive`: `todos` と同じ列 + `archived_at`。主キー `id` なので再投入は `INSERT IGNORE` で冪等
- `batch_jobs`: 実行履歴

| 列 | 型 | 説明 |
|---|---|---|
| id | BINARY(16) PK | 実行 ID (UUID v7) |
| job_name | VARCHAR(64) | ジョブ名 |
| status | ENUM('running','succeeded','failed') | |
| params | JSON | 実行パラメータ (再開時に復元) |
| processed | BIGINT UNSIGNED | 処理済み件数 |
| checkpoint | VARCHAR(255) NULL | ジョブ固有の再開位置 |
| error | TEXT NULL | 失敗理由 |
| started_at / finished_at / updated_at | DATETIME(6) | |

## 4. 実行シーケンス (JobRunner)
1. 専用コネクションで `GET_LOCK('batch:<job>', 0)` を取得。取れなければ即終了 (AC-4)。接続が切れればロックは自動解放
2. 新規: `batch_jobs` に running で INSERT / 再開: failed の行を running に戻し、params と checkpoint を復元 (AC-6)
3. `Job::run(ctx)` を実行。ジョブはチャンクごとに `ctx.commit_progress(delta, checkpoint)` を呼び、進捗を永続化する (AC-5)
4. 成功 → succeeded / 失敗 → failed + error を記録し、非 0 で終了 (AC-8)
5. `RELEASE_LOCK`

## 5. ジョブ設計
### archive-completed (AC-1〜3, 9)
- 1 チャンク = 1 トランザクション:
  1. `SELECT id FROM todos WHERE completed AND updated_at < ? ORDER BY updated_at, id LIMIT ? FOR UPDATE SKIP LOCKED`
  2. `INSERT IGNORE INTO todos_archive SELECT ..., NOW(6) FROM todos WHERE id IN (...)`
  3. `DELETE FROM todos WHERE id IN (...)`
  4. COMMIT
- 処理した行は `todos` から消えるため、次のチャンクは常に先頭から取ればよい (OFFSET もカーソルも不要)。
  途中で落ちても未コミットのチャンクはロールバックされ、再実行で続きから処理される (AC-2, 3)
- `cutoff` は開始時に 1 度だけ計算して params に保存し、再開時も同じ境界を使う
- `SKIP LOCKED` により、オンラインの更新とロック待ちで詰まらない

### seed (AC-7)
- 1 文の行数 = `min(chunk_size, 65535 / 列数)` (`chunk.rs`)
- `QueryBuilder::push_values` でバルク INSERT。`futures::StreamExt::buffered(concurrency)` で並列実行しつつ順序を保ち、
  完了済みの連続範囲をチェックポイント (投入済み件数) として保存。再開時はその件数から続行
- ID は `updated_at` に合わせた時刻の UUID v7 (古いデータを再現)

## 6. 再試行 (AC-8)
`retry.rs`: MySQL エラー 1213 (deadlock), 1205 (lock wait timeout), 接続系エラー (`Io`, `PoolTimedOut`, `PoolClosed`) のみ再試行。
指数バックオフ (100ms 起点, 最大 5 回, 上限 5 秒)。それ以外 (制約違反等) は即失敗。

## 7. 運用
- イメージは backend と共通 (`/app/batch`)。`docker compose run --rm batch <job> ...`
- スケジュール実行例: `docs/process/batch-operations.md` (k8s CronJob, ECS)

## 8. テスト設計
| AC | 単体 | 結合 (MySQL, `crates/batch/tests`) | 性能 |
|---|---|---|---|
| AC-1 | | `archive_moves_only_eligible_rows` | `make batch-bench` |
| AC-2 | | `archive_is_atomic_per_chunk` (チャンク途中失敗の注入) | |
| AC-3 | | `archive_is_idempotent` | |
| AC-4 | | `second_run_fails_while_locked` | |
| AC-5 | | `run_records_history` | |
| AC-6 | | `seed_resumes_from_checkpoint` | |
| AC-7 | `rows_per_statement_respects_placeholder_limit` | `seed_inserts_requested_count` | `make batch-bench` |
| AC-8 | `retryable_errors_are_classified`, `backoff_is_capped` | | |
| AC-9 | | `archive_dry_run_changes_nothing` | |
| AC-10 | | (`status` は手動確認) | |
