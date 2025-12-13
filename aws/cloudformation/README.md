# Rustbucket AWS Deployment with CloudFormation

This directory contains a CloudFormation template to deploy Rustbucket on AWS ECS Fargate with S3 logging.

## What This Creates

- **VPC**: Dedicated VPC (10.0.0.0/16) with public subnet
- **ECS Fargate Service**: Serverless container deployment (no EC2 instances to manage)
- **S3 Bucket**: Log storage with versioning and 90-day lifecycle policy
- **IAM Roles**: Task execution role and S3 write permissions
- **Security Group**: Allows honeypot ports (22, 21, 25, 80)
- **CloudWatch Logs**: Container log streaming

## Why ECS Fargate?

**Advantages over EC2:**
- ✅ No server management (serverless containers)
- ✅ Automatic scaling and health checks
- ✅ Pay only for container runtime (not idle EC2)
- ✅ Built-in high availability
- ✅ Automatic OS patching and updates

**Use EC2 (Terraform) if:**
- You need more control over the host
- You want to run multiple containers on one instance
- You need custom kernel modules or system configuration

## Prerequisites

1. **AWS Account** with appropriate permissions
2. **AWS CLI** configured with credentials
3. **ChatGPT API Key** from OpenAI

## Quick Start

### 1. Configure AWS Credentials

```bash
aws configure
# Enter your AWS Access Key ID, Secret Access Key, and default region
```

### 2. Create Parameters File

```bash
# Copy example parameters
cp parameters.json.example parameters.json

# Edit with your values
nano parameters.json
```

Required parameters:
- `ChatGPTAPIKey`: Your OpenAI API key
- `S3BucketName`: Unique S3 bucket name for logs (globally unique)

### 3. Deploy Stack

```bash
# Validate template
aws cloudformation validate-template \
  --template-body file://rustbucket-ecs.yaml

# Create stack
aws cloudformation create-stack \
  --stack-name rustbucket-honeypot \
  --template-body file://rustbucket-ecs.yaml \
  --parameters file://parameters.json \
  --capabilities CAPABILITY_IAM

# Monitor stack creation
aws cloudformation wait stack-create-complete \
  --stack-name rustbucket-honeypot

# View outputs
aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs'
```

### 4. Get Task Public IP

```bash
# Get the cluster name
CLUSTER=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`ClusterName`].OutputValue' \
  --output text)

# Get the service name
SERVICE=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`ServiceName`].OutputValue' \
  --output text)

# Get task ARN
TASK_ARN=$(aws ecs list-tasks \
  --cluster $CLUSTER \
  --service-name $SERVICE \
  --query 'taskArns[0]' \
  --output text)

# Get network interface ID
ENI=$(aws ecs describe-tasks \
  --cluster $CLUSTER \
  --tasks $TASK_ARN \
  --query 'tasks[0].attachments[0].details[?name==`networkInterfaceId`].value' \
  --output text)

# Get public IP
aws ec2 describe-network-interfaces \
  --network-interface-ids $ENI \
  --query 'NetworkInterfaces[0].Association.PublicIp' \
  --output text
```

Or use the command from stack outputs:

```bash
# Get the IP command from outputs
aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`GetPublicIPCommand`].OutputValue' \
  --output text
```

## Deployment Complete!

Your Rustbucket honeypot is now running on ECS Fargate.

**Services available:**
- SSH: `<public_ip>:22`
- FTP: `<public_ip>:21`
- SMTP: `<public_ip>:25`
- HTTP: `http://<public_ip>`

**Logs are being uploaded to:**
- S3 Bucket: (check stack outputs)
- CloudWatch Logs: `/ecs/rustbucket`

## Parameters Reference

### Required Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `ChatGPTAPIKey` | OpenAI API key | `sk-proj-...` |
| `S3BucketName` | Globally unique S3 bucket name | `rustbucket-logs-myname-123` |

### Optional Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `EnableS3Logging` | `true` | Enable S3 log uploading |
| `DeleteAfterUpload` | `true` | Delete local logs after S3 upload |
| `FargateTaskCPU` | `512` | Task CPU units (256, 512, 1024, 2048) |
| `FargateTaskMemory` | `1024` | Task memory in MB (512, 1024, 2048, 4096) |

**CPU and Memory Combinations:**

Valid Fargate combinations:
- 256 CPU: 512-2048 MB memory
- 512 CPU: 1024-4096 MB memory
- 1024 CPU: 2048-8192 MB memory
- 2048 CPU: 4096-16384 MB memory

## Monitoring

### View Container Logs

```bash
# Get log group from outputs
LOG_GROUP=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`CloudWatchLogGroup`].OutputValue' \
  --output text)

# Tail logs
aws logs tail $LOG_GROUP --follow
```

