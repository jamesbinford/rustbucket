# Multi-Instance Rustbucket Deployment

This guide covers deploying multiple Rustbucket instances simultaneously using Docker Compose.

## Table of Contents
- [Why Multiple Instances?](#why-multiple-instances)
- [Approach 1: Specialized Instances](#approach-1-specialized-instances-recommended)
- [Approach 2: Scaled Replicas](#approach-2-scaled-replicas)
- [Managing Multiple Instances](#managing-multiple-instances)
- [Monitoring Multiple Instances](#monitoring-multiple-instances)

## Why Multiple Instances?

Deploy multiple Rustbucket instances to:

1. **Service Isolation**: Separate logs for SSH, HTTP, FTP, SMTP
2. **Geographic Distribution**: Deploy across multiple regions
3. **Load Distribution**: Handle more concurrent connections
4. **A/B Testing**: Test different LLM prompts simultaneously
5. **Redundancy**: High availability for critical monitoring

## Approach 1: Specialized Instances (Recommended)

Deploy dedicated instances for each protocol with separate logs and S3 prefixes.

### Configuration

**File**: `docker-compose.multi.yml`

Each instance:
- Runs on its designated port (22, 25, 80, 21)
- Has unique hostname and container name
- Uploads logs to unique S3 prefix
- Has isolated log volume

### Deploy All Instances

```bash
# Start all 4 instances (SSH, HTTP, SMTP, FTP)
docker-compose -f docker-compose.multi.yml up -d

# Check status
docker-compose -f docker-compose.multi.yml ps

# View logs from all instances
docker-compose -f docker-compose.multi.yml logs -f

# View logs from specific instance
docker-compose -f docker-compose.multi.yml logs -f rustbucket-ssh
```

### Deploy Specific Instances

```bash
# Deploy only SSH and HTTP honeypots
docker-compose -f docker-compose.multi.yml up -d rustbucket-ssh rustbucket-http

# Stop FTP honeypot
docker-compose -f docker-compose.multi.yml stop rustbucket-ftp
```

### S3 Organization

With specialized instances, logs are organized by protocol:

```
s3://your-bucket/
├── ssh-honeypot/
│   ├── rustbucket-ssh/
│   │   ├── rustbucket.log.2025-12-12
│   │   └── rustbucket.log.2025-12-13
├── http-honeypot/
│   ├── rustbucket-http/
│   │   ├── rustbucket.log.2025-12-12
│   │   └── rustbucket.log.2025-12-13
├── smtp-honeypot/
│   └── rustbucket-smtp/
│       └── rustbucket.log.2025-12-12
└── ftp-honeypot/
    └── rustbucket-ftp/
        └── rustbucket.log.2025-12-12
```

### Resource Usage

| Instances | CPU | Memory | Disk (logs/day) |
|-----------|-----|--------|-----------------|
| 1 | 0.5 | 256MB | ~50MB |
| 2 | 1.0 | 512MB | ~100MB |
| 4 (all) | 2.0 | 1GB | ~200MB |

### Custom Configurations

You can customize each instance with different configs:

```yaml
# In docker-compose.multi.yml
rustbucket-ssh:
  volumes:
    - rustbucket-ssh-logs:/app/logs
    - ./configs/ssh-config.toml:/app/Config.toml:ro  # Custom SSH config

rustbucket-http:
  volumes:
    - rustbucket-http-logs:/app/logs
    - ./configs/http-config.toml:/app/Config.toml:ro  # Custom HTTP config
```

## Approach 2: Scaled Replicas

Deploy multiple identical instances that Docker Compose will distribute across random ports.

### Configuration

**File**: `docker-compose.scale.yml`

Features:
- Identical instances with same configuration
- Random host port assignment
- Useful for load testing or redundancy
- Good for Docker Swarm deployments

### Deploy Scaled Instances

```bash
# Deploy 3 identical instances
docker-compose -f docker-compose.scale.yml up -d --scale rustbucket=3

# Scale up to 5 instances
docker-compose -f docker-compose.scale.yml up -d --scale rustbucket=5

# Scale down to 2 instances
docker-compose -f docker-compose.scale.yml up -d --scale rustbucket=2

# Check which ports were assigned
docker-compose -f docker-compose.scale.yml ps
```

### Port Mappings

When scaling, Docker assigns random host ports:

```
NAME                    PORTS
rustbucket-1            0.0.0.0:32768->22/tcp, 0.0.0.0:32769->25/tcp, ...
rustbucket-2            0.0.0.0:32770->22/tcp, 0.0.0.0:32771->25/tcp, ...
rustbucket-3            0.0.0.0:32772->22/tcp, 0.0.0.0:32773->25/tcp, ...
```

### Finding Assigned Ports

```bash
# List all port mappings
docker ps --format "table {{.Names}}\t{{.Ports}}" | grep rustbucket

# Get SSH port for instance 2
docker port rustbucket-2 22
# Output: 0.0.0.0:32770
```

### Access Scaled Instances

```bash
# Connect to instance 1 SSH (assuming port 32768)
ssh user@localhost -p 32768

# Connect to instance 2 SSH (assuming port 32770)
ssh user@localhost -p 32770
```

## Managing Multiple Instances

### Start/Stop Operations

```bash
# Start all instances
docker-compose -f docker-compose.multi.yml up -d

# Stop all instances
docker-compose -f docker-compose.multi.yml down

# Restart specific instance
docker-compose -f docker-compose.multi.yml restart rustbucket-ssh

# Stop without removing
docker-compose -f docker-compose.multi.yml stop
```

### View Logs

```bash
# All instances combined
docker-compose -f docker-compose.multi.yml logs -f

# Specific instance
docker-compose -f docker-compose.multi.yml logs -f rustbucket-http

# Last 100 lines from all instances
docker-compose -f docker-compose.multi.yml logs --tail=100

# Follow logs from SSH and HTTP only
docker-compose -f docker-compose.multi.yml logs -f rustbucket-ssh rustbucket-http
```

### Health Checks

```bash
# Check health status of all instances
docker ps --filter "name=rustbucket" --format "table {{.Names}}\t{{.Status}}"

# Inspect specific instance health
docker inspect rustbucket-ssh | grep -A 10 Health
```

### Resource Monitoring

```bash
# Real-time stats for all instances
docker stats $(docker ps --filter "name=rustbucket" -q)

# Top 3 instances by memory
docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}" | grep rustbucket | sort -k2 -hr | head -3
```

## Monitoring Multiple Instances

### Centralized Logging

**Option 1: S3 with Unique Prefixes**

Each instance uploads to a unique S3 prefix (already configured in docker-compose.multi.yml):

```bash
# View all logs in S3
aws s3 ls s3://your-bucket/ --recursive | grep rustbucket

# Download logs from SSH honeypot
aws s3 sync s3://your-bucket/ssh-honeypot/rustbucket-ssh/ ./logs/ssh/

# Download all logs
aws s3 sync s3://your-bucket/ ./logs/
```

**Option 2: Log Aggregation with Splunk/ELK**

```yaml
# Add to any instance in docker-compose.multi.yml
logging:
  driver: "splunk"
  options:
    splunk-token: "your-token"
    splunk-url: "https://your-splunk:8088"
    tag: "rustbucket-{{.Name}}"
```

### Prometheus Metrics

Create `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'rustbucket'
    static_configs:
      - targets:
        - 'rustbucket-ssh:9090'
        - 'rustbucket-http:9090'
        - 'rustbucket-smtp:9090'
        - 'rustbucket-ftp:9090'
```

### Docker Compose Monitoring Stack

```yaml
# Add to docker-compose.multi.yml
  prometheus:
    image: prom/prometheus
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
```

## Advanced Patterns

### Geographic Distribution

Deploy instances across multiple Docker hosts:

```bash
# On server 1 (us-east-1)
export DOCKER_HOST=ssh://user@server1
docker-compose -f docker-compose.multi.yml up -d rustbucket-ssh rustbucket-http

# On server 2 (eu-west-1)
export DOCKER_HOST=ssh://user@server2
docker-compose -f docker-compose.multi.yml up -d rustbucket-smtp rustbucket-ftp
```

### Network Segmentation

Isolate instances on different networks:

```yaml
# docker-compose.multi.yml
networks:
  ssh_net:
  http_net:

services:
  rustbucket-ssh:
    networks:
      - ssh_net

  rustbucket-http:
    networks:
      - http_net
```

### Automated Scaling Based on Load

```bash
#!/bin/bash
# scale-based-on-connections.sh

# Get connection count
CONNECTIONS=$(docker exec rustbucket-ssh netstat -an | grep ESTABLISHED | wc -l)

if [ $CONNECTIONS -gt 100 ]; then
    # Scale up
    docker-compose -f docker-compose.scale.yml up -d --scale rustbucket=5
elif [ $CONNECTIONS -lt 20 ]; then
    # Scale down
    docker-compose -f docker-compose.scale.yml up -d --scale rustbucket=2
fi
```

## Troubleshooting

### Port Conflicts

If you get "port already in use" errors:

```bash
# Check what's using the port
sudo lsof -i :22

# Stop conflicting service
sudo systemctl stop ssh  # For actual SSH on port 22
```

### Instance Not Starting

```bash
# Check logs
docker-compose -f docker-compose.multi.yml logs rustbucket-ssh

# Check container status
docker inspect rustbucket-ssh

# Restart with fresh state
docker-compose -f docker-compose.multi.yml down
docker-compose -f docker-compose.multi.yml up -d
```

### High Memory Usage

```bash
# Check memory per instance
docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}\t{{.MemPerc}}"

# Restart heavy instance
docker-compose -f docker-compose.multi.yml restart rustbucket-http

# Adjust memory limits in docker-compose.multi.yml
```

## Best Practices

1. **Use Specialized Instances** for production (better log organization)
2. **Enable S3 Logging** with unique prefixes per instance
3. **Set Resource Limits** to prevent any instance from consuming all resources
4. **Monitor Health Checks** to detect failing instances
5. **Rotate Logs** either via S3 deletion or local rotation
6. **Use Volumes** for persistent logs across restarts
7. **Tag Instances** with meaningful names for easier identification

## Example Production Setup

```bash
# 1. Configure environment
cp .env.example .env
nano .env  # Add CHATGPT_API_KEY and S3 settings

# 2. Deploy specialized instances
docker-compose -f docker-compose.multi.yml up -d

# 3. Verify all running
docker-compose -f docker-compose.multi.yml ps

# 4. Check logs are being written
docker-compose -f docker-compose.multi.yml logs --tail=20

# 5. Verify S3 uploads (after 24 hours)
aws s3 ls s3://your-bucket/ --recursive

# 6. Monitor health
watch 'docker ps --filter "name=rustbucket" --format "table {{.Names}}\t{{.Status}}"'
```

Now you have 4 honeypot instances running on standard ports, each logging to separate S3 paths!
