---
name: test
description: 受入条件の網羅性を確認し、不足する単体・HTTP・コンポーネントテストを追加して実行する。
argument-hint: <NNNN-slug>
---

# /test: テスト

対象: docs/features/$ARGUMENTS

1. design.md のトレーサビリティ表と実際のテストを突き合わせる: `grep -rn "AC-" backend/src backend/tests frontend/src frontend/e2e`
2. 不足を追加する。観点:
   - 正常系 / 異常系 / 境界値 (0, 1, max, max+1) / 存在しない ID / 不正形式 (UUID, JSON)
   - Backend: domain は `#[cfg(test)]` 単体、HTTP は `backend/tests/api_test.rs` 形式 (InMemory repo)
   - Frontend: Vitest + Testing Library。fetch は `vi.stubGlobal` でモック
3. `make test-unit` を実行。失敗したら実装を直す (テストを弱めない)
4. 必要ならカバレッジ: `cd frontend && npx vitest run --coverage`
5. AC ごとの検証テスト一覧を報告