### View S3 Logs

```bash
# Get bucket name from outputs
BUCKET=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`S3BucketName`].OutputValue' \
  --output text)

# List logs
aws s3 ls s3://$BUCKET/ --recursive

# Download logs
aws s3 sync s3://$BUCKET/ ./logs/
```

### Check Service Health

```bash
# Get cluster and service names
CLUSTER=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`ClusterName`].OutputValue' \
  --output text)

SERVICE=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`ServiceName`].OutputValue' \
  --output text)

# Check service status
aws ecs describe-services \
  --cluster $CLUSTER \
  --services $SERVICE \
  --query 'services[0].{Status:status,Running:runningCount,Desired:desiredCount,Health:healthCheckGracePeriodSeconds}'

# Check task status
aws ecs list-tasks --cluster $CLUSTER --service-name $SERVICE
aws ecs describe-tasks \
  --cluster $CLUSTER \
  --tasks $(aws ecs list-tasks --cluster $CLUSTER --service-name $SERVICE --query 'taskArns[0]' --output text) \
  --query 'tasks[0].{Status:lastStatus,Health:healthStatus,CPU:cpu,Memory:memory}'
```

## Scaling

### Update Task Resources

```bash
# Update stack with new CPU/memory
aws cloudformation update-stack \
  --stack-name rustbucket-honeypot \
  --template-body file://rustbucket-ecs.yaml \
  --parameters file://parameters.json \
    ParameterKey=FargateTaskCPU,ParameterValue=1024 \
    ParameterKey=FargateTaskMemory,ParameterValue=2048 \
  --capabilities CAPABILITY_IAM
```

### Scale Task Count

ECS service is configured with DesiredCount=1. To run multiple tasks:

```bash
# Update service desired count
aws ecs update-service \
  --cluster $CLUSTER \
  --service $SERVICE \
  --desired-count 3
```

**Note:** Each task gets its own public IP. Consider using Application Load Balancer for multiple tasks.

### Multi-Region Deployment

Deploy to multiple regions:

```bash
# US East
aws cloudformation create-stack \
  --stack-name rustbucket-us-east-1 \
  --template-body file://rustbucket-ecs.yaml \
  --parameters file://parameters-us-east-1.json \
  --capabilities CAPABILITY_IAM \
  --region us-east-1

# EU West
aws cloudformation create-stack \
  --stack-name rustbucket-eu-west-1 \
  --template-body file://rustbucket-ecs.yaml \
  --parameters file://parameters-eu-west-1.json \
  --capabilities CAPABILITY_IAM \
  --region eu-west-1

# Asia Pacific
aws cloudformation create-stack \
  --stack-name rustbucket-ap-southeast-1 \
  --template-body file://rustbucket-ecs.yaml \
  --parameters file://parameters-ap-southeast-1.json \
  --capabilities CAPABILITY_IAM \
  --region ap-southeast-1
```

## Cost Estimation

**Monthly costs (us-east-1):**
- Fargate (0.5 vCPU, 1GB): ~$14/month (24/7)
- S3 Storage (1GB): ~$0.02/month
- Data Transfer (10GB out): ~$0.90/month
- CloudWatch Logs (1GB): ~$0.50/month
- **Total: ~$15-16/month**

**Cost optimization:**
- Use smaller task sizes (256 CPU, 512 MB) for low traffic: ~$7/month
- Enable S3 lifecycle policies to auto-delete old logs
- Use CloudWatch Logs retention policies
- Consider Spot instances for non-critical deployments

Use AWS Pricing Calculator: https://calculator.aws

## Troubleshooting

### Stack Creation Fails

```bash
# View stack events
aws cloudformation describe-stack-events \
  --stack-name rustbucket-honeypot \
  --query 'StackEvents[?ResourceStatus==`CREATE_FAILED`]'

# Common issues:
# 1. S3 bucket name already exists (must be globally unique)
# 2. Invalid API key format (check length constraint)
# 3. CPU/Memory combination invalid (see valid combinations above)
```

### Task Not Starting

```bash
# Get task failures
aws ecs describe-tasks \
  --cluster $CLUSTER \
  --tasks $(aws ecs list-tasks --cluster $CLUSTER --service-name $SERVICE --query 'taskArns[0]' --output text) \
  --query 'tasks[0].{StoppedReason:stoppedReason,Containers:containers[0].reason}'

# Common issues:
# 1. Image pull fails: Check image exists at ghcr.io/jamesbinford/rustbucket:latest
# 2. Invalid API key: Check CHATGPT_API_KEY environment variable
# 3. Out of capacity: Try different availability zones
```

### Cannot Access Honeypot

