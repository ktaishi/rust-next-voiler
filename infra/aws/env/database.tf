# RDS for MySQL 8.4 + 接続情報のシークレット。
#
# パスワードは random_password で生成し、Secrets Manager に保存して ECS が DB_PASSWORD として注入する。
# (RDS 管理パスワード (manage_master_user_password) は自動ローテーションされ、起動済みタスクの接続が
#  切れるため使わない。パスワードは state に含まれるので、state の S3 バケットは暗号化・アクセス制限すること)
#
# TODO(運用): アプリ用に権限を絞った DB ユーザーを作り、マスターユーザーは管理作業専用にする

resource "aws_db_subnet_group" "this" {
  name       = local.name
  subnet_ids = aws_subnet.private[*].id
}

resource "aws_db_parameter_group" "this" {
  name   = local.name
  family = "mysql8.4"

  parameter {
    name  = "require_secure_transport"
    value = "ON" # アプリは DB_SSL_MODE=required で接続
  }
  parameter {
    name  = "time_zone"
    value = "UTC"
  }
  parameter {
    name  = "character_set_server"
    value = "utf8mb4"
  }
  parameter {
    name  = "collation_server"
    value = "utf8mb4_0900_ai_ci"
  }
  parameter {
    name  = "slow_query_log"
    value = "1"
  }
  parameter {
    name  = "long_query_time"
    value = "0.5"
  }
}

resource "random_password" "db" {
  length           = 32
  special          = true
  override_special = "!#%*()-_=+[]{}<>:?" # RDS で使えない / @ " と空白を除外
}

resource "aws_db_instance" "this" {
  identifier     = local.name
  engine         = "mysql"
  engine_version = "8.4"
  instance_class = var.db_instance_class

  db_name  = "app"
  username = "app"
  password = random_password.db.result

  allocated_storage     = var.db_allocated_storage
  max_allocated_storage = var.db_max_allocated_storage
  storage_type          = "gp3"
  storage_encrypted     = true

  multi_az               = var.db_multi_az
  db_subnet_group_name   = aws_db_subnet_group.this.name
  vpc_security_group_ids = [aws_security_group.db.id]
  parameter_group_name   = aws_db_parameter_group.this.name
  publicly_accessible    = false

  backup_retention_period         = var.db_backup_retention_days
  backup_window                   = "17:00-18:00" # UTC (= 02:00-03:00 JST)
  maintenance_window              = "sun:18:30-sun:19:30"
  auto_minor_version_upgrade      = true
  enabled_cloudwatch_logs_exports = ["error", "slowquery"]

  deletion_protection       = var.db_deletion_protection
  skip_final_snapshot       = !var.db_deletion_protection
  final_snapshot_identifier = "${local.name}-final"
  copy_tags_to_snapshot     = true
}

resource "aws_secretsmanager_secret" "db" {
  name                    = "${local.name}/db"
  description             = "DB connection for ${local.name} (keys: host, port, username, password, dbname)"
  recovery_window_in_days = var.environment == "production" ? 30 : 0
}

resource "aws_secretsmanager_secret_version" "db" {
  secret_id     = aws_secretsmanager_secret.db.id
  secret_string = jsonencode({
    host     = aws_db_instance.this.address
    port     = aws_db_instance.this.port
    username = aws_db_instance.this.username
    password = random_password.db.result
    dbname   = aws_db_instance.this.db_name
  })
}
