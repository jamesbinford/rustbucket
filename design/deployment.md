# Deployment Design

## Overview
Rustbucket is designed for deployment in internet-exposed environments to capture malicious network activity. This document outlines deployment strategies, infrastructure requirements, and operational considerations.

## Deployment Scenarios

### 1. Standalone Server Deployment
```bash
# Direct server deployment
git clone https://github.com/jamesbinford/rustbucket.git
cd rustbucket
cp config.toml.example Config.toml
# Edit Config.toml with API keys and settings
cargo build --release
sudo ./target/release/rustbucket
```

**Use Cases**:
- Research environments
- Small-scale honeypot operations
- Development and testing

**Requirements**:
- Linux server with public IP
- Root privileges for low port binding (22, 25, 53, 80)
- OpenAI API key and credits
- AWS credentials for log upload (optional)

### 2. Container Deployment
```dockerfile
# Dockerfile included in repository
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rustbucket /usr/local/bin/
COPY --from=builder /app/Config.toml /app/
CMD ["rustbucket"]
```

```bash
# Container deployment
docker build -t rustbucket .
docker run -d \
  --name rustbucket \
  -p 21:21 -p 22:22 -p 25:25 -p 80:80 \
  -v $(pwd)/logs:/app/logs \
  -v $(pwd)/Config.toml:/app/Config.toml \
  rustbucket
```

**Advantages**:
- Consistent deployment environment
- Easy scaling and orchestration
- Isolated from host system
- Simplified dependency management

### 3. Cloud Deployment

#### AWS EC2 Deployment
```bash
# User data script for EC2 instance
#!/bin/bash
yum update -y
yum install -y docker
systemctl start docker
systemctl enable docker

# Pull and run rustbucket container
docker run -d \
  --name rustbucket \
  --restart unless-stopped \
  -p 21:21 -p 22:22 -p 25:25 -p 80:80 \
  -e AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID} \
  -e AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY} \
  rustbucket:latest
```

#### Google Cloud Platform
```yaml
# cloud-run.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: rustbucket
spec:
  template:
    spec:
      containers:
      - image: gcr.io/project/rustbucket:latest
        ports:
        - containerPort: 80
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: openai-secret
              key: api-key
```

#### Azure Container Instances
```bash
az container create \
  --resource-group honeypot-rg \
  --name rustbucket \
  --image rustbucket:latest \
  --ports 21 22 25 80 \
  --environment-variables OPENAI_API_KEY=$OPENAI_API_KEY \
  --restart-policy Always
```

## Infrastructure Requirements

### Compute Resources
- **CPU**: 1-2 vCPUs (sufficient for hundreds of concurrent connections)
- **Memory**: 512MB-1GB RAM (minimal memory footprint)
- **Storage**: 10-20GB (for logs and application)
- **Network**: Public IP with unrestricted inbound access

### Network Configuration
```bash
# Required firewall rules (example: iptables)
iptables -A INPUT -p tcp --dport 21 -j ACCEPT   # FTP
iptables -A INPUT -p tcp --dport 22 -j ACCEPT   # SSH
iptables -A INPUT -p tcp --dport 25 -j ACCEPT   # SMTP
iptables -A INPUT -p tcp --dport 80 -j ACCEPT   # HTTP

# AWS Security Group (Terraform example)
resource "aws_security_group" "rustbucket" {
  ingress {
    from_port   = 21
    to_port     = 25
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

### Storage Requirements
- **Local Logs**: `./logs/` directory for tracing output
- **Batch Processing**: Temporary storage for compression
- **Configuration**: `Config.toml` file with secrets
- **AWS S3**: Remote log storage and analysis

## Configuration Management

### Environment-Based Configuration
```bash
# Environment variables override
export OPENAI_API_KEY="sk-..."
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="..."
export RUSTBUCKET_LOG_LEVEL="debug"
```

### Kubernetes ConfigMap
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rustbucket-config
data:
  Config.toml: |
    [general]
    log_level = "info"
    log_directory = "/app/logs"
    
    [ports]
    ssh = { enabled = true, port = 22 }
    http = { enabled = true, port = 80 }
    
    [openai]
    api_key = "${OPENAI_API_KEY}"
```

