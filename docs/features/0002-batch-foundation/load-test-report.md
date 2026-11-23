# 0002: バッチ処理基盤 性能試験結果

- 実施日: 2026-11-23
- 環境: ローカル docker compose (4 vCPU / 15 GB RAM, MySQL 8.4 既定設定 = innodb_buffer_pool_size 128 MB)。batch は release ビルドをホストで実行
- 手順: `make up && make batch-bench` (seed 100 万行 → dry-run → archive)、k6 は `--vus 50 --duration 30s`

> ローカル計測のため絶対値は参考。本番相当の判定は stg (本番同等のインスタンス・buffer pool) で再計測すること。

## バッチ処理性能
| ジョブ | 条件 | 件数 | 所要時間 | スループット |
|---|---|---|---|---|
| seed | chunk 2,000 行/文 × 並列 4 | 1,000,000 | 8.0 s | 約 125,000 行/s |
| archive-completed (dry-run) | 対象件数の COUNT | 253,596 | 0.05 s | — (`idx_todos_completed_updated` で範囲検索) |
| archive-completed | 180 日より前, chunk 2,000 | 253,596 | 6.7 s | 約 38,000 行/s |
| archive-completed | 30 日より前, chunk 2,000 (k6 負荷と同時) | 205,693 | 5.9 s | 約 35,000 行/s |

判定: 要件「100 万行の seed と 50 万行規模のアーカイブが数分以内」 → **合格** (いずれも 10 秒未満)

## バッチ実行中のオンライン API への影響 (todos 約 75 万行)
| 指標 | 閾値 | バッチなし | archive 実行中 | 判定 |
|---|---|---|---|---|
| p95 | < 200ms | 7.3 ms | 16.1 ms | ✅ |
| p99 | < 500ms | 17.8 ms | 55.4 ms | ✅ |
| エラー率 | < 1% | 0.00% | 0.00% | ✅ |
| リクエスト数 (30s) | — | 11,800 | 11,612 | |

チャンク単位の短いトランザクションと `FOR UPDATE SKIP LOCKED` により、オンラインの更新とのロック競合でエラーは発生しなかった。
p99 は約 3 倍になっており、本番ではオフピークに実行するか `--chunk-size` を下げてトランザクションを短くする。

## ページネーション (Phase 1 の効果確認, todos 約 75 万行)
| 方式 | クエリ | 所要時間 |
|---|---|---|
| OFFSET (旧) | `ORDER BY id DESC LIMIT 21 OFFSET 700000` | 279.8 ms |
| keyset (現行) | `WHERE id < ? ORDER BY id DESC LIMIT 21` | 3.3 ms |
| API 経由 (keyset, 同じ深さ) | `GET /api/v1/todos?limit=20&cursor=...` | 1〜2 ms |

## 機能確認 (CLI)
- 同名ジョブの 2 重起動: 2 つ目が `job 'seed' is already running` で **終了コード 1** (AC-4)
- `status` で実行履歴を表示 (AC-10)

## 所見・フォローアップ
- seed は並列 INSERT で buffer pool (128 MB) が小さいため、本番相当の設定ではさらに速くなる見込み
- アーカイブ済みデータの削除 (保持期間) は別ジョブとして未実装 (要件 Q1)
