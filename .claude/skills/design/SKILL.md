---
name: design
description: 承認済み要件から設計書 (API/DB/画面/テスト設計) を作成し、OpenAPI とマイグレーション案を更新する。
argument-hint: <NNNN-slug>
---

# /design: 設計

対象: docs/features/$ARGUMENTS

前提: `requirements.md` が `Approved`。でなければ停止して `/requirements` を促す。

1. `docs/templates/design.md` から `design.md` を作成
2. 方式検討: 最低 2 案を比較し採用理由を書く。技術選定や横断的な判断なら `docs/adr/NNNN-*.md` を追加
3. API: `docs/api/openapi.yaml` を先に更新 (契約先行)。エラーは共通 `Error` スキーマ
4. DB: 新規マイグレーションファイル名とDDLを設計書に記載 (この段階では作成のみ可、既存は編集しない)。
   インデックス、NULL 可否、後方互換 (expand/contract) を検討
5. テスト設計: 全 AC について単体 / HTTP / 結合 / E2E / 負荷のどこで検証するかをトレーサビリティ表に埋める。空欄の AC が無いこと
6. 要約を提示し **承認を求めて停止 (ゲート G2)**。承認後ステータスを `Approved` に
