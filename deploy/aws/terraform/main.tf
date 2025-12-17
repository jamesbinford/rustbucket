# Rustbucket AWS Deployment with Terraform
# Creates an EC2 instance running Rustbucket with S3 logging

terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# Data source for latest Ubuntu AMI
data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"] # Canonical

  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd/ubuntu-jammy-22.04-amd64-server-*"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

# VPC for Rustbucket
resource "aws_vpc" "rustbucket" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name        = "rustbucket-vpc"
    Project     = "Rustbucket"
    Environment = var.environment
  }
}

# Internet Gateway
resource "aws_internet_gateway" "rustbucket" {
  vpc_id = aws_vpc.rustbucket.id

  tags = {
    Name    = "rustbucket-igw"
    Project = "Rustbucket"
  }
}

# Public Subnet
resource "aws_subnet" "public" {
  vpc_id                  = aws_vpc.rustbucket.id
  cidr_block              = "10.0.1.0/24"
  availability_zone       = data.aws_availability_zones.available.names[0]
  map_public_ip_on_launch = true

  tags = {
    Name    = "rustbucket-public-subnet"
    Project = "Rustbucket"
  }
}

data "aws_availability_zones" "available" {
  state = "available"
}

# Route Table
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.rustbucket.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.rustbucket.id
  }

  tags = {
    Name    = "rustbucket-public-rt"
    Project = "Rustbucket"
  }
}

resource "aws_route_table_association" "public" {
  subnet_id      = aws_subnet.public.id
  route_table_id = aws_route_table.public.id
}

# Security Group - Honeypot ports open to the world
resource "aws_security_group" "rustbucket" {
  name        = "rustbucket-sg"
  description = "Security group for Rustbucket honeypot"
  vpc_id      = aws_vpc.rustbucket.id

  # SSH Honeypot
  ingress {
    description = "SSH Honeypot"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # FTP Honeypot
  ingress {
    description = "FTP Honeypot"
    from_port   = 21
    to_port     = 21
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # SMTP Honeypot
  ingress {
    description = "SMTP Honeypot"
    from_port   = 25
    to_port     = 25
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # HTTP Honeypot
  ingress {
    description = "HTTP Honeypot"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # DNS Honeypot (future)
  ingress {
    description = "DNS Honeypot"
    from_port   = 53
    to_port     = 53
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    description = "DNS Honeypot UDP"
    from_port   = 53
    to_port     = 53
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Allow all outbound traffic
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name    = "rustbucket-sg"
    Project = "Rustbucket"
  }
}

# S3 Bucket for logs
resource "aws_s3_bucket" "logs" {
  bucket = var.s3_bucket_name

  tags = {
    Name    = "rustbucket-logs"
    Project = "Rustbucket"
  }
}

resource "aws_s3_bucket_versioning" "logs" {
  bucket = aws_s3_bucket.logs.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "logs" {
  bucket = aws_s3_bucket.logs.id

  rule {
    id     = "delete-old-logs"
    status = "Enabled"

    filter {
      prefix = ""
    }

    expiration {
      days = var.log_retention_days
    }

    noncurrent_version_expiration {
      noncurrent_days = 7
    }
  }
}

# NOTE: IAM Role (rustbucket-ec2-role), Policy (rustbucket-s3-access), and
# Instance Profile (rustbucket-instance-profile) are managed outside Terraform
# due to IAM permission constraints. They were created via AWS CLI.
#
# To recreate manually:
#   aws iam create-role --role-name rustbucket-ec2-role --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},"Action":"sts:AssumeRole"}]}'
#   aws iam put-role-policy --role-name rustbucket-ec2-role --policy-name rustbucket-s3-access --policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":["s3:PutObject","s3:PutObjectAcl"],"Resource":"arn:aws:s3:::BUCKET_NAME/*"}]}'
#   aws iam create-instance-profile --instance-profile-name rustbucket-instance-profile
#   aws iam add-role-to-instance-profile --instance-profile-name rustbucket-instance-profile --role-name rustbucket-ec2-role

# EC2 Instance
resource "aws_instance" "rustbucket" {
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.instance_type
  subnet_id              = aws_subnet.public.id
  vpc_security_group_ids = [aws_security_group.rustbucket.id]
  iam_instance_profile   = "rustbucket-instance-profile"  # Managed outside Terraform

  root_block_device {
    volume_size = 20
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/user-data.sh", {
    chatgpt_api_key     = var.chatgpt_api_key
    s3_bucket_name      = aws_s3_bucket.logs.id
    s3_region           = var.aws_region
    enable_s3           = var.enable_s3_logging
    delete_after_upload = var.delete_after_upload
  })

  tags = {
    Name        = "rustbucket-${var.environment}"
    Project     = "Rustbucket"
    Environment = var.environment
  }

  lifecycle {
    ignore_changes = [ami, iam_instance_profile]
  }
}

# Elastic IP
resource "aws_eip" "rustbucket" {
  instance = aws_instance.rustbucket.id
  domain   = "vpc"

  tags = {
    Name    = "rustbucket-eip"
    Project = "Rustbucket"
  }
}
