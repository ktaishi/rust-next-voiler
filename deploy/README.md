# デプロイ

アプリは **特定のクラウドに依存しない「デプロイ契約」** を満たすコンテナとして作られている。
どのプラットフォームでも、この契約 (イメージ・環境変数・ポート・ヘルスチェック・一度きりのタスク・定期実行) を
そのプラットフォームの機能に対応付けるだけで動く。

| ディレクトリ | 対象 | 状態 |
|---|---|---|
| `../compose.yaml` | ローカル / 単一 VM | 動作確認済み (本番と同じ起動順) |
| `k8s/` | Kubernetes (EKS / GKE / AKS / オンプレ) | Kustomize の雛形 |
| `../infra/aws/` + `aws/` | **AWS ECS on Fargate** (Terraform + CD スクリプト) | 雛形 (要 `terraform plan` での確認) |

```mermaid
flowchart TB
    subgraph contract["デプロイ契約 (どこでも同じ)"]
        IMG["イメージ 2 つ<br/>backend (api + batch) / frontend"]
        ENV["環境変数で設定 (12-factor)"]
        HC["ヘルスチェック<br/>/healthz, /readyz, api healthcheck"]
        ONE["一度きりのタスク<br/>api migrate"]
        CRON["定期実行<br/>batch &lt;job&gt;"]
    end
    contract --> C["docker compose"]
    contract --> K["Kubernetes<br/>Deployment / Job / CronJob"]
    contract --> A["AWS ECS<br/>Service / RunTask / EventBridge Scheduler"]
    contract --> O["その他<br/>Cloud Run, Azure Container Apps ..."]
```

## 1. イメージ
| イメージ | 含まれるもの | 既定のコマンド | ポート | 実行ユーザー |
|---|---|---|---|---|
| `backend` | `/app/api`, `/app/batch` (distroless) | `/app/api` (= `api serve`) | 8080 | nonroot (65532) |
| `frontend` | Next.js standalone (`node server.js`) | `node server.js` | 3000 | node (1000) |

- タグは **コミット SHA** (`sha-<7桁>`)。同じイメージを staging → production へ昇格させる (環境ごとにビルドしない)
- レジストリは CD の設定で切り替える (既定 GHCR、AWS の場合 ECR)

## 2. プロセスの種類
| 種類 | イメージ / コマンド | 実行形態 | 外部公開 |
|---|---|---|---|
| **frontend** | `frontend` | 常駐 (2 台以上推奨) | ✅ ロードバランサの背後 |
| **backend** | `backend` / `api` | 常駐 (2 台以上推奨) | ❌ frontend からのみ到達 |
| **migrate** | `backend` / `api migrate` | **デプロイ前に 1 回**。成功 (exit 0) してから backend を更新 | — |
| **batch** | `backend` / `batch <job> ...` (entrypoint を `/app/batch` に) | スケジュール or 手動。exit 0 = 成功 | — |

起動順: `DB 準備完了 → migrate (exit 0) → backend (healthy) → frontend`

## 3. 環境変数
### backend (`api` / `batch` 共通)
| 変数 | 必須 | 既定 | 説明 |
|---|---|---|---|
| `DATABASE_URL` | どちらか | — | `mysql://user:pass@host:3306/db` (ローカル・CI 向け) |
| `DB_HOST` / `DB_PORT` / `DB_USER` / `DB_PASSWORD` / `DB_NAME` | どちらか | port 3306 | 個別指定 (**本番推奨**: `DB_PASSWORD` だけをシークレットから注入できる) |
| `DB_SSL_MODE` | | sqlx 既定 (preferred) | `disabled` / `preferred` / `required` / `verify_ca` / `verify_identity` |
| `DB_SSL_CA` | | — | CA 証明書ファイルのパス (`verify_*` 時) |
| `DB_MAX_CONNECTIONS` | | api 20 / batch 10 | プール上限。`台数 × 値` が DB の `max_connections` を超えないこと |
| `RUN_MIGRATIONS` | | `true` | **本番は `false`** (migrate タスクで実行) |
| `BIND_ADDR` | | `0.0.0.0:8080` | |
| `CORS_ALLOWED_ORIGINS` | | `http://localhost:3000` | カンマ区切り。frontend 経由のみなら実質不要 |
| `LOG_FORMAT` | | `pretty` | **本番は `json`** (イメージの既定は json) |
| `RUST_LOG` | | `info,api=debug,...` | ログレベル |

### frontend
| 変数 | 必須 | 既定 | 説明 |
|---|---|---|---|
| `API_ORIGIN` | ✅ | `http://backend:8080` (イメージ既定) | backend の到達先。**実行時に解決** (ビルド時に焼き込まない) |
| `PORT` / `HOSTNAME` | | `3000` / `0.0.0.0` | |

## 4. ヘルスチェック
| 対象 | 種類 | 方法 | 用途 |
|---|---|---|---|
| backend | liveness | `GET /healthz` または コンテナ内 `/app/api healthcheck` (exit 0) | プロセス生存。失敗したら再起動 |
| backend | readiness | `GET /readyz` (DB 疎通、失敗時 503) | トラフィックを流すか。**再起動の判定には使わない** (DB 障害で全台再起動を防ぐ) |
| frontend | liveness / readiness | `GET /healthz` | backend の状態は見ない |

## 5. 終了とシグナル
- `SIGTERM` で graceful shutdown (処理中のリクエストを完了してから終了)。猶予は 30 秒以上を推奨
- batch は SIGTERM で中断されても、チャンク単位のトランザクションと `resume` で再開できる

## 6. ログ
- すべて **標準出力** (JSON)。収集はプラットフォームに任せる (CloudWatch Logs, Cloud Logging, Loki など)
- backend はリクエストごとに `x-request-id` を付与・伝播する

## 7. プラットフォーム対応表
| 契約 | docker compose | Kubernetes | AWS ECS Fargate |
|---|---|---|---|
| frontend / backend 常駐 | `services` | `Deployment` + `Service` | ECS Service |
| backend の名前解決 (`http://backend:8080`) | サービス名 | `Service` 名 | **ECS Service Connect** |
| 外部公開 | `ports` | `Ingress` / LoadBalancer | ALB (+ CloudFront / WAF) |
| migrate | `service_completed_successfully` | `Job` (デプロイ前に実行) | `RunTask` (CD から実行し終了コード確認) |
| batch 定期実行 | `docker compose run` | `CronJob` | EventBridge Scheduler → `RunTask` |
| シークレット | `.env` | `Secret` | Secrets Manager (RDS 管理パスワード) |
| liveness | `healthcheck` | `livenessProbe` | コンテナヘルスチェック |
| readiness | `depends_on: service_healthy` | `readinessProbe` | ALB ターゲットグループのヘルスチェック |
| ログ | `docker compose logs` | stdout → 収集基盤 | awslogs → CloudWatch Logs |

## 8. CD (`.github/workflows/cd.yml`)
ビルドとデプロイを分離している。デプロイ先はリポジトリ変数 `DEPLOY_TARGET` で選ぶ。

| `DEPLOY_TARGET` | 動作 |
|---|---|
| 未設定 / `none` | イメージをビルドして GHCR に push するだけ |
| `aws-ecs` | ECR に push → `deploy/aws/ecs-deploy.sh` (migrate → backend/frontend 更新 → 安定待ち) |

新しいデプロイ先を足すときは `deploy/<target>/` にスクリプトを置き、`cd.yml` の deploy ジョブに分岐を追加する。
