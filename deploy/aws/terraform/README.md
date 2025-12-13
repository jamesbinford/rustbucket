# Rustbucket AWS Deployment with Terraform

This directory contains Terraform configuration to deploy Rustbucket on AWS EC2 with S3 logging.

## What This Creates

- **VPC**: Dedicated VPC for Rustbucket with public subnet
- **EC2 Instance**: Ubuntu instance running Rustbucket in Docker
- **S3 Bucket**: Log storage with lifecycle policies
- **IAM Role**: EC2 instance profile with S3 write permissions
- **Security Group**: Allows honeypot ports (22, 21, 25, 80, 53)
- **Elastic IP**: Static public IP for the honeypot
- **CloudWatch Logs**: Optional log streaming to CloudWatch

## Prerequisites

1. **AWS Account** with appropriate permissions
2. **AWS CLI** configured with credentials
3. **Terraform** installed (v1.0+)
4. **ChatGPT API Key** from OpenAI

## Quick Start

### 1. Configure AWS Credentials

```bash
aws configure
# Enter your AWS Access Key ID, Secret Access Key, and default region
```

### 2. Configure Terraform Variables

```bash
# Copy example variables
cp terraform.tfvars.example terraform.tfvars

# Edit with your values
nano terraform.tfvars
```

Required variables:
- `chatgpt_api_key`: Your OpenAI API key
- `s3_bucket_name`: Unique S3 bucket name for logs

### 3. Deploy

```bash
# Initialize Terraform
terraform init

# Review the plan
terraform plan

# Deploy!
terraform apply
```

When prompted, type `yes` to confirm.

### 4. Get Instance Information

```bash
# View all outputs
terraform output

# Get public IP
terraform output instance_public_ip

# Get S3 bucket name
terraform output s3_bucket_name
```

## Deployment Complete!

Your Rustbucket honeypot is now running at the public IP shown in the outputs.

**Services available:**
- SSH: `<public_ip>:22`
- FTP: `<public_ip>:21`
- SMTP: `<public_ip>:25`
- HTTP: `http://<public_ip>`

**Logs are being uploaded to:**
- S3 Bucket: (shown in terraform outputs)
- CloudWatch Logs: `/aws/ec2/rustbucket`

## Monitoring

### View Instance Status

```bash
# Get instance ID
INSTANCE_ID=$(terraform output -raw instance_id)

# Check instance status
aws ec2 describe-instance-status --instance-ids $INSTANCE_ID

# Connect to instance
ssh ubuntu@$(terraform output -raw instance_public_ip)
```

### View Logs

```bash
# View Docker container logs
ssh ubuntu@$(terraform output -raw instance_public_ip) \
  "docker logs -f rustbucket"

# View S3 logs
aws s3 ls s3://$(terraform output -raw s3_bucket_name)/ --recursive

# Download logs
aws s3 sync s3://$(terraform output -raw s3_bucket_name)/ ./logs/

# View CloudWatch logs
aws logs tail /aws/ec2/rustbucket --follow
```

### Check Health

```bash
# Check if container is running
ssh ubuntu@$(terraform output -raw instance_public_ip) \
  "docker ps | grep rustbucket"

# Check container health
ssh ubuntu@$(terraform output -raw instance_public_ip) \
  "docker inspect rustbucket | grep -A 10 Health"
```

## Scaling

### Deploy Multiple Instances

Deploy multiple honeypots across different regions:

```bash
# US East
cd deployments/us-east-1
terraform init
terraform apply -var="aws_region=us-east-1"

# EU West
cd ../eu-west-1
terraform init
terraform apply -var="aws_region=eu-west-1"

# Asia Pacific
cd ../ap-southeast-1
terraform init
terraform apply -var="aws_region=ap-southeast-1"
```

### Adjust Instance Size

Edit `terraform.tfvars`:

```hcl
# For light traffic
instance_type = "t3.micro"   # 1 vCPU, 1GB RAM

# For moderate traffic (recommended)
instance_type = "t3.small"   # 2 vCPU, 2GB RAM

# For heavy traffic
instance_type = "t3.medium"  # 2 vCPU, 4GB RAM
```

Then apply changes:
```bash
terraform apply
```

## Cost Estimation

**Monthly costs (us-east-1):**
- t3.small EC2: ~$15/month
- S3 Storage (1GB): ~$0.02/month
- Data Transfer (10GB out): ~$0.90/month
- **Total: ~$16-20/month**

Use AWS Cost Calculator for precise estimates: https://calculator.aws

## Security Notes

