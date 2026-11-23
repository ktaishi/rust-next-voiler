# DB スキーマ

正は `backend/migrations/*.sql`。本書は概要。

## 規約
- 文字コード `utf8mb4` / 照合順序 `utf8mb4_0900_ai_ci` / エンジン InnoDB
- 主キーは UUID v4 文字列 `CHAR(36)`
- 日時は `DATETIME(6)` (UTC で保存)
- マイグレーションは追加のみ (既存ファイルを編集しない)。破壊的変更は expand → migrate → contract の 3 段階で行う
- ファイル名: `YYYYMMDDHHMMSS_<snake_case>.sql`

## テーブル

### todos
| カラム | 型 | NULL | 既定 | 説明 |
|---|---|---|---|---|
| id | CHAR(36) | NO | | PK |
| title | VARCHAR(255) | NO | | 1〜255 文字 (前後空白除去) |
| completed | BOOLEAN | NO | FALSE | |
| created_at | DATETIME(6) | NO | CURRENT_TIMESTAMP(6) | idx_todos_created_at |
| updated_at | DATETIME(6) | NO | ON UPDATE | |
