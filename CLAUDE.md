# CLAUDE.md

このリポジトリは **AI 駆動開発** 前提のボイラープレート (Rust + Next.js + MySQL + Docker)。
作業前に `docs/process/ai-driven-workflow.md` を読み、フェーズとゲートに従うこと。

## 構成
```
backend/    Rust (axum 0.8, sqlx 0.8 MySQL). handlers -> domain -> repository の一方向依存
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
- SQL は repository 層のみ。`sqlx::query_as` + Row 構造体 (ビルド時 DB 不要)
- `cargo clippy -D warnings` が通ること
### Frontend
- API 呼び出しは `src/lib/api.ts` 経由のみ
- アクセシビリティ: テストは `getByRole` / `getByLabelText` で書ける UI にする
- Next.js 16 は破壊的変更あり。API は `frontend/node_modules/next/dist/docs/` を確認 (frontend/AGENTS.md)

## 新しいリソースを追加するとき (例: users)
1. `backend/migrations/<ts>_create_users.sql`
2. `backend/src/domain/user.rs` (型 + バリデーション + 単体テスト)
3. `backend/src/repository/{mod,mysql,memory}.rs` に trait と実装
4. `backend/src/handlers/users.rs`, `routes.rs` に登録, `state.rs` に repo 追加
5. `backend/tests/` に HTTP テストと `integration` feature の結合テスト
6. `docs/api/openapi.yaml`, `frontend/src/lib/api.ts`, 画面, Vitest, Playwright
7. `loadtest/scenarios/` にシナリオ