1. **Honeypot Exposure**: This instance is intentionally exposed to the internet on honeypot ports
2. **IAM Permissions**: Instance only has permission to write to its S3 bucket
3. **Encryption**: EBS volume is encrypted at rest
4. **Network Isolation**: Honeypot runs in dedicated VPC
5. **No SSH Access**: Real SSH access is blocked by Rustbucket running on port 22

## Customization

### Change Log Retention

Edit `terraform.tfvars`:
```hcl
log_retention_days = 180  # Keep logs for 6 months
```

### Disable S3 Logging

Edit `terraform.tfvars`:
```hcl
enable_s3_logging = false
delete_after_upload = false
```

### Custom Configuration

To use a custom `Config.toml`:

1. Create your config file
2. Add to `user-data.sh`:
   ```bash
   cat > /opt/rustbucket/Config.toml << 'CONFIG_EOF'
   # Your custom configuration
   CONFIG_EOF
   ```
3. Update docker-compose.yml to mount it:
   ```yaml
   volumes:
     - ./Config.toml:/app/Config.toml:ro
   ```

## Troubleshooting

### Instance Not Responding

```bash
# Check instance status
aws ec2 describe-instances \
  --instance-ids $(terraform output -raw instance_id) \
  --query 'Reservations[0].Instances[0].State.Name'

# View user-data logs
ssh ubuntu@$(terraform output -raw instance_public_ip) \
  "sudo cat /var/log/cloud-init-output.log"
```

### Container Not Running

```bash
# Connect to instance
ssh ubuntu@$(terraform output -raw instance_public_ip)

# Check Docker status
sudo systemctl status docker

# Check container
cd /opt/rustbucket
docker compose ps
docker compose logs

# Restart if needed
docker compose restart
```

### S3 Upload Issues

```bash
# Check IAM role
aws iam get-instance-profile \
  --instance-profile-name rustbucket-instance-profile

# Test S3 access from instance
ssh ubuntu@$(terraform output -raw instance_public_ip) \
  "aws s3 ls s3://$(terraform output -raw s3_bucket_name)/"
```

### High Costs

```bash
# Check CloudWatch metrics
aws cloudwatch get-metric-statistics \
  --namespace AWS/EC2 \
  --metric-name NetworkOut \
  --dimensions Name=InstanceId,Value=$(terraform output -raw instance_id) \
  --start-time $(date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%S) \
  --end-time $(date -u +%Y-%m-%dT%H:%M:%S) \
  --period 86400 \
  --statistics Sum

# Reduce instance size
# Edit terraform.tfvars and set instance_type = "t3.micro"
terraform apply
```

## Cleanup

To destroy all resources:

```bash
# Review what will be deleted
terraform plan -destroy

# Destroy (WARNING: This deletes everything including logs!)
terraform destroy

# If you want to keep S3 logs, manually export first:
aws s3 sync s3://$(terraform output -raw s3_bucket_name)/ ./backup-logs/
```

## Advanced

### Enable SSH Debugging Access

Add a separate security group rule for your IP only:

```hcl
# In main.tf, add to security group:
ingress {
  description = "Admin SSH"
  from_port   = 2222
  to_port     = 2222
  protocol    = "tcp"
  cidr_blocks = ["YOUR_IP/32"]  # Replace with your IP
}
```

Then modify user-data.sh to run SSH on port 2222.

### Auto-Scaling

For production deployments with auto-scaling:

```hcl
# Create Launch Template
resource "aws_launch_template" "rustbucket" {
  name_prefix   = "rustbucket-"
  image_id      = data.aws_ami.ubuntu.id
  instance_type = var.instance_type

  iam_instance_profile {
    name = aws_iam_instance_profile.rustbucket.name
  }

  vpc_security_group_ids = [aws_security_group.rustbucket.id]
  user_data = base64encode(templatefile("user-data.sh", {...}))
}

# Auto Scaling Group
resource "aws_autoscaling_group" "rustbucket" {
  min_size         = 1
  max_size         = 5
  desired_capacity = 1

  launch_template {
    id      = aws_launch_template.rustbucket.id
    version = "$Latest"
  }

  vpc_zone_identifier = [aws_subnet.public.id]
}
```

### Multi-Region Deployment

Create workspace per region:

```bash
# Create workspaces
terraform workspace new us-east-1
terraform workspace new eu-west-1
terraform workspace new ap-southeast-1

# Deploy to each region
terraform workspace select us-east-1
terraform apply -var="aws_region=us-east-1"

terraform workspace select eu-west-1
terraform apply -var="aws_region=eu-west-1"

terraform workspace select ap-southeast-1
terraform apply -var="aws_region=ap-southeast-1"
```

## Support

For issues or questions:
- GitHub Issues: https://github.com/jamesbinford/rustbucket/issues
- Documentation: https://github.com/jamesbinford/rustbucket
