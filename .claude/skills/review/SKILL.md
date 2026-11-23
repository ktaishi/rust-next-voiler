---
name: review
description: PR またはローカル差分をレビューチェックリストに沿ってレビューし、必要なら PR にコメントする。
argument-hint: [PR 番号] [--comment]
---

# /review: レビュー

対象: $ARGUMENTS (省略時は `git diff origin/main...HEAD`)

1. 差分と、関連する requirements.md / design.md を読む
2. `reviewer` サブエージェント (`.claude/agents/reviewer.md`) に差分と設計を渡して独立レビューさせる。
   セキュリティに関わる変更 (認証, 入力処理, SQL, 秘密情報) があれば `security-reviewer` も並行で起動
3. 指摘を重要度 (Must / Should / Nit) で整理し、誤検知を除外
4. `--comment` 指定時は GitHub MCP の pending review → 行コメント → submit (event: COMMENT) で投稿。
   **APPROVE はしない** (人間の役割)
5. Must 指摘があれば修正案を提示し、ユーザーの了承後に修正コミット
