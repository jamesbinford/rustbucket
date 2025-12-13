# AWS Region
variable "aws_region" {
  description = "AWS region to deploy resources"
  type        = string
  default     = "us-east-1"
}

# Environment
variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "prod"
}

# Instance Configuration
variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t3.small"  # 2 vCPU, 2GB RAM - good for moderate traffic
}

# ChatGPT Configuration
variable "chatgpt_api_key" {
  description = "OpenAI ChatGPT API key"
  type        = string
  sensitive   = true
}

# S3 Configuration
variable "s3_bucket_name" {
  description = "S3 bucket name for log storage (must be globally unique)"
  type        = string
}

variable "enable_s3_logging" {
  description = "Enable S3 log uploading"
  type        = bool
  default     = true
}

variable "delete_after_upload" {
  description = "Delete local logs after uploading to S3"
  type        = bool
  default     = true
}

variable "log_retention_days" {
  description = "Number of days to retain logs in S3"
  type        = number
  default     = 90
}
