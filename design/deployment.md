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
# Edit Config.toml with API keys (OpenAI) and optionally the registration URL.
# Ensure Config.toml is in the same directory as the executable or provide path.
cargo build --release
# sudo is needed if binding to privileged ports (<1024) like 21, 22, 25, 80.
sudo ./target/release/rustbucket
```

**Use Cases**:
- Research environments
- Small-scale honeypot operations
- Development and testing

**Requirements**:
- Linux server with public IP.
- Root privileges if binding to low port numbers (e.g., 21, 23, 25, 80 are currently active).
- OpenAI API key and credits.
- (Optional) URL for instance registration if that feature is to be used.
- `Config.toml` file correctly configured and accessible by the application.
- (AWS credentials for log upload are NOT currently required as S3 upload is not implemented).

### 2. Container Deployment
The existing Dockerfile in the repository (`rustbucket/Dockerfile`) should be used.
It typically involves:
```dockerfile
# Example structure (refer to actual Dockerfile for specifics)
FROM rust:latest as builder
WORKDIR /usr/src/rustbucket
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/rustbucket /usr/local/bin/rustbucket
# Ensure Config.toml is ADDed or mounted into the container, e.g., at /app/Config.toml
# COPY Config.toml /app/Config.toml
# WORKDIR /app
# CMD ["rustbucket"] # Command might need to point to Config.toml if not in default path
```
*(Review and update the Dockerfile snippet above based on the actual `Dockerfile` in the repository if it differs significantly, especially regarding `Config.toml` handling and working directory.)*

```bash
# Container deployment
docker build -t rustbucket .
# Active ports are 21, 23, 25, 80. SSH (22) is not active by default.
docker run -d \
  --name rustbucket \
  -p 21:21 -p 23:23 -p 25:25 -p 80:80 \
  -v $(pwd)/logs:/usr/src/rustbucket/logs \ # Adjust path if WORKDIR in Dockerfile is different
  -v $(pwd)/Config.toml:/usr/src/rustbucket/Config.toml \ # Mount Config.toml
  # Ensure RUST_LOG can be set, e.g., -e RUST_LOG="info"
  rustbucket
```
*(Note: The volume mount path for `Config.toml` and `logs` inside the container should match where the application expects them, often related to the `WORKDIR` in the Dockerfile or how the application resolves paths. The example executable path in `src/main.rs` for logs is relative, "logs", so it would be relative to the WORKDIR.)*

**Advantages**:
- Consistent deployment environment.
- Easy scaling and orchestration.
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

# Pull and run rustbucket container. Ensure Config.toml is mounted.
docker run -d \
  --name rustbucket \
  --restart unless-stopped \
  -p 21:21 -p 23:23 -p 25:25 -p 80:80 \
  -v /path/to/your/Config.toml:/usr/src/rustbucket/Config.toml \ # Adjust path
  -v /path/to/your/logs:/usr/src/rustbucket/logs \ # Adjust path
  # Pass OpenAI API key via environment if preferred over Config.toml for this deployment
  # -e OPENAI_API_KEY="sk-..."
  # -e RUST_LOG="info"
  rustbucket:latest # Assuming image is tagged this way
```
*(Note: Removed AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY from this example as S3 upload is not current. Adjusted ports to reflect active listeners: 21, 23, 25, 80.)*

#### Google Cloud Platform
Cloud Run is suitable for stateless services, often HTTP-triggered. Rustbucket listens on multiple raw TCP ports, making it a better fit for GCE (VMs) or GKE (Kubernetes) where port mapping is more direct. If using Cloud Run, it would typically only expose one HTTP port (e.g., for a web-based honeypot aspect, not its current multi-protocol TCP nature).
```yaml
# Example for GKE (Kubernetes) - more suitable than Cloud Run for multi-TCP-port services
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustbucket
spec:
  replicas: 1 # Or more for scaling
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
        image: gcr.io/your-project/rustbucket:latest # Replace with your image path
        ports:
        - containerPort: 21
          name: ftp
        - containerPort: 23
          name: telnet
        - containerPort: 25
          name: smtp
        - containerPort: 80
          name: http
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: openai-api-key-secret # Kubernetes secret
              key: apikey
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: config-volume
          mountPath: /usr/src/rustbucket/Config.toml # Adjust if app expects it elsewhere
          subPath: Config.toml
        - name: logs-volume
          mountPath: /usr/src/rustbucket/logs # For persistent logs if needed
      volumes:
      - name: config-volume
        configMap:
          name: rustbucket-configmap # Kubernetes ConfigMap for Config.toml
      - name: logs-volume
        emptyDir: {} # Or a persistent volume claim
---
apiVersion: v1
kind: Service
metadata:
  name: rustbucket-service
spec:
  type: LoadBalancer # Exposes the service externally
  selector:
    app: rustbucket
  ports:
  - port: 21
    targetPort: 21
    protocol: TCP
    name: ftp
  - port: 23
    targetPort: 23
    protocol: TCP
    name: telnet
  - port: 25
    targetPort: 25
    protocol: TCP
    name: smtp
  - port: 80
    targetPort: 80
    protocol: TCP
    name: http
```

