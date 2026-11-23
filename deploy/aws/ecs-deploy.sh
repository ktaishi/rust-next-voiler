#!/usr/bin/env bash
# ECS (Fargate) へのデプロイ。CD (.github/workflows/cd.yml) から呼ばれる。手元からも実行可能。
#
#   1. backend / frontend / migrate / batch のタスク定義に新しいイメージを設定して新リビジョンを登録
#   2. migrate タスクを実行し、終了コード 0 を確認 (失敗したらここで中断し、サービスは更新しない)
#   3. backend → frontend の順にサービスを更新し、安定するまで待つ
#      (失敗時は ECS のデプロイサーキットブレーカーが自動ロールバック)
#
# 必須の環境変数:
#   ECS_CLUSTER         クラスタ名
#   ECS_NAME_PREFIX     タスク定義ファミリーの接頭辞 (例: voiler-staging)
#   ECS_SUBNETS         migrate タスクのサブネット (カンマ区切り)
#   ECS_SECURITY_GROUP  migrate タスクのセキュリティグループ
#   BACKEND_IMAGE       例: 123456789012.dkr.ecr.ap-northeast-1.amazonaws.com/voiler/backend:sha-abc1234
#   FRONTEND_IMAGE      同上 (frontend)
# 任意: DRY_RUN=1 なら登録・実行せずに差し替え後のタスク定義を表示する
set -euo pipefail

: "${ECS_CLUSTER:?}" "${ECS_NAME_PREFIX:?}" "${ECS_SUBNETS:?}" "${ECS_SECURITY_GROUP:?}"
: "${BACKEND_IMAGE:?}" "${FRONTEND_IMAGE:?}"
DRY_RUN="${DRY_RUN:-0}"

log() { printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*" >&2; }

# 現在のタスク定義を取得し、全コンテナのイメージを差し替えて新リビジョンを登録する。
# 標準出力に新しいタスク定義 ARN を返す。
register_revision() {
  local family="$1" image="$2" current next
  current=$(aws ecs describe-task-definition --task-definition "$family" --query taskDefinition --output json)
  next=$(jq --arg img "$image" '
      .containerDefinitions |= map(.image = $img)
      | del(.taskDefinitionArn, .revision, .status, .requiresAttributes, .compatibilities,
            .registeredAt, .registeredBy, .deregisteredAt)' <<<"$current")
  if [[ "$DRY_RUN" == "1" ]]; then
    log "DRY_RUN: $family -> $image"
    jq -c '.containerDefinitions | map({name, image})' <<<"$next" >&2
    echo "$family"
    return
  fi
  aws ecs register-task-definition --cli-input-json "$next" \
    --query taskDefinition.taskDefinitionArn --output text
}

log "registering task definitions"
backend_td=$(register_revision "${ECS_NAME_PREFIX}-backend" "$BACKEND_IMAGE")
migrate_td=$(register_revision "${ECS_NAME_PREFIX}-migrate" "$BACKEND_IMAGE")
batch_td=$(register_revision "${ECS_NAME_PREFIX}-batch" "$BACKEND_IMAGE")
frontend_td=$(register_revision "${ECS_NAME_PREFIX}-frontend" "$FRONTEND_IMAGE")
log "backend=$backend_td"
log "migrate=$migrate_td"
log "batch=$batch_td (次回のスケジュール実行から使われる)"
log "frontend=$frontend_td"
[[ "$DRY_RUN" == "1" ]] && { log "DRY_RUN: stop here"; exit 0; }

log "running migrations"
task_arn=$(aws ecs run-task \
  --cluster "$ECS_CLUSTER" \
  --task-definition "$migrate_td" \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[${ECS_SUBNETS}],securityGroups=[${ECS_SECURITY_GROUP}],assignPublicIp=DISABLED}" \
  --started-by "cd-migrate" \
  --query 'tasks[0].taskArn' --output text)
[[ -z "$task_arn" || "$task_arn" == "None" ]] && { log "failed to start migrate task"; exit 1; }
log "migrate task: $task_arn"
aws ecs wait tasks-stopped --cluster "$ECS_CLUSTER" --tasks "$task_arn"
exit_code=$(aws ecs describe-tasks --cluster "$ECS_CLUSTER" --tasks "$task_arn" \
  --query 'tasks[0].containers[0].exitCode' --output text)
if [[ "$exit_code" != "0" ]]; then
  reason=$(aws ecs describe-tasks --cluster "$ECS_CLUSTER" --tasks "$task_arn" \
    --query 'tasks[0].[stoppedReason, containers[0].reason]' --output text)
  log "migration failed: exitCode=$exit_code reason=$reason (ログ: /ecs/${ECS_NAME_PREFIX}/migrate)"
  exit 1
fi
log "migrations applied"

for svc in backend frontend; do
  td_var="${svc}_td"
  log "updating service $svc -> ${!td_var}"
  aws ecs update-service --cluster "$ECS_CLUSTER" --service "$svc" \
    --task-definition "${!td_var}" --query 'service.serviceName' --output text >/dev/null
  aws ecs wait services-stable --cluster "$ECS_CLUSTER" --services "$svc"
  log "$svc is stable"
done

log "deploy completed"
