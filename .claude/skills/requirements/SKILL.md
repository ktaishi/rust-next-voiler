---
name: requirements
description: 機能の要件定義書 (受入条件付き) を docs/features/NNNN-slug/requirements.md に作成する。新機能・仕様変更の最初に使う。
argument-hint: <機能の説明 or Issue 番号>
---

# /requirements: 要件定義

対象: $ARGUMENTS

1. Issue 番号なら GitHub から本文を取得する。既存コード・`docs/features/` を読み、関連機能と重複を確認する
2. 次の連番 `NNNN` を決め (`ls docs/features`)、slug を英小文字ケバブケースで付ける
3. `docs/templates/requirements.md` をコピーして `docs/features/NNNN-slug/requirements.md` を作る
4. 記入ルール
   - 受入条件は **Given/When/Then** で、テストで機械的に検証できる粒度にする。ID `AC-1..n`
   - 異常系・境界値 (空, 最大長, 存在しない ID, 重複, 権限) を必ず受入条件に含める
   - 非機能要件は標準 SLO (p95<200ms, エラー率<1%) から逸脱する場合のみ明記
   - 分からないことは推測せず「未決事項」に質問として書く
5. ユーザーに要約 (スコープ / AC 一覧 / 未決事項) を提示し、**承認を求めて停止する (ゲート G1)**
6. 承認されたらステータスを `Approved` にし、承認者と日付を記入
