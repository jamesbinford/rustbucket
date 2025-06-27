# Rustbucket Architecture Design

## Module Structure

```
src/
├── main.rs           # Entry point, listener management
├── handler.rs        # Connection handling logic
├── chatgpt.rs        # OpenAI API integration
├── prelude.rs        # Common imports and utilities
├── log_collector.rs  # Log collection utilities
├── log_batcher.rs    # Batch processing coordination
├── log_compressor.rs # Log compression functionality
└── log_uploader.rs   # S3 upload capabilities
```

## Component Interactions

### 1. Main Application Flow
```
main.rs
  ├── Initialize tracing/logging
  ├── Spawn listeners for each configured port
  │   ├── Port 21 (FTP): "220 (vsFTPd 3.0.3)"
  │   ├── Port 23 (Telnet): Generic handling
  │   ├── Port 25 (SMTP): "220 mail.example.com ESMTP Postfix"
  │   └── Port 80 (HTTP): "GET / HTTP/1.1\nHost: example.com"
  └── Wait for all listeners indefinitely
```

### 2. Connection Handling Flow
```
handle_client() in handler.rs
  ├── Read data from TCP stream
  ├── Send user input to ChatGPT API
  ├── Receive AI-generated response
  ├── Write response back to client
  ├── Log all interactions
  └── Repeat until connection closes
```

### 3. ChatGPT Integration
```
ChatGPT struct
  ├── Load configuration from Config.toml
  ├── Initialize HTTP client for API calls
  ├── Maintain static prompt messages
  └── send_message() method:
      ├── Prepare system prompts
      ├── Add user input
      ├── Make API call to OpenAI
      └── Return formatted response
```

### 4. Log Management Pipeline
```
Log Management System
  ├── log_collector.rs: Append logs to files
  ├── log_batcher.rs: Coordinate batch processing
  │   ├── Collect logs at intervals
  │   ├── Call compression
  │   └── Trigger upload
  ├── log_compressor.rs: Gzip compression
  └── log_uploader.rs: AWS S3 upload
```

## Configuration Architecture

### Config.toml Structure
```toml
[general]
log_level = "info"
log_directory = "./logs"
upload_interval_secs = 3600

[ports]
ssh = { enabled = true, port = 22 }
http = { enabled = true, port = 80 }
# ... other ports

[openai]
api_key = "your-key-here"
[openai.static_messages]
message1 = "System prompt 1"
message2 = "System prompt 2"

[aws]
app_id = "unique-app-identifier"
s3_bucket = "log-storage-bucket"
```

## Async Architecture

### Concurrency Model
- **Main Thread**: Spawns multiple port listeners
- **Per-Port Tasks**: Each port runs in separate Tokio task
- **Per-Connection Tasks**: Each client connection spawns new task
- **Background Tasks**: Log processing runs independently

### Resource Management
- **Memory**: Minimal per-connection state
- **File Handles**: Daily rolling logs with automatic cleanup
- **Network**: Multiple simultaneous TCP connections
- **CPU**: Async I/O reduces thread overhead

## Error Handling Strategy

### Connection Errors
- Network failures logged but don't crash server
- Individual connection failures isolated from other connections
- Graceful degradation when ChatGPT API unavailable

### Configuration Errors
- Application fails fast on invalid configuration
- Missing API keys logged with clear error messages
- Invalid port configurations prevent startup

### Log Processing Errors
- Failed uploads retried on next interval
- Compression failures logged but don't stop collection
- S3 connectivity issues don't affect core honeypot functionality