### Secret Management
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rustbucket-secrets
type: Opaque
stringData:
  openai-api-key: "sk-your-actual-key"
  aws-access-key: "AKIA..."
  aws-secret-key: "..."
```

## Monitoring and Observability

### Application Metrics
```rust
// Potential metrics to expose
- connections_total{port="22"}
- responses_sent_total{protocol="ssh"}
- chatgpt_requests_total
- chatgpt_errors_total
- log_uploads_total
- log_upload_failures_total
```

### Health Checks
```rust
// Health check endpoint (future enhancement)
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "uptime": get_uptime(),
        "active_connections": get_connection_count(),
        "last_chatgpt_response": get_last_api_call()
    }))
}
```

### Log Aggregation
```yaml
# Fluentd/Fluent Bit configuration
<source>
  @type tail
  path /app/logs/rustbucket.log
  pos_file /var/log/fluentd/rustbucket.pos
  tag rustbucket.application
  format json
</source>

<match rustbucket.**>
  @type elasticsearch
  host elasticsearch.logging.svc.cluster.local
  index rustbucket-logs
</match>
```

## Security Considerations

### Honeypot Security
- **Isolation**: Deploy in isolated network segments
- **Monitoring**: Watch for lateral movement attempts
- **Containment**: Limit access to production systems
- **Updates**: Regular security updates for base images

### Operational Security
```bash
# Recommended security hardening
# 1. Non-root user (requires port remapping)
USER 1001:1001

# 2. Read-only filesystem
docker run --read-only \
  --tmpfs /tmp \
  --tmpfs /app/logs \
  rustbucket

# 3. Resource limits
docker run \
  --memory="512m" \
  --cpus="1.0" \
  --pids-limit=100 \
  rustbucket
```

### Access Control
- **SSH Access**: Separate management interface
- **API Keys**: Rotate regularly, use least privilege
- **Log Access**: Secure S3 bucket permissions
- **Network**: VPC/firewall isolation from production

## Scaling Strategies

### Horizontal Scaling
```yaml
# Kubernetes Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustbucket
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rustbucket
  template:
    spec:
      containers:
      - name: rustbucket
        image: rustbucket:latest
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
```

### Load Distribution
```bash
# Multiple instances on different IPs
# Instance 1: SSH/Telnet focus
docker run -p 22:22 -p 23:23 rustbucket

# Instance 2: Web services focus  
docker run -p 80:80 -p 443:443 rustbucket

# Instance 3: Mail services focus
docker run -p 25:25 -p 110:110 -p 143:143 rustbucket
```

### Geographic Distribution
- **Multi-region deployment** for diverse attack sources
- **Regional S3 buckets** for log storage locality
- **CDN integration** for web-based honeypots
- **DNS-based load balancing** across regions

## Operational Procedures

### Deployment Checklist
1. **Pre-deployment**:
   - [ ] Configure API keys and credentials
   - [ ] Set up S3 bucket and IAM permissions
   - [ ] Review firewall rules and network access
   - [ ] Test configuration file validity

2. **Deployment**:
   - [ ] Deploy application/container
   - [ ] Verify port listeners are active
   - [ ] Test ChatGPT API connectivity
   - [ ] Confirm log file creation
   - [ ] Validate S3 upload functionality

3. **Post-deployment**:
   - [ ] Monitor initial connections
   - [ ] Review log output quality
   - [ ] Set up alerting and monitoring
   - [ ] Document deployment details

### Maintenance Procedures
- **Daily**: Check log upload status and API quotas
- **Weekly**: Review attack patterns and response quality
- **Monthly**: Update dependencies and security patches
- **Quarterly**: Rotate API keys and credentials

### Incident Response
- **High Connection Volume**: Monitor resource usage, scale if needed
- **API Failures**: Check quotas, rotate keys, implement fallbacks
- **Security Alerts**: Isolate honeypot, review logs for compromise
- **Performance Issues**: Scale resources, optimize configuration