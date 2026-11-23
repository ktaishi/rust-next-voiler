# 0001: Todo CRUD 設計 (サンプル)

- ステータス: Approved
- 要件: ./requirements.md

## 1. 概要
REST API (`/api/v1/todos`) + MySQL `todos` テーブル + Next.js のクライアントコンポーネント。
ブラウザは同一オリジン `/api/*` を叩き、Next.js の rewrites でバックエンドへ転送する (CORS 不要)。

## 3. API 設計
`docs/api/openapi.yaml` 参照。

## 4. DB 設計
`backend/migrations/20260101000000_create_todos.sql`。`created_at` にインデックス。

## 5. バックエンド
- `domain/todo.rs`: タイトル正規化 (trim, 1〜255 文字)
- `repository/`: `TodoRepository` trait、MySQL 実装とインメモリ実装
- `handlers/todos.rs`: 入力抽出エラーも `AppError` の JSON 形式に統一 (`extract.rs`)

## 6. フロントエンド
- `components/TodoApp.tsx`: 一覧・追加・切替・削除。サーバ応答後に state 更新 (楽観更新なし)

## 8. テスト設計
| AC | 単体 | HTTP/コンポーネント | 結合 (MySQL) | E2E | 負荷 |
|---|---|---|---|---|---|
| AC-1 | | `todo_crud_flow`, `タスクを追加できる` | `create_and_get` | `todo.spec.ts` | `todos.js` |
| AC-2 | `empty_title_is_rejected` | `create_rejects_blank_title` | | | |
| AC-3 | `title_length_boundary` | | | | |
| AC-4 | | `todo_crud_flow` | `update_partial_fields` | `todo.spec.ts` (reload) | `todos.js` |
| AC-5 | | `todo_crud_flow` | `update_and_delete_missing` | `todo.spec.ts` | `todos.js` |
| AC-6 | | | `list_pagination` | | |
