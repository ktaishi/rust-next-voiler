# AI 駆動開発ワークフロー

要件定義からマージまでを **AI (Claude Code) が実行し、人間はゲートで判断する** 前提のプロセス。
各フェーズは `.claude/skills/` のスキル (スラッシュコマンド) に対応し、成果物はすべてリポジトリに残す。

```
 /requirements ─▶ /design ─▶ /implement ─▶ /test ─▶ /integration-test ─▶ /load-test ─▶ /pr ─▶ /review ─▶ merge
      │ G1            │ G2                                                                   │ G3        │ G4
   人間承認        人間承認                                                              CI green   人間承認
```

`/feature <説明>` は上記を順に実行するオーケストレータ。ゲート (G1〜G4) では必ず停止して人間の承認を待つ。

## フェーズ定義

| # | フェーズ | スキル | 入力 | 成果物 | 完了条件 (DoD) |
|---|---|---|---|---|---|
| 1 | 要件定義 | `/requirements` | 依頼文・Issue | `docs/features/NNNN-slug/requirements.md` | 受入条件が Given/When/Then で検証可能。未決事項が 0 か、明示的に保留 |
| G1 | ゲート | — | requirements.md | 承認コメント | 人間が「承認」と返答 |
| 2 | 設計 | `/design` | requirements.md | `design.md`, `docs/api/openapi.yaml` 差分, migration 案, 必要なら ADR | 受入条件 → API/DB/画面/テストへのトレーサビリティ表が埋まっている |
| G2 | ゲート | — | design.md | 承認コメント | 人間が承認 |
| 3 | 実装 | `/implement` | design.md | コード + 単体テスト | `make check` が green。テスト先行 (TDD) で書く |
| 4 | テスト | `/test` | 実装 | 単体/HTTP/コンポーネントテストの追加 | 受入条件ごとに最低 1 テスト。境界値・異常系を網羅 |
| 5 | 内部結合 | `/integration-test` | 実装 | MySQL 結合テスト, Playwright E2E | `make test-integration` と `make up && make e2e` が green |
| 6 | 負荷試験 | `/load-test` | 実装 | `docs/features/NNNN-slug/load-test-report.md` | SLO (下記) を満たす。満たさない場合は原因と対策を記載 |
| 7 | PR 作成 | `/pr` | 全成果物 | GitHub PR (テンプレート準拠) | ブランチ push 済み、PR 本文に受入条件チェックリスト |
| G3 | ゲート | — | PR | CI 結果 | CI (`ci.yml`) 全ジョブ green |
| 8 | レビュー | `/review` | PR | レビューコメント | 指摘が全て解消 or 合意の上で別 Issue 化 |
| G4 | ゲート | — | PR | Approve | 人間 1 名以上の Approve |
| 9 | マージ | `/merge` | 承認済み PR | main へ squash merge | CD (`cd.yml`) がイメージを公開 |

## 品質基準 (SLO / 閾値)

| 指標 | 閾値 | 定義元 |
|---|---|---|
| API p95 レイテンシ | < 200ms | `loadtest/lib/config.js` |
| API p99 レイテンシ | < 500ms | 同上 |
| エラー率 | < 1% | 同上 |
| Lint/型 | 警告 0 (`clippy -D warnings`, `eslint`, `tsc`) | `Makefile` |

## 人間と AI の役割分担

- **人間**: 何を作るか決める / ゲートで承認する / 本番リリース判断 / 秘密情報の管理
- **AI**: 文書化、設計案 (代替案とトレードオフ付き)、実装、テスト、計測、PR 作成、一次レビュー
- AI は **ゲートを自分で通過させない**。承認なしに次フェーズへ進まない。
- AI は不明点を推測で埋めず、`requirements.md` の「未決事項」に書いて質問する。

## ブランチ・コミット規約

- ブランチ: `feat/NNNN-slug`, `fix/NNNN-slug`, `chore/...`
- コミット: [Conventional Commits](https://www.conventionalcommits.org/ja/) (`feat(api): add todo search`)
- マージ: squash merge のみ。main への直接 push 禁止 (branch protection)

## トレーサビリティ

受入条件 ID (`AC-1` など) を、設計表・テスト名・PR チェックリストで共通に使う。
テスト名やテストコメントに `AC-n` を含めると、`grep -rn "AC-" backend frontend` で網羅性を確認できる。
