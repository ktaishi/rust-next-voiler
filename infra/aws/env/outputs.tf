output "app_url" {
  value = var.certificate_arn == "" ? "http://${aws_lb.this.dns_name}" : "https://${aws_lb.this.dns_name}"
}

output "alb_dns_name" {
  description = "Route 53 のエイリアスレコードの向き先"
  value       = aws_lb.this.dns_name
}

output "db_endpoint" {
  value = aws_db_instance.this.address
}

# ---- GitHub Environment (staging / production) の Variables に設定する値 ----
output "github_environment_variables" {
  value = {
    AWS_DEPLOY_ROLE_ARN = aws_iam_role.deploy.arn
    ECS_CLUSTER         = aws_ecs_cluster.this.name
    ECS_NAME_PREFIX     = local.name
    ECS_SUBNETS         = join(",", aws_subnet.private[*].id)
    ECS_SECURITY_GROUP  = aws_security_group.backend.id
  }
}
