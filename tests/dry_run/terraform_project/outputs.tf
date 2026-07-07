output "vpc_id" {
  description = "The ID of the VPC"
  value       = module.networking.vpc_id
}

output "vpc_cidr_block" {
  description = "The CIDR block of the VPC"
  value       = module.networking.vpc_cidr
}

output "public_subnet_ids" {
  description = "List of public subnet IDs"
  value       = module.networking.public_subnet_ids
}

output "private_subnet_ids" {
  description = "List of private subnet IDs"
  value       = module.networking.private_subnet_ids
}

output "database_subnet_ids" {
  description = "List of database subnet IDs"
  value       = module.networking.database_subnet_ids
}

output "rds_endpoint" {
  description = "The connection endpoint for the RDS instance"
  value       = module.database.endpoint
  sensitive   = true
}

output "rds_arn" {
  description = "ARN of the RDS instance"
  value       = module.database.arn
}

output "app_bucket_arn" {
  description = "ARN of the application S3 bucket"
  value       = module.storage.app_bucket_arn
}

output "bastion_public_ip" {
  description = "Public IP of the bastion host"
  value       = var.enable_bastion ? aws_instance.bastion[0].public_ip : null
}
