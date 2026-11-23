# バッチ運用ガイド

## 実行方法
| 環境 | コマンド |
|---|---|
| ローカル (ホスト) | `make batch ARGS="archive-completed --older-than-days 30"` |
| ローカル (コンテナ) | `docker compose run --rm batch archive-completed --older-than-days 30` |
| 本番 | backend イメージの `/app/batch` を ENTRYPOINT にして起動 (下記例) |

```
batch [--dry-run] seed --count N [--chunk-size 1000] [--concurrency 4] [--completed-ratio 0.5] [--max-age-days 365]
batch [--dry-run] archive-completed --older-than-days N [--chunk-size 1000]
batch resume <RUN_ID>
batch status [--limit 20]
```
環境変数: `DATABASE_URL` (必須), `DB_MAX_CONNECTIONS` (既定 10, `--concurrency + 2` 以上), `LOG_FORMAT=json`, `RUST_LOG`

## 終了コードと監視
| 終了コード | 意味 | 対応 |
|---|---|---|
| 0 | 成功 | — |
| 1 | 失敗 (ジョブエラー / 多重起動 / 接続失敗) | ログと `batch status` の ERROR 列を確認 |

スケジューラ側で「非 0 終了」をアラートにする。`batch_jobs` の `status='running'` が長時間続く行も監視対象 (異常終了の可能性)。

## 失敗時の再開
```bash
batch status                 # failed の RUN_ID を確認
batch resume <RUN_ID>        # 同じパラメータ・チェックポイントから再開
```
- 再開時はロックを取得してから行うため、実行中のジョブを誤って再開することはない
- プロセスが強制終了され `running` のまま残った行も、ロックが取れれば再開できる

## スケジュール実行の例
### Kubernetes CronJob
```yaml
apiVersion: batch/v1
kind: CronJob
metadata: { name: archive-completed }
spec:
  schedule: "17 3 * * *"          # 毎日 03:17 (オフピーク)
  concurrencyPolicy: Forbid        # 多重起動はアプリ側の GET_LOCK でも防止している
  jobTemplate:
    spec:
      backoffLimit: 0              # 再試行はアプリ内 (一時エラー) と resume で行う
      template:
        spec:
          restartPolicy: Never
          containers:
            - name: batch
              image: ghcr.io/<owner>/<repo>/backend:<tag>
              command: ["/app/batch"]
              args: ["archive-completed", "--older-than-days", "30", "--chunk-size", "1000"]
              envFrom: [{ secretRef: { name: backend-db } }]
              env: [{ name: LOG_FORMAT, value: json }]
```
### ECS Scheduled Task (EventBridge Scheduler)
タスク定義の `command` を `["/app/batch", "archive-completed", "--older-than-days", "30"]` に上書きして、cron 式でスケジュールする。

## チューニング
| パラメータ | 上げると | 下げると |
|---|---|---|
| `--chunk-size` | スループット↑、1 トランザクションが長くなりロック保持・レプリカ遅延が増える | オンラインへの影響↓ |
| `--concurrency` (seed) | スループット↑、DB 負荷↑ (`DB_MAX_CONNECTIONS` も上げる) | 負荷↓ |

目安: オンライン API の p99 を監視しながら調整する。実測値は `docs/features/0002-batch-foundation/load-test-report.md`。

## 新しいジョブの追加
1. `backend/crates/batch/src/jobs/<name>.rs` に構造体 (= パラメータ, `Serialize + Deserialize`) を作り `Job` を実装
   - 1 チャンク = 1 トランザクション、`retry()` で包む、チャンクごとに `ctx.commit_progress()`
   - 処理済みの行が対象から外れる設計 (移動・フラグ更新) か、`ctx.checkpoint()` に keyset の位置を保存して再開可能にする
   - 「今」に依存する境界 (日時など) は構築時に確定させ、params に保存する
2. `jobs/mod.rs` の `build()` に登録、`main.rs` にサブコマンドを追加
3. `crates/batch/tests/` に結合テスト (正常系・冪等・途中失敗・再開)
4. `make batch-bench` 相当の性能計測を行い、レポートを残す
