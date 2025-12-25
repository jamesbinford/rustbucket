# Rustbucket

![Rustbucket Logo](https://drive.google.com/uc?id=1RHe19I8YuFVjgXVx5BkwnxkhgLr9dzz6)

Rustbucket is a lightweight honeypot written in Rust that runs on virtual machines and containers exposed to the internet. By simulating common services on commonly attacked ports, Rustbucket captures and logs malicious activity for analysis.

The fun sauce is that Rustbucket has built-in LLM integration, allowing it to generate responses to attackers in real-time. This can be used to confuse attackers, gather more information, or simply have fun with them.

**Supported LLM Providers:**
- **OpenAI** (GPT-4, GPT-4o-mini, etc.)
- **Anthropic Claude** (Claude 3 Haiku, Sonnet, Opus)
- **Google Gemini** (Gemini 1.5 Flash, Pro)
- **Ollama** (local/self-hosted - Llama, Mistral, etc.)

You can modify the prompts in Config.toml to customize how the LLM responds to attackers!

## Quick Start

Get Rustbucket running in 2 minutes:

```bash
git clone https://github.com/jamesbinford/rustbucket.git
cd rustbucket
cp deploy/docker/.env.example .env
# Edit .env and add your API key for your chosen provider:
#   OPENAI_API_KEY=sk-...      (OpenAI)
#   ANTHROPIC_API_KEY=sk-...   (Claude)
#   GEMINI_API_KEY=AIza...     (Gemini)
#   (no key needed for Ollama)
docker-compose -f deploy/docker/docker-compose.yml up -d
```

See [docs/deployment.md](docs/deployment.md) for detailed deployment options (AWS, Kubernetes, etc.).

## Features

- **Protocol Emulation**: Mimics popular services such as SSH, SMTP, HTTP, and FTP with intelligent response generation.
- **Multi-LLM Support**: Choose from OpenAI, Claude, Gemini, or Ollama (local) for dynamic, realistic responses to attacker commands.
- **Smart LLM Escalation**: Automatically escalates unknown or suspicious commands to LLM while handling known commands natively to minimize API costs.
- **Configurable Ports**: Easily configure which ports to monitor and the services to emulate through a TOML configuration file.
- **S3 Log Upload**: Automatically upload daily rotated log files to AWS S3 with configurable retention and cleanup.
- **Structured Logging**: Captures all interactions with daily log rotation, providing valuable insights into potential attacks.
- **Concurrency**: Utilizes Rust's async capabilities for handling multiple simultaneous connections efficiently.
- **Registry Integration**: Optional registration with a central registry for managing multiple honeypot instances.

### Prerequisites

- Rust (1.50 or later)
- Cargo (Rust's package manager and build system)
- API key for your chosen LLM provider (or Ollama installed locally)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/rustbucket.git
   cd rustbucket
    ```
2. Update Config.toml as needed. Feel free to customize the Prompt messages to make Rustbucket behave the way you want.
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Optionally, build it in a container:
   ```bash
   docker build -t rustbucket .
   ```
5. Set the API key for your chosen provider (see [LLM Configuration](#llm-configuration) below).

## Configuration

Rustbucket uses a `Config.toml` file for configuration. Here are the key sections:

### General Settings

```toml
[general]
log_level = "info"
log_directory = "./logs"
verbose = true
```

### Port Configuration

Enable or disable specific protocol handlers:

```toml
[ports]
ssh = true    # Port 22 - SSH honeypot
http = true   # Port 80 - HTTP honeypot
ftp = true    # Port 21 - FTP honeypot
smtp = true   # Port 25 - SMTP honeypot
dns = false   # Port 53 - DNS honeypot (not yet implemented)
sms = false   # SMS honeypot (not yet implemented)
```

### LLM Configuration

Rustbucket supports multiple LLM providers. Configure your preferred provider in `Config.toml`:

```toml
[llm]
provider = "openai"  # Options: "openai", "claude", "gemini", "ollama"
model = "gpt-4o-mini"

[llm.static_messages]
message1 = "You are an Ubuntu 20.04 server..."
message2 = "Respond exactly as a real Ubuntu server would..."

# Optional: Protocol-specific prompts for more realistic responses
[llm.prompts]
ssh = "You are an Ubuntu 22.04 server with bash shell..."
http = "You are an Apache 2.4 web server on Ubuntu..."
ftp = "You are a vsftpd FTP server..."
smtp = "You are a Postfix SMTP server..."

# For Ollama only:
# ollama_host = "http://localhost:11434"
```

#### Provider-Specific Setup

| Provider | Environment Variable | Default Model | Notes |
|----------|---------------------|---------------|-------|
| OpenAI | `OPENAI_API_KEY` or `CHATGPT_API_KEY` | gpt-4o-mini | Recommended for best quality |
| Claude | `ANTHROPIC_API_KEY` | claude-3-haiku-20240307 | Fast and cost-effective |
| Gemini | `GEMINI_API_KEY` | gemini-1.5-flash | Google's latest model |
| Ollama | (none required) | llama3.2 | Self-hosted, no API costs |

#### Example: Using Claude

```bash
export ANTHROPIC_API_KEY=sk-ant-your-key-here
```

```toml
[llm]
provider = "claude"
model = "claude-3-haiku-20240307"
```

#### Example: Using Ollama (Local)

First, install and run Ollama:
```bash
# Install Ollama (see https://ollama.ai)
ollama pull llama3.2
ollama serve
```

Then configure Rustbucket:
```toml
[llm]
provider = "ollama"
model = "llama3.2"
ollama_host = "http://localhost:11434"  # Optional, this is the default
```

No API key is required for Ollama - it runs entirely on your local machine.

### S3 Log Upload

Rustbucket can automatically upload rotated log files to AWS S3 for centralized log management across multiple honeypot instances.

#### Configuration Options

```toml
[s3_logging]
enabled = false
bucket_name = ""  # e.g., "my-honeypot-logs"
region = "us-east-1"
prefix = ""  # Optional: organize logs by prefix, e.g., "production" or "honeypot-logs"
upload_interval_hours = 24  # Check for logs to upload every 24 hours
retry_interval_hours = 24   # Retry failed uploads after 24 hours
delete_after_upload = false # Set to true to delete local logs after successful S3 upload
```

#### Environment Variables

You can also configure S3 logging via environment variables (overrides Config.toml):

```bash
export S3_LOGGING_ENABLED=true
export S3_BUCKET_NAME=my-honeypot-logs
export S3_REGION=us-east-1
export S3_PREFIX=rustbucket-logs  # Optional
export S3_DELETE_AFTER_UPLOAD=true  # Optional
```

#### AWS Credentials

Rustbucket uses the standard AWS SDK credential chain. Configure credentials using one of these methods:

1. **IAM Role** (recommended for EC2/ECS):
   - Attach an IAM role with S3 write permissions to your instance
   - No additional configuration needed

2. **Environment Variables**:
   ```bash
   export AWS_ACCESS_KEY_ID=your_access_key
   export AWS_SECRET_ACCESS_KEY=your_secret_key
   ```

3. **AWS Credentials File** (`~/.aws/credentials`):
   ```ini
   [default]
   aws_access_key_id = your_access_key
   aws_secret_access_key = your_secret_key
   ```

#### Required IAM Permissions

Your AWS credentials need the following S3 permissions:

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

#### How It Works

1. Rustbucket rotates log files daily (e.g., `rustbucket.log.2025-12-12`)
2. The S3 uploader runs every 24 hours (configurable)
3. It uploads rotated log files (older than 5 minutes) to S3
4. Files are organized by instance: `s3://bucket/[prefix/]instance-name/rustbucket.log.2025-12-12`
5. Each Rustbucket instance generates a unique name for multi-instance deployments
6. Optionally deletes local files after successful upload to save disk space

### Registry Integration

Rustbucket can optionally register with a central registry server:

```toml
[registration]
rustbucket_registry_url = "https://your-registry.example.com"
```

Or via environment variable:
```bash
export RUSTBUCKET_REGISTRY_URL=https://your-registry.example.com
```

Registration sends system information (IP, OS, resource usage) to help manage multiple honeypot instances.

## Usage

### Running Rustbucket

```bash
# Set your API key (choose one based on your provider)
export OPENAI_API_KEY=sk-your-api-key-here      # OpenAI
# export ANTHROPIC_API_KEY=sk-ant-your-key-here # Claude
# export GEMINI_API_KEY=AIza-your-key-here      # Gemini
# (no key needed for Ollama)

# Run the honeypot
cargo run --release
```

Or with Docker:

```bash
# OpenAI example
docker run -e OPENAI_API_KEY=sk-your-api-key-here \
  -p 22:22 -p 25:25 -p 80:80 -p 21:21 \
  -v ./logs:/app/logs \
  rustbucket

# Claude example
docker run -e ANTHROPIC_API_KEY=sk-ant-your-key-here \
  -p 22:22 -p 25:25 -p 80:80 -p 21:21 \
  -v ./logs:/app/logs \
  -v ./Config.toml:/app/Config.toml \
  rustbucket
```

### Running with S3 Logging

```bash
# Set your LLM provider API key
export OPENAI_API_KEY=sk-your-api-key-here  # or other provider

# Configure S3 via environment variables
export S3_LOGGING_ENABLED=true
export S3_BUCKET_NAME=my-honeypot-logs
export S3_REGION=us-east-1

# With IAM role (EC2/ECS - no credentials needed)
cargo run --release

# Or with AWS credentials
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
cargo run --release
```

### Viewing Logs

Logs are stored in the `logs/` directory (configurable):

```bash
# View current log
tail -f logs/rustbucket.log

# View specific day's log
cat logs/rustbucket.log.2025-12-12
```

If S3 logging is enabled, rotated logs are automatically uploaded to your S3 bucket.

## Architecture

### Smart LLM Escalation

Rustbucket minimizes API costs by intelligently deciding when to escalate commands to the LLM:

1. **Known Commands**: Standard protocol commands (e.g., `ls`, `pwd`, `USER`, `HELO`) are handled natively with pre-defined responses
2. **Unknown Commands**: Suspicious or unusual commands are escalated to the configured LLM for dynamic responses
3. **Bot Detection**: Rapid-fire commands or patterns suggesting automated scanning skip LLM escalation to save costs
4. **Configurable Thresholds**: Set escalation thresholds and patterns in `Config.toml`

This approach provides realistic interactions while keeping API costs minimal for bulk automated scanning.

### Protocol Handlers

Each protocol handler (SSH, FTP, SMTP, HTTP) implements:

- **Command Recognition**: Identifies and validates protocol-specific commands
- **Native Responses**: Returns appropriate responses for known commands
- **LLM Integration**: Escalates unknown/suspicious commands to your configured LLM provider
- **Session State**: Tracks session information for realistic interactions

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests serially (avoid env var conflicts)
cargo test -- --test-threads=1

# Generate coverage report
cargo tarpaulin --out Html
```

### Test Coverage

Current test coverage: **40.54%** (375/925 lines)

- Protocol handlers: Unit tests for command parsing and response generation
- LLM escalation logic: 97.5% coverage
- Configuration loading: Comprehensive tests with mocks

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is open source and available under the MIT License.

## Security Note

Rustbucket is designed to be exposed to the internet and receive malicious traffic. Run it in an isolated environment (VM, container, or dedicated server) with appropriate network segmentation. Do not run on systems with sensitive data or production infrastructure.