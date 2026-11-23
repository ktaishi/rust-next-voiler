---
name: merge
description: 承認済み・CI green の PR を squash merge する。ユーザーが明示的に指示した場合のみ使う。
argument-hint: <PR 番号>
disable-model-invocation: true
---

# /merge: マージ

対象 PR: $ARGUMENTS

ユーザーが明示的にこのコマンドを実行した場合のみ進める。

1. PR を取得し、以下を全て確認。1 つでも満たさなければ停止して報告
   - 人間による Approve が 1 件以上、Changes requested が 0 件
   - 必須 CI チェックが全て success
   - main との conflict が無い
   - PR 本文の受入条件チェックリストが全てチェック済み
2. squash merge (コミットタイトルは PR タイトル)
3. 関連 Issue のクローズと、`docs/features/<slug>/` のステータス更新が必要か確認
4. CD (`cd.yml`) の起動を確認して報告
