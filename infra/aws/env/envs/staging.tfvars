environment       = "staging"
github_repository = "OWNER/REPO"
# shared スタックの出力 github_oidc_provider_arn
github_oidc_provider_arn = "arn:aws:iam::123456789012:oidc-provider/token.actions.githubusercontent.com"

nat_gateway_count      = 1
db_instance_class      = "db.t4g.micro"
db_multi_az            = false
db_deletion_protection = false

backend  = { cpu = 256, memory = 512, desired_count = 1, min_count = 1, max_count = 2, db_max_conns = 10 }
frontend = { cpu = 256, memory = 512, desired_count = 1, min_count = 1, max_count = 2 }