```bash
# Check security group rules
aws ec2 describe-security-groups \
  --filters "Name=tag:Name,Values=rustbucket-sg" \
  --query 'SecurityGroups[0].IpPermissions'

# Check task public IP (may change if task restarts)
# Use the GetPublicIPCommand from stack outputs
```

### S3 Upload Issues

```bash
# Check task IAM role permissions
TASK_ROLE=$(aws cloudformation describe-stack-resources \
  --stack-name rustbucket-honeypot \
  --logical-resource-id TaskRole \
  --query 'StackResources[0].PhysicalResourceId' \
  --output text)

aws iam get-role --role-name $TASK_ROLE

# Test S3 access from container
aws ecs execute-command \
  --cluster $CLUSTER \
  --task $(aws ecs list-tasks --cluster $CLUSTER --service-name $SERVICE --query 'taskArns[0]' --output text) \
  --container rustbucket \
  --interactive \
  --command "aws s3 ls s3://$BUCKET/"
```

### High Costs

```bash
# Check Fargate usage
aws ce get-cost-and-usage \
  --time-period Start=2025-12-01,End=2025-12-12 \
  --granularity DAILY \
  --metrics BlendedCost \
  --filter file://filter.json

# filter.json:
{
  "Dimensions": {
    "Key": "SERVICE",
    "Values": ["Amazon Elastic Container Service"]
  }
}

# Reduce task size
aws cloudformation update-stack \
  --stack-name rustbucket-honeypot \
  --use-previous-template \
  --parameters \
    ParameterKey=FargateTaskCPU,ParameterValue=256 \
    ParameterKey=FargateTaskMemory,ParameterValue=512 \
  --capabilities CAPABILITY_IAM
```

## Cleanup

### Delete Stack (WARNING: Deletes Everything)

```bash
# Export logs first (optional)
BUCKET=$(aws cloudformation describe-stacks \
  --stack-name rustbucket-honeypot \
  --query 'Stacks[0].Outputs[?OutputKey==`S3BucketName`].OutputValue' \
  --output text)

aws s3 sync s3://$BUCKET/ ./backup-logs/

# Empty S3 bucket (required before stack deletion)
aws s3 rm s3://$BUCKET/ --recursive

# Delete stack
aws cloudformation delete-stack --stack-name rustbucket-honeypot

# Wait for deletion
aws cloudformation wait stack-delete-complete \
  --stack-name rustbucket-honeypot
```

### Partial Cleanup (Keep Logs)

You cannot keep resources after stack deletion. To preserve logs:
1. Export from S3 to local storage
2. Copy to another S3 bucket outside the stack
3. Then delete the stack

## Advanced Configurations

### Add Load Balancer

To distribute traffic across multiple tasks:

1. Add Application Load Balancer to template
2. Create target group for each port
3. Update service to use load balancer
4. Tasks will register automatically

### Enable Container Insights

```bash
# Update cluster settings
aws ecs update-cluster-settings \
  --cluster $CLUSTER \
  --settings name=containerInsights,value=enabled
```

### Custom Configuration

To use custom `Config.toml`:

1. Store config in S3 or Parameter Store
2. Update task definition to download config at startup
3. Mount as volume or copy to container

Example with S3:

```yaml
# In task definition container command:
Command:
  - /bin/sh
  - -c
  - |
    aws s3 cp s3://my-configs/Config.toml /app/Config.toml
    /usr/local/bin/rustbucket
```

### VPC Peering

To connect multiple honeypot deployments:

1. Create VPC peering connections between regions
2. Update route tables
3. Update security groups to allow inter-VPC traffic

## Comparison: CloudFormation vs Terraform

| Feature | CloudFormation | Terraform |
|---------|----------------|-----------|
| **Deployment** | ECS Fargate (serverless) | EC2 (traditional VMs) |
| **Management** | Automatic | Manual Docker/OS updates |
| **Cost** | ~$15/month | ~$16/month |
| **Scaling** | Built-in, automatic | Manual or ASG |
| **Control** | Limited to container | Full host access |
| **Multi-cloud** | AWS only | Any provider |
| **State Management** | AWS-managed | State file required |

**Use CloudFormation/ECS if:**
- You want serverless containers
- You prefer AWS-native tools
- You need automatic scaling/healing

**Use Terraform/EC2 if:**
- You need host-level control
- You want multi-cloud portability
- You prefer infrastructure as code

## Support

For issues or questions:
- GitHub Issues: https://github.com/jamesbinford/rustbucket/issues
- Documentation: https://github.com/jamesbinford/rustbucket
- AWS ECS Documentation: https://docs.aws.amazon.com/ecs/
- AWS CloudFormation Documentation: https://docs.aws.amazon.com/cloudformation/
