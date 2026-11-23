variable "project" {
  type    = string
  default = "voiler"
}

variable "environment" {
  description = "staging / production"
  type        = string
}

variable "region" {
  type    = string
  default = "ap-northeast-1"
}

variable "github_repository" {
  description = "owner/repo。デプロイ用ロールを引き受けられるリポジトリ"
  type        = string
}

variable "github_oidc_provider_arn" {
  description = "shared スタックの出力 github_oidc_provider_arn"
  type        = string
}

variable "image_tag" {
  description = "初回作成時のイメージタグ。以降は CD がタスク定義を更新する (Terraform は追従しない)"
  type        = string
  default     = "latest"
}

# ---- ネットワーク ----
variable "vpc_cidr" {
  type    = string
  default = "10.0.0.0/16"
}

variable "az_count" {
  type    = number
  default = 2
}

variable "nat_gateway_count" {
  description = "1 なら全 AZ で共有 (安価)、az_count と同じなら AZ ごと (高可用)"
  type        = number
  default     = 1
}

# ---- 公開 ----
variable "certificate_arn" {
  description = "ACM 証明書の ARN。空なら HTTP のみ (検証用)"
  type        = string
  default     = ""
}

variable "alb_ingress_cidrs" {
  type    = list(string)
  default = ["0.0.0.0/0"]
}

# ---- DB ----
variable "db_instance_class" {
  type    = string
  default = "db.t4g.micro"
}

variable "db_allocated_storage" {
  type    = number
  default = 20
}

variable "db_max_allocated_storage" {
  description = "ストレージ自動拡張の上限 (GB)"
  type        = number
  default     = 100
}

variable "db_multi_az" {
  type    = bool
  default = false
}

variable "db_deletion_protection" {
  type    = bool
  default = true
}

variable "db_backup_retention_days" {
  type    = number
  default = 7
}

# ---- ECS ----
variable "backend" {
  type = object({
    cpu           = number
    memory        = number
    desired_count = number
    min_count     = number
    max_count     = number
    db_max_conns  = number
  })
  default = { cpu = 256, memory = 512, desired_count = 2, min_count = 2, max_count = 6, db_max_conns = 20 }
}

variable "frontend" {
  type = object({
    cpu           = number
    memory        = number
    desired_count = number
    min_count     = number
    max_count     = number
  })
  default = { cpu = 256, memory = 512, desired_count = 2, min_count = 2, max_count = 6 }
}

variable "batch" {
  type = object({
    cpu    = number
    memory = number
  })
  default = { cpu = 512, memory = 1024 }
}

variable "batch_schedules" {
  description = "定期バッチ。key = スケジュール名, schedule = EventBridge Scheduler の式 (timezone で解釈), args = /app/batch の引数"
  type        = map(object({
    schedule = string
    timezone = optional(string, "Asia/Tokyo")
    args     = list(string)
    enabled  = optional(bool, true)
  }))
  default = {
    archive-completed = {
      schedule = "cron(17 3 * * ? *)"
      args     = ["archive-completed", "--older-than-days", "30", "--chunk-size", "1000"]
    }
  }
}

variable "log_retention_days" {
  type    = number
  default = 30
}

variable "alarm_email" {
  description = "アラーム通知先メール。空なら購読を作らない"
  type        = string
  default     = ""
}
