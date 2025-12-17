# Instance Information
output "instance_id" {
  description = "EC2 Instance ID"
  value       = aws_instance.rustbucket.id
}

output "instance_public_ip" {
  description = "Public IP address of the Rustbucket instance"
  value       = aws_eip.rustbucket.public_ip
}

output "instance_public_dns" {
  description = "Public DNS name of the Rustbucket instance"
  value       = aws_instance.rustbucket.public_dns
}

# S3 Bucket Information
output "s3_bucket_name" {
  description = "S3 bucket name for logs"
  value       = aws_s3_bucket.logs.id
}

output "s3_bucket_arn" {
  description = "S3 bucket ARN"
  value       = aws_s3_bucket.logs.arn
}

# Honeypot Endpoints
output "honeypot_endpoints" {
  description = "Honeypot service endpoints"
  value = {
    ssh  = "${aws_eip.rustbucket.public_ip}:22"
    ftp  = "${aws_eip.rustbucket.public_ip}:21"
    smtp = "${aws_eip.rustbucket.public_ip}:25"
    http = "http://${aws_eip.rustbucket.public_ip}"
  }
}

# Log Access
output "view_logs_command" {
  description = "AWS CLI command to view logs in S3"
  value       = "aws s3 ls s3://${aws_s3_bucket.logs.id}/ --recursive"
}
