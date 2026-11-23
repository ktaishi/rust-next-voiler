---
name: integration-test
description: 実 MySQL を使った結合テストと、docker compose フルスタックでの Playwright E2E を実行・追加する。
argument-hint: <NNNN-slug>
---

# /integration-test: 内部結合テスト

対象: docs/features/$ARGUMENTS

1. **Repository ↔ MySQL**: `backend/tests/*_test.rs` に `#![cfg(feature = "integration")]` と `#[sqlx::test(migrations = "./migrations")]` で追加。
   テストごとに使い捨て DB が作られるので相互依存させない
   - 実行: `make test-integration`
2. **Frontend ↔ API ↔ DB (E2E)**: `frontend/e2e/*.spec.ts` に AC ベースのシナリオを追加
   - リロード後も状態が保持されること (永続化) を確認する
   - 他テストのデータに依存しないよう、一意なタイトル (`Date.now()`) を使う
   - 実行: `make up && make e2e`
3. 失敗時は `docker compose logs backend` と Playwright trace (`frontend/test-results/`) を確認し原因を特定
4. 結果を報告。終わったら `make down`