#### Azure Container Instances
```bash
# Ensure Config.toml content is available, perhaps by baking into image or mounting via Azure File Share
az container create \
  --resource-group your-rg \
  --name rustbucket \
  --image youracr.azurecr.io/rustbucket:latest \ # Replace with your image path
  --ports 21 23 25 80 \
  --dns-name-label rustbucket-honeypot \ # Makes it publicly accessible
  --environment-variables OPENAI_API_KEY=$OPENAI_API_KEY RUST_LOG=info \
  # For Config.toml, consider building it into the image if simple,
  # or use Azure File Share for mounting:
  # --azure-file-volume-account-name <storage_account_name>
  # --azure-file-volume-account-key <storage_account_key>
  # --azure-file-volume-share-name <file_share_name>
  # --azure-file-volume-mount-path /usr/src/rustbucket/ # Mount point for the share
  --restart-policy Always
```

## Infrastructure Requirements

### Compute Resources
- **CPU**: 1 vCPU is likely sufficient for moderate load; 2 vCPUs for higher.
- **Memory**: 256MB-512MB RAM should be adequate; Rust applications are generally memory-efficient.
- **Storage**: 5-10GB for the OS, application, and some log retention. Daily logs can grow, so plan for log rotation/archival if keeping long-term.
- **Network**: Public IP with unrestricted inbound access on the active honeypot ports (21, 23, 25, 80).

### Network Configuration
Firewall rules should allow inbound TCP traffic on the active ports:
```bash
# Example: iptables for ports 21, 23, 25, 80
iptables -A INPUT -p tcp --dport 21 -j ACCEPT   # FTP
iptables -A INPUT -p tcp --dport 23 -j ACCEPT   # Telnet
iptables -A INPUT -p tcp --dport 25 -j ACCEPT   # SMTP
iptables -A INPUT -p tcp --dport 80 -j ACCEPT   # HTTP
# Note: Port 22 (SSH) is not actively listened on by default in main.rs
```
AWS Security Group or similar cloud firewall configurations should reflect these active ports.
```terraform
# AWS Security Group (Terraform example for active ports)
resource "aws_security_group" "rustbucket" {
  name        = "rustbucket-sg"
  description = "Allow traffic to Rustbucket honeypot"

  ingress {
    from_port   = 21
    to_port     = 21
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  ingress {
    from_port   = 23
    to_port     = 23
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  ingress {
    from_port   = 25
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
  # Add other ports if they become active
}
```

