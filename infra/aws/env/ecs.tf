resource "aws_ecs_cluster" "this" {
  name = local.name
  setting {
    name  = "containerInsights"
    value = "enabled"
  }
  service_connect_defaults {
    namespace = aws_service_discovery_http_namespace.this.arn
  }
}

# ECS Service Connect: frontend から http://backend:8080 で backend に到達できる
# (docker compose / k8s と同じ API_ORIGIN をそのまま使うため)
resource "aws_service_discovery_http_namespace" "this" {
  name = local.name
}

resource "aws_cloudwatch_log_group" "this" {
  for_each          = toset(["backend", "frontend", "migrate", "batch"])
  name              = "/ecs/${local.name}/${each.key}"
  retention_in_days = var.log_retention_days
}

locals {
  # backend / migrate / batch 共通の環境変数 (deploy/README.md の「デプロイ契約」)
  backend_environment = [
    { name = "DB_HOST", value = aws_db_instance.this.address },
    { name = "DB_PORT", value = tostring(aws_db_instance.this.port) },
    { name = "DB_USER", value = aws_db_instance.this.username },
    { name = "DB_NAME", value = aws_db_instance.this.db_name },
    { name = "DB_SSL_MODE", value = "required" },
    { name = "RUN_MIGRATIONS", value = "false" },
    { name = "LOG_FORMAT", value = "json" },
    { name = "RUST_LOG", value = "info" },
  ]
  backend_secrets = [
    { name = "DB_PASSWORD", valueFrom = "${aws_secretsmanager_secret.db.arn}:password::" },
  ]

  log_config = { for k, g in aws_cloudwatch_log_group.this : k => {
    logDriver = "awslogs"
    options   = {
      awslogs-group         = g.name
      awslogs-region        = var.region
      awslogs-stream-prefix = k
    }
  } }
}

# ---- タスク定義 ----
# イメージタグは初回のみ Terraform が設定し、以降は CD (deploy/aws/ecs-deploy.sh) が新リビジョンを登録する。

resource "aws_ecs_task_definition" "backend" {
  family                   = "${local.name}-backend"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.backend.cpu
  memory                   = var.backend.memory
  execution_role_arn       = aws_iam_role.execution.arn
  task_role_arn            = aws_iam_role.task.arn
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  container_definitions = jsonencode([{
    name         = "api"
    image        = "${local.ecr.backend}:${var.image_tag}"
    command      = ["serve"]
    essential    = true
    portMappings = [{ name = "http", containerPort = 8080, protocol = "tcp", appProtocol = "http" }]
    environment  = concat(local.backend_environment, [{ name = "DB_MAX_CONNECTIONS", value = tostring(var.backend.db_max_conns) }])
    secrets      = local.backend_secrets
    healthCheck  = {
      command     = ["CMD", "/app/api", "healthcheck"]
      interval    = 10
      timeout     = 5
      retries     = 3
      startPeriod = 10
    }
    stopTimeout      = 30
    logConfiguration = local.log_config["backend"]
  }])
}

resource "aws_ecs_task_definition" "frontend" {
  family                   = "${local.name}-frontend"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.frontend.cpu
  memory                   = var.frontend.memory
  execution_role_arn       = aws_iam_role.execution.arn
  task_role_arn            = aws_iam_role.task.arn
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  container_definitions = jsonencode([{
    name         = "frontend"
    image        = "${local.ecr.frontend}:${var.image_tag}"
    essential    = true
    portMappings = [{ name = "http", containerPort = 3000, protocol = "tcp", appProtocol = "http" }]
    environment  = [{ name = "API_ORIGIN", value = "http://backend:8080" }]
    healthCheck  = {
      command     = ["CMD", "node", "-e", "fetch('http://127.0.0.1:3000/healthz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"]
      interval    = 10
      timeout     = 5
      retries     = 3
      startPeriod = 15
    }
    stopTimeout      = 30
    logConfiguration = local.log_config["frontend"]
  }])
}

