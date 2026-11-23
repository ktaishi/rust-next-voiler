---
name: feature
description: 要件定義からマージまでの全フェーズを順に実行するオーケストレータ。新機能開発を一気通貫で進めたいときに使う。
argument-hint: <機能の説明 or Issue 番号>
disable-model-invocation: true
---

# /feature: 一気通貫開発

対象: $ARGUMENTS

`docs/process/ai-driven-workflow.md` のフェーズを順に実行する。各フェーズは対応するスキルの手順に従う。

1. `/requirements` の手順で要件定義 → **G1: ユーザーに承認を求めて停止**
2. 承認後 `/design` → **G2: 承認を求めて停止**
3. `/implement` → `/test` → `/integration-test` → `/load-test` を連続実行。各フェーズ完了時に 1〜2 行で進捗報告
   - どこかで失敗したら原因を直して該当フェーズからやり直す。3 回直せなければ停止して相談
4. `/pr` で PR 作成 → **G3: CI 結果を確認** (失敗なら修正 push)
5. `/review` で一次レビューし、指摘を修正
6. **G4: 人間の Approve を待つ**。`/merge` はユーザーが明示的に指示したときのみ

最後に、成果物 (docs/features/... と PR URL) と残課題を一覧で報告する。
