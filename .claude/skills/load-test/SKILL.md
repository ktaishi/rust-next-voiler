---
name: load-test
description: k6 で負荷試験を実行し SLO 判定、結果レポートを docs/features/NNNN-slug/load-test-report.md に残す。
argument-hint: <NNNN-slug> [smoke|load|stress|spike]
---

# /load-test: 負荷試験

引数: $ARGUMENTS (プロファイル省略時は `load`)

1. 新しい/変更されたエンドポイントが `loadtest/scenarios/` に含まれているか確認し、無ければシナリオを追加
   - `tags: { name: "METHOD /path" }` を付けてエンドポイント別に集計できるようにする
2. `make up` 後、まず `make loadtest PROFILE=smoke` で動作確認、次に指定プロファイルで実行
3. `loadtest/results/summary.json` から p95 / p99 / エラー率 / RPS を読み取り、閾値 (`loadtest/lib/config.js`) と比較
4. `docs/templates/load-test-report.md` から `docs/features/<slug>/load-test-report.md` を作成し結果を記入
5. 閾値未達なら: スロークエリ (EXPLAIN), N+1, コネクションプール (`DB_MAX_CONNECTIONS`), インデックスを調査し対策案を書く。
   閾値を緩めるのは最終手段で、必ずユーザーに相談する
6. 注意: ローカル結果は参考値。本番相当の判定は stg 環境で `loadtest.yml` ワークフローを使う