resource "aws_ecs_task_definition" "migrate" {
  family                   = "${local.name}-migrate"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = 256
  memory                   = 512
  execution_role_arn       = aws_iam_role.execution.arn
  task_role_arn            = aws_iam_role.task.arn
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  container_definitions = jsonencode([{
    name             = "migrate"
    image            = "${local.ecr.backend}:${var.image_tag}"
    command          = ["migrate"]
    essential        = true
    environment      = local.backend_environment
    secrets          = local.backend_secrets
    logConfiguration = local.log_config["migrate"]
  }])
}

resource "aws_ecs_task_definition" "batch" {
  family                   = "${local.name}-batch"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = var.batch.cpu
  memory                   = var.batch.memory
  execution_role_arn       = aws_iam_role.execution.arn
  task_role_arn            = aws_iam_role.task.arn
  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }
  container_definitions = jsonencode([{
    name             = "batch"
    image            = "${local.ecr.backend}:${var.image_tag}"
    entryPoint       = ["/app/batch"]
    command          = ["status"] # スケジュールごとに containerOverrides で上書き
    essential        = true
    environment      = concat(local.backend_environment, [{ name = "DB_MAX_CONNECTIONS", value = "10" }])
    secrets          = local.backend_secrets
    stopTimeout      = 120
    logConfiguration = local.log_config["batch"]
  }])
}

# ---- サービス ----
resource "aws_ecs_service" "backend" {
  name                   = "backend"
  cluster                = aws_ecs_cluster.this.id
  task_definition        = aws_ecs_task_definition.backend.arn
  desired_count          = var.backend.desired_count
  launch_type            = "FARGATE"
  propagate_tags         = "SERVICE"
  enable_execute_command = false

  network_configuration {
    subnets          = aws_subnet.private[*].id
    security_groups  = [aws_security_group.backend.id]
    assign_public_ip = false
  }

  service_connect_configuration {
    enabled   = true
    namespace = aws_service_discovery_http_namespace.this.arn
    service {
      port_name      = "http"
      discovery_name = "backend"
      client_alias {
        port     = 8080
        dns_name = "backend"
      }
    }
  }

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }
  deployment_minimum_healthy_percent = 100
  deployment_maximum_percent         = 200

  lifecycle {
    ignore_changes = [task_definition, desired_count] # CD とオートスケーリングが管理する
  }
}

resource "aws_ecs_service" "frontend" {
  name                              = "frontend"
  cluster                           = aws_ecs_cluster.this.id
  task_definition                   = aws_ecs_task_definition.frontend.arn
  desired_count                     = var.frontend.desired_count
  launch_type                       = "FARGATE"
  propagate_tags                    = "SERVICE"
  health_check_grace_period_seconds = 30

  network_configuration {
    subnets          = aws_subnet.private[*].id
    security_groups  = [aws_security_group.frontend.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.frontend.arn
    container_name   = "frontend"
    container_port   = 3000
  }

  # クライアントとして Service Connect に参加 (backend を名前解決するため)
  service_connect_configuration {
    enabled   = true
    namespace = aws_service_discovery_http_namespace.this.arn
  }

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }

  lifecycle {
    ignore_changes = [task_definition, desired_count]
  }

  depends_on = [aws_lb_listener.http]
}

# ---- オートスケーリング (CPU 使用率 60% を目標) ----
locals {
  scaling = {
    backend  = { service = aws_ecs_service.backend.name, min = var.backend.min_count, max = var.backend.max_count }
    frontend = { service = aws_ecs_service.frontend.name, min = var.frontend.min_count, max = var.frontend.max_count }
  }
}

resource "aws_appautoscaling_target" "this" {
  for_each           = local.scaling
  service_namespace  = "ecs"
  scalable_dimension = "ecs:service:DesiredCount"
  resource_id        = "service/${aws_ecs_cluster.this.name}/${each.value.service}"
  min_capacity       = each.value.min
  max_capacity       = each.value.max
}

resource "aws_appautoscaling_policy" "cpu" {
  for_each           = aws_appautoscaling_target.this
  name               = "${local.name}-${each.key}-cpu"
  policy_type        = "TargetTrackingScaling"
  service_namespace  = each.value.service_namespace
  scalable_dimension = each.value.scalable_dimension
  resource_id        = each.value.resource_id
  target_tracking_scaling_policy_configuration {
    target_value       = 60
    scale_in_cooldown  = 120
    scale_out_cooldown = 60
    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}
