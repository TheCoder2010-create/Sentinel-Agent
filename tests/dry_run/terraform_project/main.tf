# Realistic multi-module Terraform project for dry-run testing
# Simulates a production-grade AWS infrastructure deployment

terraform {
  required_version = ">= 1.6"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  backend "s3" {
    bucket = "my-company-tfstate"
    key    = "prod/networking/terraform.tfstate"
    region = "us-east-1"
  }
}

provider "aws" {
  region  = var.aws_region
  profile = var.aws_profile

  default_tags {
    tags = {
      Environment = var.environment
      ManagedBy   = "terraform"
      Project     = "platform-engineering"
    }
  }
}

# VPC and networking
module "networking" {
  source = "./modules/networking"

  environment      = var.environment
  vpc_cidr         = var.vpc_cidr
  availability_zones = var.availability_zones
  enable_nat_gateway = var.enable_nat_gateway
  enable_vpn_gateway = var.enable_vpn_gateway
  single_nat_gateway = var.single_nat_gateway
  enable_dns_hostnames = true
}

# RDS database
module "database" {
  source = "./modules/database"

  environment         = var.environment
  vpc_id              = module.networking.vpc_id
  subnet_ids          = module.networking.private_subnet_ids
  db_instance_class   = var.db_instance_class
  db_engine           = "postgres"
  db_engine_version   = "15.4"
  db_name             = var.db_name
  db_username         = var.db_username
  db_password         = var.db_password
  allocated_storage   = var.db_allocated_storage
  backup_retention_days = var.db_backup_retention_days
  multi_az            = var.environment == "production" ? true : false
  storage_encrypted   = true
  deletion_protection = var.environment == "production" ? true : false
}

# S3 buckets
module "storage" {
  source = "./modules/storage"

  environment           = var.environment
  bucket_prefix         = var.s3_bucket_prefix
  enable_versioning     = var.environment == "production" ? true : false
  log_bucket_name       = var.s3_log_bucket_name
  force_destroy         = var.environment != "production" ? true : false
  lifecycle_rule_enabled = true
  transition_days_glacier = 90
  expiration_days       = 365
}

# EC2 bastion host
resource "aws_instance" "bastion" {
  count = var.enable_bastion ? 1 : 0

  ami                    = data.aws_ami.amazon_linux_2.id
  instance_type          = var.bastion_instance_type
  subnet_id              = module.networking.public_subnet_ids[0]
  vpc_security_group_ids = [aws_security_group.bastion_sg[0].id]
  key_name               = var.bastion_key_name
  associate_public_ip_address = true

  root_block_device {
    volume_type = "gp3"
    volume_size = 30
    encrypted   = true
  }

  metadata_options {
    http_endpoint = "enabled"
    http_tokens   = "required"
  }

  user_data = <<-EOF
              #!/bin/bash
              yum update -y
              yum install -y postgresql15
              EOF

  tags = {
    Name = "${var.environment}-bastion"
  }
}

data "aws_ami" "amazon_linux_2" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["amzn2-ami-hvm-*-x86_64-gp2"]
  }
}

resource "aws_security_group" "bastion_sg" {
  count  = var.enable_bastion ? 1 : 0
  name   = "${var.environment}-bastion-sg"
  vpc_id = module.networking.vpc_id

  ingress {
    description = "SSH from corporate CIDR"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = var.corporate_cidr_blocks
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.environment}-bastion-sg"
  }
}
