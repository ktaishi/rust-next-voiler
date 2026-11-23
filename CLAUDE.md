# CLAUDE.md

このリポジトリは **AI 駆動開発** 前提のボイラープレート (Rust + Next.js + MySQL + Docker)。
作業前に `docs/process/ai-driven-workflow.md` を読み、フェーズとゲートに従うこと。

## 構成
```
backend/    Rust workspace
  crates/core/   app-core: domain, repository, DB 接続 (API/バッチ共有。axum に依存しない)
  crates/api/    HTTP API (axum 0.8)。handlers -> core::domain -> core::repository の一方向依存
  crates/batch/  バッチ CLI (JobRunner: ロック・履歴・チェックポイント・再試行)
  migrations/    共有マイグレーション
frontend/   Next.js 16 App Router + TypeScript. Vitest (単体) / Playwright (E2E: e2e/)
loadtest/   k6 シナリオ。閾値は loadtest/lib/config.js
docs/       要件・設計・ADR・OpenAPI・テンプレート
.claude/    skills (各フェーズのスラッシュコマンド), agents (reviewer 等), hooks
.github/    CI/CD, PR テンプレート, Claude レビュー
```

## コマンド (必ず Makefile 経由)
| 目的 | コマンド |
|---|---|
| コミット前チェック | `make check` (lint + typecheck + 単体) |
| 内部結合 (MySQL) | `make test-integration` |
| フルスタック起動 | `make up` / 停止 `make down` |
| E2E | `make up && make e2e` |
| 負荷試験 | `make loadtest PROFILE=smoke\|load\|stress\|spike` |
| バッチ実行 / 履歴 | `make batch ARGS="..."` / `make batch-status` |
| バッチ性能計測 | `make batch-bench BENCH_ROWS=1000000` |

## 開発フロー (スキル)
`/feature <説明>` で一気通貫。個別: `/requirements` → `/design` → `/implement` → `/test` →
`/integration-test` → `/load-test` → `/pr` → `/review` → `/merge`

## 絶対ルール
- **ゲートで止まる**: requirements / design の承認、PR の Approve は人間が行う。AI が自分で承認・マージ判断しない (`/merge` は人間の明示指示がある場合のみ)。
- **推測で埋めない**: 不明点は requirements.md の「未決事項」に書いて質問する。
- **契約先行**: API 変更は先に `docs/api/openapi.yaml` を更新し、`frontend/src/lib/api.ts` の型と backend の serde 構造体を一致させる。
- **TDD**: 失敗するテストを書いてから実装する。受入条件 ID (`AC-n`) をテスト名かコメントに含める。
- **マイグレーションは追加のみ**。既存の `backend/migrations/*.sql` を編集しない。
- テストを skip / 削除して green にしない。閾値を緩めるときは理由を PR に書く。
- main へ直接 push しない。ブランチ `feat/NNNN-slug` → PR → squash merge。
- 秘密情報をコミットしない (`.env` は gitignore 済み。`.env.example` を更新する)。

## コーディング規約
### Backend
- エラーは `AppError` に集約し、HTTP 変換は `error.rs` のみ。`unwrap()` はテスト以外禁止
- 入力抽出は `crate::extract::{Json, Path, Query}` を使う (エラー形式統一のため)
- SQL は repository 層のみ。`sqlx::query_as` + `FromRow` (ビルド時 DB 不要)
- 主キーは UUID v7 (`Uuid::now_v7()`) を `BINARY(16)` で保存。v4 / `CHAR(36)` は使わない
- 一覧 API は keyset (cursor) ページネーション。OFFSET は使わない
- テスト用の実装 (InMemory 等) は `tests/common/` に置き、本番コードに含めない
- `cargo clippy -D warnings` が通ること
- エラー: core は `AppError` (HTTP 非依存)、api は `ApiError` で HTTP に変換
### Batch
- 新規ジョブは `docs/process/batch-operations.md` の「新しいジョブの追加」に従う
- 1 チャンク = 1 トランザクション。全件をメモリに載せない。OFFSET で走査しない
- 一時エラーのみ `retry()`。チャンク処理は再実行しても安全 (冪等) に書く
- バルク SQL の行数は `chunk::rows_per_statement` でプレースホルダ上限内に収める
### Frontend
- API 呼び出しは `src/lib/api.ts` 経由のみ。ブラウザ → `/api/*` (Route Handler `src/lib/proxy.ts`) → backend
- backend の到達先は実行時環境変数 `API_ORIGIN`。ビルド時に URL を焼き込まない
- アクセシビリティ: テストは `getByRole` / `getByLabelText` で書ける UI にする
- Next.js 16 は破壊的変更あり。API は `frontend/node_modules/next/dist/docs/` を確認 (frontend/AGENTS.md)

## 新しいリソースを追加するとき (例: users)
1. `backend/migrations/<ts>_create_users.sql`
2. `backend/crates/core/src/domain/user.rs` (型 + バリデーション + 単体テスト)
3. `backend/crates/core/src/repository/{mod,mysql}.rs` に trait と MySQL 実装、結合テストは `crates/core/tests/`
4. `backend/crates/api/src/handlers/users.rs`, `routes.rs` に登録, `state.rs` に repo 追加
5. `backend/crates/api/tests/` に HTTP テスト (InMemory 実装は `tests/common/`)
6. `docs/api/openapi.yaml`, `frontend/src/lib/api.ts`, 画面, Vitest, Playwright
7. `loadtest/scenarios/` にシナリオ