### Storage Requirements
- **Local Logs**: `./logs/` directory (relative to executable's working directory) for `tracing` daily rolling logs (e.g., `rustbucket.log.YYYY-MM-DD`).
- **Configuration**: `Config.toml` file with secrets (OpenAI API key) and configuration (prompts, registration URL). Must be accessible by the application.
- (Batch processing storage and AWS S3 for remote log storage are NOT currently implemented features.)

## Configuration Management

### `Config.toml` File
This is the primary method of configuration.
```toml
# Example Config.toml content to be deployed with the application
[general]
# log_level is primarily controlled by RUST_LOG env var or main.rs default
# log_directory is hardcoded to "logs" in main.rs for rolling files

[openai]
api_key = "sk-your-openai-api-key" # Replace with actual key
[openai.static_messages]
message1 = "Hi ChatGPT! You are the backend for a honeypot..."
message2 = "Please maintain the history of each command..."

[registration]
# Optional: URL for instance registration
# rustbucket_registry_url = "http://your-registry.example.com/register"
```

### Environment Variables
- `RUST_LOG`: Standard way to control log verbosity for `tracing` (e.g., `RUST_LOG=info`, `RUST_LOG=debug`). `main.rs` defaults to "info" if not set.
- `OPENAI_API_KEY`: While `chatgpt.rs` loads from `Config.toml`, deployments (especially containerized) might prefer setting this via environment variable for secrets management. The application would need modification to read this from env if not already doing so as a fallback or primary. (Current `chatgpt.rs` reads directly from `Config.toml`).

### Kubernetes ConfigMap & Secrets
For Kubernetes deployments:
- `Config.toml` can be managed via a `ConfigMap`.
- `OPENAI_API_KEY` should be stored in a Kubernetes `Secret` and injected as an environment variable or mounted file.
```yaml
# Example Kubernetes ConfigMap for Config.toml (excluding secrets)
apiVersion: v1
kind: ConfigMap
metadata:
  name: rustbucket-configmap
data:
  Config.toml: |
    [general]
    # log_level typically managed by RUST_LOG env var
    # log_directory is "logs" by default in the app

    [openai.static_messages]
    message1 = "Hi ChatGPT! You are the backend for a honeypot..."
    message2 = "Please maintain the history of each command..."

    [registration]
    # rustbucket_registry_url = "http://your-registry.example.com/register"

# Example Kubernetes Secret for OpenAI API Key
apiVersion: v1
kind: Secret
metadata:
  name: openai-api-key-secret
type: Opaque
stringData:
  apikey: "sk-your-actual-openai-key" # The key itself
```
*(Note: Removed AWS keys from Secret example.)*

## Monitoring and Observability

### Application Metrics
Currently, the application does not expose Prometheus metrics. Potential future metrics could include:
```
- connections_total{port="21"}
- responses_sent_total{protocol="ftp"}
- chatgpt_requests_total
- chatgpt_errors_total
# log_uploads_total / log_upload_failures_total are not applicable
```

### Health Checks
No dedicated HTTP health check endpoint is currently implemented. A simple TCP check on the listening ports can verify if the application is running and responsive at the socket level.

### Log Aggregation
Logs are written to daily rolling files in the `./logs` directory. For centralized logging:
- Use a log shipper agent (e.g., Fluentd, Fluent Bit, Vector, Filebeat) deployed on the host or as a sidecar in Kubernetes.
- Configure the agent to monitor `*.log` files in the application's log directory.
```yaml
# Example Fluent Bit input configuration (kubernetes sidecar or host agent)
# [INPUT]
#   Name tail
#   Path /path/to/rustbucket/logs/*.log  # Adjust to actual log path
#   multiline.parser docker, cri # If logs are structured over multiple lines
#   Tag rustbucket.*
```
*(The previous example for Fluentd/Elasticsearch is a valid approach for log aggregation.)*

## Security Considerations

### Honeypot Security
- **Isolation**: Critical. Deploy in an isolated network segment (DMZ, separate VPC) away from production systems.
- **Monitoring**: Monitor the honeypot host for signs of compromise or unexpected outbound activity.
- **Containment**: Ensure the honeypot cannot be used as a pivot point into internal networks. Strict egress filtering.
- **Updates**: Keep the underlying OS and any host-level dependencies patched. Update Rust and application dependencies regularly.

### Operational Security
Recommendations for container deployments:
```bash
# 1. Non-root user: Dockerfile should define a non-root user.
#    Requires remapping privileged ports if not using capabilities.
# USER rustbucket_user

# 2. Read-only root filesystem (if possible, logs and config need to be writable mounts)
# docker run --read-only ...

# 3. Resource limits (important for preventing DoS on the host)
# docker run --memory="512m" --cpus="1.0" ...
```

### Access Control
- **Management Access**: Secure SSH or other management access to the honeypot host with strong credentials and MFA.
- **API Keys**: Protect the `OPENAI_API_KEY`. Use secrets management tools appropriate for your deployment environment. Rotate keys periodically.
- **Log Access**: If logs are shipped to a central store, secure access to that store.
- **Network**: Strict firewall rules (ingress and egress).

## Scaling Strategies

### Horizontal Scaling
- Deploy multiple instances of Rustbucket, each with its own IP or behind a load balancer that can handle TCP.
- Kubernetes `Deployment` with `replicas > 1` and a `Service` of type `LoadBalancer` is a standard way to achieve this.
```yaml
# Kubernetes Deployment (already provided, ensure it reflects current needs)
# ... (replicas, image path, resource requests/limits)
```

### Load Distribution
- If not using Kubernetes, multiple instances can be run on different IPs.
- Specialized instances for different protocols/ports could be considered if one type of traffic becomes dominant, but the current single binary handles all active ports.
```bash
# Example: Multiple Docker instances on different hosts/IPs
# Host A:
# docker run -d --name rustbucket-a -p 21:21 -p 23:23 -p 25:25 -p 80:80 ... rustbucket
# Host B:
# docker run -d --name rustbucket-b -p 21:21 -p 23:23 -p 25:25 -p 80:80 ... rustbucket
# Then use DNS round-robin or a TCP load balancer.
```
*(Note: Port 22 is not active by default, adjusted example.)*

### Geographic Distribution
- Deploy instances in different geographic regions to capture diverse attack sources.
- Local log files would need to be aggregated to a central location or analyzed regionally.
- (Regional S3 buckets for logs are not applicable with current logging.)
- (CDN integration is not applicable for these raw TCP services.)
- DNS-based load balancing (e.g., GeoDNS) can direct attackers to the nearest honeypot instance.

## Operational Procedures

### Deployment Checklist
1. **Pre-deployment**:
   - [ ] Configure `Config.toml`:
     - [ ] Set `[openai] api_key`.
     - [ ] Review `[openai.static_messages]`.
     - [ ] (Optional) Configure `[registration] rustbucket_registry_url`.
   - [ ] Ensure `Config.toml` will be accessible by the application in the target environment.
   - [ ] Prepare secrets management for `OPENAI_API_KEY` if externalizing from `Config.toml` (e.g., env var, K8s secret).
   - [ ] Review firewall rules for active ports (21, 23, 25, 80) and management access.
   - [ ] Test `Config.toml` validity if possible (e.g., by a dry run or local test).

2. **Deployment**:
   - [ ] Deploy application/container.
   - [ ] Verify port listeners (21, 23, 25, 80) are active and accessible from the internet.
   - [ ] Test ChatGPT API connectivity (e.g., by connecting to a service and sending a command).
   - [ ] Confirm log file creation in the `./logs` directory (e.g., `rustbucket.log.YYYY-MM-DD`).
   - [ ] (S3 upload validation is N/A).

3. **Post-deployment**:
   - [ ] Monitor initial connections and interactions via logs.
   - [ ] Review log output quality and verbosity (adjust `RUST_LOG` if needed).
   - [ ] Set up log aggregation/shipping if required.
   - [ ] Document deployment details (IPs, configuration specifics).

### Maintenance Procedures
- **Daily/Weekly**:
  - Check application logs for errors or unexpected behavior.
  - Monitor disk space used by local logs; implement manual or scripted rotation/archival if needed.
- **API Quotas**: Monitor OpenAI API usage and quotas.
- **Weekly/Monthly**: Review attack patterns from logs and AI response quality.
- **Monthly/As Needed**:
  - Update OS and host dependencies.
  - Update Rust, application dependencies (rebuild and redeploy).
- **Quarterly/Annually**: Rotate `OPENAI_API_KEY`.

### Incident Response
- **High Connection Volume**: Monitor resource usage (CPU, memory, network). Scale horizontally if using containers/Kubernetes.
- **API Failures (OpenAI)**:
    - Check logs for specific error messages from ChatGPT.
    - Verify API key validity and quota status with OpenAI.
    - Consider rotating API key if compromise is suspected.
    - (Fallback mechanisms are currently basic "Error processing request".)
- **Host Security Alerts**:
    - Isolate the honeypot instance immediately (e.g., change firewall rules to block all traffic).
    - Preserve logs and system state for forensics.
    - Re-image or redeploy to a clean state if compromise is confirmed.
- **Performance Issues**: Profile application if necessary. Scale resources or optimize critical code paths.