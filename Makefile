# 開発の入口。AI エージェントも人間もこのターゲット経由で操作する。
SHELL := /bin/bash
.DEFAULT_GOAL := help
COMPOSE := docker compose
DB_URL_TEST ?= mysql://root:root@127.0.0.1:3306/app
K6_IMAGE := grafana/k6:2.4.0
PROFILE ?= smoke
BASE_URL ?= http://localhost:8080

.PHONY: help
help: ## ターゲット一覧
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-18s\033[0m %s\n",$$1,$$2}'

# ---- セットアップ / 起動 ----
.PHONY: setup up down clean logs db-up dev-backend dev-frontend
setup: ## 依存インストール
	cd frontend && npm ci
	cd backend && cargo fetch
up: ## フルスタック起動 (mysql/backend/frontend)
	$(COMPOSE) up -d --build --wait
down: ## 停止 (データ保持)
	$(COMPOSE) down
clean: ## 停止 + DB ボリューム削除
	$(COMPOSE) down --volumes
logs: ## ログ追跡
	$(COMPOSE) logs -f
db-up: ## MySQL のみ起動 (ローカル開発・内部結合テスト用)
	$(COMPOSE) up -d --wait mysql
dev-backend: db-up ## backend をホストで起動
	cd backend && DATABASE_URL=mysql://app:app@127.0.0.1:3306/app cargo run
dev-frontend: ## frontend を dev モードで起動
	cd frontend && npm run dev

# ---- 品質ゲート ----
.PHONY: fmt lint typecheck test test-unit test-integration e2e loadtest check ci
fmt: ## 自動整形
	cd backend && cargo fmt
	cd frontend && npm run format
lint: ## 静的解析
	cd backend && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings
	cd frontend && npm run lint && npm run format:check
typecheck: ## 型検査
	cd frontend && npm run typecheck
test-unit: ## 単体テスト (DB 不要)
	cd backend && cargo test
	cd frontend && npm test
test-integration: db-up ## 内部結合テスト (実 MySQL)
	cd backend && DATABASE_URL=$(DB_URL_TEST) cargo test --features integration
test: test-unit test-integration ## 単体 + 内部結合
e2e: ## E2E (make up 済み前提)
	cd frontend && npx playwright test
loadtest: ## 負荷試験 PROFILE=smoke|load|stress|spike (make up 済み前提)
	docker run --rm --network host -u $$(id -u) -v $(CURDIR):/work -w /work \
		-e PROFILE=$(PROFILE) -e BASE_URL=$(BASE_URL) $(K6_IMAGE) run loadtest/scenarios/todos.js
check: lint typecheck test-unit ## コミット前の高速チェック
ci: lint typecheck test ## CI 相当 (E2E/負荷除く)
