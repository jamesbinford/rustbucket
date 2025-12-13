# Rustbucket Deployment Guide

This guide covers various deployment scenarios for Rustbucket honeypot.

## Table of Contents
- [Quick Start with Docker Compose](#quick-start-with-docker-compose)
- [Docker Deployment](#docker-deployment)
- [AWS EC2 Deployment](#aws-ec2-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Pre-built Images](#pre-built-images)

## Quick Start with Docker Compose

**Fastest way to get started:**

```bash
# 1. Clone the repository
git clone https://github.com/jamesbinford/rustbucket.git
cd rustbucket

# 2. Copy environment template
cp .env.example .env

# 3. Edit .env and add your ChatGPT API key
nano .env  # or vim, code, etc.

# 4. Start Rustbucket
docker-compose up -d

# 5. View logs
docker-compose logs -f
```

That's it! Rustbucket is now running and listening on ports 22, 21, 25, 80.

## Docker Deployment

### Using Pre-built Images

```bash
docker pull ghcr.io/jamesbinford/rustbucket:latest

docker run -d \
  --name rustbucket \
  --cap-add=NET_BIND_SERVICE \
  -e CHATGPT_API_KEY=sk-your-key \
  -p 22:22 -p 25:25 -p 80:80 -p 21:21 \
  -v rustbucket-logs:/app/logs \
  ghcr.io/jamesbinford/rustbucket:latest
```

### Building from Source

```bash
# Clone and build
git clone https://github.com/jamesbinford/rustbucket.git
cd rustbucket
docker build -t rustbucket:latest .

# Run
docker run -d \
  --name rustbucket \
  --cap-add=NET_BIND_SERVICE \
  -e CHATGPT_API_KEY=sk-your-key \
  -p 22:22 -p 25:25 -p 80:80 -p 21:21 \
  -v rustbucket-logs:/app/logs \
  rustbucket:latest
```

### With S3 Logging

```bash
docker run -d \
  --name rustbucket \
  --cap-add=NET_BIND_SERVICE \
  -e CHATGPT_API_KEY=sk-your-key \
  -e S3_LOGGING_ENABLED=true \
  -e S3_BUCKET_NAME=my-honeypot-logs \
  -e S3_REGION=us-east-1 \
  -e AWS_ACCESS_KEY_ID=your-access-key \
  -e AWS_SECRET_ACCESS_KEY=your-secret-key \
  -p 22:22 -p 25:25 -p 80:80 -p 21:21 \
  -v rustbucket-logs:/app/logs \
  ghcr.io/jamesbinford/rustbucket:latest
```

## AWS EC2 Deployment

### Launch Script (User Data)

```bash
#!/bin/bash
set -e

# Update system
apt-get update && apt-get upgrade -y
apt-get install -y docker.io docker-compose

# Start Docker
systemctl start docker
systemctl enable docker

# Create app directory
mkdir -p /opt/rustbucket
cd /opt/rustbucket

# Create .env file
cat > .env << 'EOF'
CHATGPT_API_KEY=sk-your-key-here
S3_LOGGING_ENABLED=true
S3_BUCKET_NAME=your-bucket-name
S3_REGION=us-east-1
S3_DELETE_AFTER_UPLOAD=true
EOF

# Create docker-compose.yml
cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  rustbucket:
    image: ghcr.io/jamesbinford/rustbucket:latest
    container_name: rustbucket
    restart: unless-stopped
    env_file: .env
    ports:
      - "22:22"
      - "21:21"
      - "25:25"
      - "80:80"
    cap_add:
      - NET_BIND_SERVICE
    volumes:
      - rustbucket-logs:/app/logs
volumes:
  rustbucket-logs:
EOF

# Pull and start
docker-compose pull
docker-compose up -d

echo "Rustbucket deployed successfully!"
```

### IAM Role for S3 (Recommended)

Instead of using AWS credentials, attach an IAM role to your EC2 instance:

**IAM Policy:**
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:PutObject",
        "s3:PutObjectAcl"
      ],
      "Resource": "arn:aws:s3:::your-bucket-name/*"
    }
  ]
}
```

Then omit AWS credentials from your .env file - the SDK will use the IAM role automatically.

## Kubernetes Deployment

### Basic Deployment

```yaml
# rustbucket-deployment.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: honeypot
---
apiVersion: v1
kind: Secret
metadata:
  name: rustbucket-secrets
  namespace: honeypot
