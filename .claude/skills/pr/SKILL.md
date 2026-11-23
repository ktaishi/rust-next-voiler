---
name: pr
description: 変更をコミット・push し、PR テンプレートに沿ったプルリクエストを作成する。
argument-hint: <NNNN-slug>
---

# /pr: PR 作成

対象: $ARGUMENTS

1. `make check` と `make test-integration` が green か確認 (未実行なら実行)
2. `git status` で意図しないファイル (.env, 結果ファイル, 生成物) が無いか確認
3. 未コミットがあれば Conventional Commits でコミットし、`git push -u origin <branch>`
4. `.github/pull_request_template.md` の見出しに沿って本文を作る
   - 概要 / 関連 (要件・設計ドキュメントへのリンク, `Closes #n`)
   - 受入条件チェックリスト: 各 AC と検証テスト名
   - テスト結果 (単体・結合・E2E・負荷の要約値)
   - レビュー観点 / リスク / ロールバック方法
5. GitHub MCP (`create_pull_request`) で PR を作成。タイトルは Conventional Commits 形式
6. PR URL を報告し、CI (`ci.yml`) の結果を確認する。失敗したら修正して push
