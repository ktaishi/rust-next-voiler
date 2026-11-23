# DB スキーマ

正は `backend/migrations/*.sql`。本書は概要。

## 規約
- 文字コード `utf8mb4` / 照合順序 `utf8mb4_0900_ai_ci` / エンジン InnoDB
- 主キーは **UUID v7 を `BINARY(16)`** で保持する (アプリ側で `Uuid::now_v7()` を採番)
  - v7 は先頭 48bit が Unix ミリ秒なので、InnoDB のクラスタ化インデックスに末尾追記され、ページ分割が起きにくい
  - `CHAR(36)` の v4 はランダム挿入 + 36byte で、セカンダリインデックスにも主キーが複製されるため大量データで不利
  - 手で調べるときは `SELECT BIN_TO_UUID(id), ...` (MySQL 8.0+)
- 一覧は **keyset ページネーション** (`WHERE id < ? ORDER BY id DESC LIMIT n`)。OFFSET は使わない
- 日時は `DATETIME(6)` (UTC で保存)
- マイグレーションは追加のみ (既存ファイルを編集しない)。破壊的変更は expand → migrate → contract の 3 段階で行う
- ファイル名: `YYYYMMDDHHMMSS_<snake_case>.sql`

## テーブル

### todos
| カラム | 型 | NULL | 既定 | 説明 |
|---|---|---|---|---|
| id | BINARY(16) | NO | | PK, UUID v7 |
| title | VARCHAR(255) | NO | | 1〜255 文字 (前後空白除去) |
| completed | BOOLEAN | NO | FALSE | |
| created_at | DATETIME(6) | NO | CURRENT_TIMESTAMP(6) | |
| updated_at | DATETIME(6) | NO | ON UPDATE | |