type: Opaque
stringData:
  chatgpt-api-key: "sk-your-key-here"
  s3-bucket-name: "your-bucket-name"
  aws-access-key-id: "your-access-key"
  aws-secret-access-key: "your-secret-key"
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: rustbucket-logs
  namespace: honeypot
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustbucket
  namespace: honeypot
  labels:
    app: rustbucket
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rustbucket
  template:
    metadata:
      labels:
        app: rustbucket
    spec:
      containers:
      - name: rustbucket
        image: ghcr.io/jamesbinford/rustbucket:latest
        ports:
        - containerPort: 22
          name: ssh
        - containerPort: 21
          name: ftp
        - containerPort: 25
          name: smtp
        - containerPort: 80
          name: http
        env:
        - name: CHATGPT_API_KEY
          valueFrom:
            secretKeyRef:
              name: rustbucket-secrets
              key: chatgpt-api-key
        - name: S3_LOGGING_ENABLED
          value: "true"
        - name: S3_BUCKET_NAME
          valueFrom:
            secretKeyRef:
              name: rustbucket-secrets
              key: s3-bucket-name
        - name: S3_REGION
          value: "us-east-1"
        - name: AWS_ACCESS_KEY_ID
          valueFrom:
            secretKeyRef:
              name: rustbucket-secrets
              key: aws-access-key-id
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: rustbucket-secrets
              key: aws-secret-access-key
        securityContext:
          capabilities:
            add:
            - NET_BIND_SERVICE
        volumeMounts:
        - name: logs
          mountPath: /app/logs
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
      volumes:
      - name: logs
        persistentVolumeClaim:
          claimName: rustbucket-logs
---
apiVersion: v1
kind: Service
metadata:
  name: rustbucket
  namespace: honeypot
spec:
  type: LoadBalancer
  selector:
    app: rustbucket
  ports:
  - name: ssh
    port: 22
    targetPort: 22
  - name: ftp
    port: 21
    targetPort: 21
  - name: smtp
    port: 25
    targetPort: 25
  - name: http
    port: 80
    targetPort: 80
```

Deploy:
```bash
kubectl apply -f rustbucket-deployment.yaml
```

## Pre-built Images

Rustbucket publishes Docker images to GitHub Container Registry on every commit to main:

- **Latest**: `ghcr.io/jamesbinford/rustbucket:latest`
- **Specific version**: `ghcr.io/jamesbinford/rustbucket:v1.0.0`
- **By commit SHA**: `ghcr.io/jamesbinford/rustbucket:sha-abc1234`

## Security Considerations

1. **Network Isolation**: Deploy Rustbucket in an isolated network segment
2. **Firewall**: Ensure only honeypot ports (22, 21, 25, 80) are accessible
3. **Monitoring**: Set up alerting for unusual activity
4. **Log Rotation**: Enable S3 logging with automatic deletion to save disk space
5. **Updates**: Regularly update to the latest image for security patches

## Monitoring

### Health Check Endpoint
Docker and Kubernetes automatically monitor the process via healthcheck.

### Logs
```bash
# Docker
docker logs -f rustbucket

# Docker Compose
docker-compose logs -f

# Kubernetes
kubectl logs -f -n honeypot deployment/rustbucket
```

### Metrics
View activity in S3 bucket or local logs:
```bash
# View logs
docker exec rustbucket tail -f /app/logs/rustbucket.log

# S3
aws s3 ls s3://your-bucket/rustbucket-instance-name/
```

## Troubleshooting

### Container exits immediately
- Check ChatGPT API key is set correctly
- Verify Config.toml syntax
- Check logs: `docker logs rustbucket`

### Cannot bind to ports
- Add `--cap-add=NET_BIND_SERVICE` flag
- Or run with `--privileged` (less secure)

### S3 upload fails
- Verify AWS credentials
- Check IAM permissions
- Ensure bucket exists and region is correct

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/jamesbinford/rustbucket/issues
- Documentation: https://github.com/jamesbinford/rustbucket/blob/main/README.md
