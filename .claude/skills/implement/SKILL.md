---
name: implement
description: 承認済み設計に従い TDD でバックエンド・フロントエンドを実装する。
argument-hint: <NNNN-slug>
---

# /implement: 実装

対象: docs/features/$ARGUMENTS/design.md

前提: design.md が `Approved`。

1. 作業ブランチ `feat/$ARGUMENTS` を作成 (既にあればそれを使う)
2. CLAUDE.md の「新しいリソースを追加するとき」の順で、**層ごとに Red → Green → Refactor**
   - まず失敗するテストを書き、`cargo test` / `npm test` で失敗を確認してから実装
   - テスト名かコメントに `AC-n` を入れる
3. バックエンド完了ごとに `cd backend && cargo clippy --all-targets --all-features -- -D warnings`
4. API 型は `docs/api/openapi.yaml` ↔ serde 構造体 ↔ `frontend/src/lib/api.ts` を一致させる
5. 最後に `make check` が green であることを確認
6. 論理的な単位で Conventional Commits のコミットを作る (例: `feat(api): add users endpoint`)
7. 変更ファイルと残作業を報告
