# rust-next-voiler

**AI 駆動開発** 前提のフルスタック ボイラープレート。
要件定義 → 設計 → 実装 → テスト → 内部結合 → 負荷試験 → CI/CD → PR → レビュー → マージ を、
Claude Code のスキル (スラッシュコマンド) と GitHub Actions で一貫して回せる。

| 層 | 技術 |
|---|---|
| Backend | Rust, axum 0.8, sqlx 0.8, tokio |
| Frontend | Next.js 16 (App Router), React 19, TypeScript |
| DB | MySQL 8.4 |
| Infra | Docker / docker compose |
| Test | cargo test, `#[sqlx::test]`, Vitest + Testing Library, Playwright, k6 |
| CI/CD | GitHub Actions (CI, CD→GHCR, 負荷試験, Claude 自動レビュー) |

## クイックスタート
```bash
cp .env.example .env
make up            # mysql + backend(:8080) + frontend(:3000)
open http://localhost:3000
make e2e           # E2E
make loadtest      # k6 smoke
make down
```
ローカル開発 (ホットリロード):
```bash
make setup
make dev-backend   # 別ターミナル
make dev-frontend
```
`make help` で全コマンド。

## AI 駆動開発フロー
Claude Code でリポジトリを開き:
```
/feature タスクに期限日を追加したい
```
フェーズごとに実行する場合:

| フェーズ | コマンド | 成果物 / ゲート |
|---|---|---|
| 要件定義 | `/requirements <説明>` | `docs/features/NNNN-slug/requirements.md` → **人間承認** |
| 設計 | `/design NNNN-slug` | `design.md`, `docs/api/openapi.yaml`, ADR → **人間承認** |
| 実装 | `/implement NNNN-slug` | TDD でコード + テスト |
| テスト | `/test NNNN-slug` | AC 網羅の単体/HTTP/コンポーネントテスト |
| 内部結合 | `/integration-test NNNN-slug` | MySQL 結合テスト + Playwright E2E |
| 負荷試験 | `/load-test NNNN-slug load` | `load-test-report.md` (SLO 判定) |
| PR | `/pr NNNN-slug` | テンプレート準拠の PR → **CI green** |
| レビュー | `/review <PR番号> --comment` | reviewer / security-reviewer エージェントによるレビュー |
| マージ | `/merge <PR番号>` | **人間 Approve 後のみ** squash merge → CD |

詳細: [docs/process/ai-driven-workflow.md](docs/process/ai-driven-workflow.md) / AI 向け規約: [CLAUDE.md](CLAUDE.md)

## ディレクトリ
```
backend/            Rust API (handlers → domain → repository)
  migrations/       sqlx マイグレーション (追加のみ)
  tests/            HTTP テスト / MySQL 結合テスト (--features integration)
frontend/           Next.js
  src/lib/api.ts    API クライアント (型は openapi.yaml と一致)
  e2e/              Playwright
loadtest/           k6 (scenarios/, lib/config.js に SLO 閾値)
docs/               プロセス, テンプレート, 機能別成果物, ADR, OpenAPI, DB
.claude/            skills (フェーズ別コマンド), agents (reviewer 等), hooks (自動整形), settings
.github/            CI/CD, PR/Issue テンプレート, Claude レビュー, dependabot
```

## CI/CD
| ワークフロー | トリガ | 内容 |
|---|---|---|
| `ci.yml` | PR / main push | backend: fmt, clippy, 単体, MySQL 結合 / frontend: lint, format, 型, 単体, build / compose で E2E + k6 smoke |
| `claude-review.yml` | PR | Claude による自動一次レビュー |
| `claude.yml` | `@claude` メンション | Issue/PR 上で Claude が実装・修正 |
| `loadtest.yml` | 手動 / 毎週 | k6 load/stress/spike。結果を Job Summary に表示 |
| `cd.yml` | main / `v*` タグ | GHCR へイメージ push → staging / production (承認制) |

初期設定 (ブランチ保護・Secrets など): [docs/process/github-setup.md](docs/process/github-setup.md)

## API
| Method | Path | 説明 |
|---|---|---|
| GET | `/healthz` | liveness |
| GET | `/readyz` | readiness (DB 疎通) |
| GET | `/api/v1/todos?limit&offset` | 一覧 |
| POST | `/api/v1/todos` | 作成 |
| GET/PATCH/DELETE | `/api/v1/todos/{id}` | 取得 / 部分更新 / 削除 |

エラー形式: `{"code":"VALIDATION_ERROR|NOT_FOUND|INTERNAL_ERROR","message":"..."}`。契約: [docs/api/openapi.yaml](docs/api/openapi.yaml)
