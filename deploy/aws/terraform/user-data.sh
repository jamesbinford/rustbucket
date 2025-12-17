#!/bin/bash
set -e

# Log all output
exec > >(tee /var/log/user-data.log|logger -t user-data -s 2>/dev/console) 2>&1

echo "Starting Rustbucket deployment..."

# Update system
apt-get update
apt-get upgrade -y

# Install build dependencies
apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    apt-transport-https \
    ca-certificates \
    software-properties-common

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
export PATH="$HOME/.cargo/bin:$PATH"

# Install Docker
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

# Start and enable Docker
systemctl start docker
systemctl enable docker

# Create app directory
mkdir -p /opt/rustbucket
cd /opt/rustbucket

# Clone the repository
git clone https://github.com/jamesbinford/rustbucket.git .

# Create Config.toml from environment variables
cat > Config.toml << 'CONFIG_EOF'
[general]
log_level = "info"
log_directory = "./logs"
verbose = false

[llm]
model = "gpt-3.5-turbo"
static_messages = { message1 = "You are an Ubuntu Server.", message2 = "Respond as an Ubuntu server would. Do not break character." }

[llm_escalation]
enabled = true
unknown_command_threshold = 3
max_llm_calls_per_session = 10
use_llm_for_human_like = true

[s3_logging]
enabled = ${enable_s3}
bucket_name = "${s3_bucket_name}"
region = "${s3_region}"
prefix = "rustbucket-logs"
upload_interval_hours = 24
retry_interval_hours = 24
delete_after_upload = ${delete_after_upload}
CONFIG_EOF

# Set environment variable for OpenAI API key
echo "CHATGPT_API_KEY=${chatgpt_api_key}" >> /etc/environment

# Build the Docker image
docker build -t rustbucket:latest .

# Create docker-compose.yml
cat > docker-compose.yml << 'COMPOSE_EOF'
version: '3.8'

services:
  rustbucket:
    image: rustbucket:latest
    container_name: rustbucket
    restart: unless-stopped

    environment:
      - CHATGPT_API_KEY=${chatgpt_api_key}
      - S3_LOGGING_ENABLED=${enable_s3}
      - S3_BUCKET_NAME=${s3_bucket_name}
      - S3_REGION=${s3_region}
      - S3_DELETE_AFTER_UPLOAD=${delete_after_upload}

    ports:
      - "22:22"
      - "21:21"
      - "25:25"
      - "80:80"

    cap_add:
      - NET_BIND_SERVICE

    volumes:
      - rustbucket-logs:/app/logs
      - ./Config.toml:/app/Config.toml:ro

    healthcheck:
      test: ["CMD", "pgrep", "-x", "rustbucket"]
      interval: 30s
      timeout: 3s
      start_period: 5s
      retries: 3

volumes:
  rustbucket-logs:
    driver: local
COMPOSE_EOF

# Start Rustbucket
docker compose up -d

# Create systemd service for auto-restart
cat > /etc/systemd/system/rustbucket.service << 'SERVICE_EOF'
[Unit]
Description=Rustbucket Honeypot
Requires=docker.service
After=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/opt/rustbucket
ExecStart=/usr/bin/docker compose up -d
ExecStop=/usr/bin/docker compose down
TimeoutStartSec=0

[Install]
WantedBy=multi-user.target
SERVICE_EOF

# Enable systemd service
systemctl daemon-reload
systemctl enable rustbucket.service

# Install CloudWatch agent
wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb
dpkg -i -E ./amazon-cloudwatch-agent.deb

# Configure CloudWatch agent
cat > /opt/aws/amazon-cloudwatch-agent/etc/config.json << 'CW_EOF'
{
  "logs": {
    "logs_collected": {
      "files": {
        "collect_list": [
          {
            "file_path": "/var/lib/docker/volumes/opt_rustbucket_rustbucket-logs/_data/*.log",
            "log_group_name": "/aws/ec2/rustbucket",
            "log_stream_name": "{instance_id}/rustbucket"
          }
        ]
      }
    }
  }
}
CW_EOF

# Start CloudWatch agent
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl \
  -a fetch-config \
  -m ec2 \
  -s \
  -c file:/opt/aws/amazon-cloudwatch-agent/etc/config.json

echo "Rustbucket deployment complete!"
echo "Instance is now running and collecting honeypot data."
echo "Logs will be uploaded to S3 bucket: ${s3_bucket_name}"
