variable "environment" { type = string }
variable "bucket_prefix" { type = string }
variable "enable_versioning" { type = bool }
variable "log_bucket_name" { type = string }
variable "force_destroy" { type = bool }
variable "lifecycle_rule_enabled" { type = bool }
variable "transition_days_glacier" { type = number }
variable "expiration_days" { type = number }

resource "aws_s3_bucket" "app" {
  bucket        = "${var.bucket_prefix}-${var.environment}-app"
  force_destroy = var.force_destroy

  tags = {
    Name = "${var.environment}-app-bucket"
  }
}

resource "aws_s3_bucket_versioning" "app" {
  bucket = aws_s3_bucket.app.id
  versioning_configuration {
    status = var.enable_versioning ? "Enabled" : "Suspended"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "app" {
  bucket = aws_s3_bucket.app.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "app" {
  count  = var.lifecycle_rule_enabled ? 1 : 0
  bucket = aws_s3_bucket.app.id

  rule {
    id     = "transition-to-glacier"
    status = "Enabled"

    transition {
      days          = var.transition_days_glacier
      storage_class = "GLACIER"
    }

    expiration {
      days = var.expiration_days
    }

    filter {}
  }
}

resource "aws_s3_bucket" "logs" {
  bucket        = var.log_bucket_name
  force_destroy = false

  tags = {
    Name = "${var.environment}-logs"
  }
}

resource "aws_s3_bucket_logging" "app" {
  bucket = aws_s3_bucket.app.id
  target_bucket = aws_s3_bucket.logs.id
  target_prefix = "app-logs/"
}

output "app_bucket_id" { value = aws_s3_bucket.app.id }
output "app_bucket_arn" { value = aws_s3_bucket.app.arn }
output "log_bucket_id" { value = aws_s3_bucket.logs.id }
