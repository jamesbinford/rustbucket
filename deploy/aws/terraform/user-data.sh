#!/bin/bash

# Log all output
exec > /var/log/user-data.log 2>&1
set -ex

echo "Starting Rustbucket deployment at $(date)"

# Update system
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get upgrade -y

# Install Docker dependencies
apt-get install -y curl ca-certificates

# Install Docker
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc

echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin

systemctl start docker
systemctl enable docker

# Move system SSH to port 2222 BEFORE starting container on port 22
# Use drop-in config for Ubuntu 22.04+ compatibility
mkdir -p /etc/ssh/sshd_config.d
echo "Port 2222" > /etc/ssh/sshd_config.d/99-honeypot.conf
systemctl restart ssh

# Create app directory
mkdir -p /opt/rustbucket
cd /opt/rustbucket

# Pull the container from Docker Hub
docker pull jamesbinford/rustbucket:latest

# Create Config.toml
cat > Config.toml << 'CONFIG_EOF'
[general]
log_level = "info"
log_directory = "./logs"
verbose = false

[llm]
model = "gpt-4o-mini"
static_messages = { message1 = "You are an Ubuntu Server.", message2 = "Respond as an Ubuntu server would. Do not break character." }

[llm_escalation]
enabled = true
unknown_command_threshold = 3
max_llm_calls_per_session = 10
use_llm_for_human_like = true

[s3_logging]
bucket_name = "${s3_bucket_name}"
region = "${s3_region}"
prefix = "rustbucket-logs"
upload_interval_hours = 1
retry_interval_hours = 1
delete_after_upload = ${delete_after_upload}
CONFIG_EOF

# Create docker-compose.yml (using COMPOSE_EOF without quotes to allow variable substitution)
cat > docker-compose.yml << COMPOSE_EOF
services:
  rustbucket:
    image: jamesbinford/rustbucket:latest
    container_name: rustbucket
    restart: unless-stopped
    environment:
      - CHATGPT_API_KEY=${chatgpt_api_key}
    ports:
      - "22:2222"
      - "21:2121"
      - "25:2525"
      - "80:8080"
    volumes:
      - rustbucket-logs:/app/logs
      - ./Config.toml:/app/Config.toml:ro

volumes:
  rustbucket-logs:
    driver: local
COMPOSE_EOF

# Start Rustbucket
docker compose up -d

# Create systemd service for auto-restart on reboot
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

systemctl daemon-reload
systemctl enable rustbucket.service

echo "Rustbucket deployment complete at $(date)!"
echo "Admin SSH on port 2222, honeypot on ports 21/22/25/80"
