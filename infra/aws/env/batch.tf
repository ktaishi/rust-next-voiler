# 定期バッチ: EventBridge Scheduler → ECS RunTask (/app/batch <args>)
# タスク定義はリビジョン無しの ARN を指定し、CD が登録した最新リビジョンで実行する。

resource "aws_scheduler_schedule" "batch" {
  for_each                     = var.batch_schedules
  name                         = "${local.name}-${each.key}"
  schedule_expression          = each.value.schedule
  schedule_expression_timezone = each.value.timezone
  state                        = each.value.enabled ? "ENABLED" : "DISABLED"

  flexible_time_window {
    mode = "OFF"
  }

  target {
    arn      = aws_ecs_cluster.this.arn
    role_arn = aws_iam_role.scheduler.arn

    ecs_parameters {
      task_definition_arn = aws_ecs_task_definition.batch.arn_without_revision
      launch_type         = "FARGATE"
      task_count          = 1
      network_configuration {
        subnets          = aws_subnet.private[*].id
        security_groups  = [aws_security_group.backend.id]
        assign_public_ip = false
      }
      tags = { BatchSchedule = each.key }
    }

    input = jsonencode({
      containerOverrides = [{ name = "batch", command = each.value.args }]
    })

    # 一時エラーはアプリ内で再試行し、失敗時は `batch resume` で再開する方針のため、ここでは再試行しない
    retry_policy {
      maximum_retry_attempts = 0
    }
  }
}
