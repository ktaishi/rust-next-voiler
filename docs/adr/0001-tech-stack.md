# ADR-0001: 技術スタック

- ステータス: Accepted
- 日付: 2026-11-23

## コンテキスト
AI 駆動開発を前提に、型による早期検出と自動テストのしやすさを重視したボイラープレートを用意する。

## 決定
| 領域 | 採用 | 理由 |
|---|---|---|
| Backend | Rust / axum 0.8 / sqlx 0.8 / tokio | 型安全・高性能。AI の生成コードもコンパイラで検証できる |
| DB | MySQL 8.4 LTS | 運用実績。sqlx の migrate でスキーマ管理 |
| Frontend | Next.js 16 (App Router) / React 19 / TypeScript | 標準的で情報が多い |
| API 契約 | OpenAPI 3.1 (`docs/api/openapi.yaml`) | フロント・バック・テストの単一の真実 |
| テスト | cargo test, `#[sqlx::test]`, Vitest + Testing Library, Playwright | 層ごとに最適なツール |
| 負荷試験 | k6 | スクリプトが JS で書け、閾値で合否判定できる |
| CI/CD | GitHub Actions + GHCR | リポジトリと一体で管理 |
| AI | Claude Code (skills / subagents / hooks) + claude-code-action | ローカルと PR の両方で同じ規約を使う |

## 検討した代替案
- SeaORM / Diesel: 抽象度が高く、生成 SQL の可読性が下がるため sqlx を採用
- PostgreSQL: 要件として MySQL 指定のため不採用

## 結果
- sqlx は実行時クエリ (`query_as`) を使い、ビルドに DB 接続を要求しない (オフラインビルド可)。
- 型安全性は repository 層の Row 構造体と結合テストで担保する。
