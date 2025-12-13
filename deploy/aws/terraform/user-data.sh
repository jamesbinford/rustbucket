#!/bin/bash
set -e

# Update system
apt-get update
apt-get upgrade -y

# Install Docker
apt-get install -y apt-transport-https ca-certificates curl software-properties-common
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

# Create docker-compose.yml
cat > docker-compose.yml << 'COMPOSE_EOF'
version: '3.8'

services:
  rustbucket:
    image: ghcr.io/jamesbinford/rustbucket:latest
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

# Pull and start Rustbucket
docker compose pull
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

# Install CloudWatch agent (optional)
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
            "file_path": "/opt/rustbucket/logs/rustbucket.log",
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
