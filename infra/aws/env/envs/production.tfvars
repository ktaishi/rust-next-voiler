environment              = "production"
github_repository        = "OWNER/REPO"
github_oidc_provider_arn = "arn:aws:iam::123456789012:oidc-provider/token.actions.githubusercontent.com"

# certificate_arn = "arn:aws:acm:ap-northeast-1:123456789012:certificate/..."
# alarm_email     = "oncall@example.com"

nat_gateway_count        = 2
db_instance_class        = "db.t4g.medium"
db_multi_az              = true
db_deletion_protection   = true
db_backup_retention_days = 14

# backend: 台数 × db_max_conns が RDS の max_connections を超えないこと
backend  = { cpu = 512, memory = 1024, desired_count = 2, min_count = 2, max_count = 8, db_max_conns = 20 }
frontend = { cpu = 512, memory = 1024, desired_count = 2, min_count = 2, max_count = 8 }
