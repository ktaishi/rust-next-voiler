# GitHub リポジトリ初期設定

ボイラープレートを使い始めるときに 1 度だけ行う設定。

## 1. ブランチ保護 (Settings → Rules → Rulesets → main)
- Require a pull request before merging — Required approvals: **1**、Dismiss stale approvals
- Require status checks to pass:
  - `backend (fmt / clippy / unit / integration)`
  - `frontend (lint / typecheck / unit / build)`
  - `e2e + load smoke (docker compose)`
- Require conversation resolution before merging
- Block force pushes
- Allowed merge methods: **Squash** のみ (Settings → General → Pull Requests)
- 任意: Allow auto-merge を有効化 → Approve + CI green で自動マージ

## 2. Claude 連携
1. Claude GitHub App をインストール: https://github.com/apps/claude
2. Secrets に `CLAUDE_CODE_OAUTH_TOKEN` を登録 (ローカルで `claude setup-token` で発行)。
   API キー課金の場合は `ANTHROPIC_API_KEY` を登録し、ワークフローの該当行を切り替える
3. 動作: PR 作成で `claude-review.yml` が一次レビュー、コメントで `@claude` と書くと `claude.yml` が応答

## 3. Environments (Settings → Environments)
- `staging`: main への merge で自動デプロイ
- `production`: `v*` タグで起動。**Required reviewers** を設定して手動承認にする
- Variables: `API_ORIGIN` (frontend からの backend 到達先。既定 `http://backend:8080`)

## 4. パッケージ
CD は `ghcr.io/<owner>/<repo>/{backend,frontend}` にイメージを push する。

## 5. CODEOWNERS
`.github/CODEOWNERS` のレビュアーをチームに合わせて変更する。
