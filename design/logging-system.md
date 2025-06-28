# Logging System Design

## Overview
Rustbucket uses the `tracing` crate for logging application events and interactions. Logs are written to daily rolling files locally. The previously designed comprehensive log processing pipeline (including batching, compression, and S3 upload) is not currently implemented.

## Logging Architecture

### Application Logging (Tracing-based)
The core logging mechanism relies on the `tracing` ecosystem:
- `tracing`: Provides the framework for instrumenting the application and emitting structured log events.
- `tracing_subscriber`: Used to configure how traces are collected and filtered (e.g., log level via `EnvFilter`).
- `tracing_appender`: Handles the actual writing of logs, specifically `rolling::daily` is used to create new log files each day.

```rust
// Example of operational logs using tracing crate in main.rs and handler.rs
use tracing::{info, error};

info!("Tracing initialized");
info!("Actor attempted to connect to port {} - {}", port_type, service_name);
error!("Actor connected to an unexpected port.");
info!("Received data: {}", received_data);
info!("Response message: {}", response_message);
error!("Failed to send data: {}", e);
```

(The "Attack Data Collection (File-based)" system with `log_collector.rs` is not present in the current implementation.)

## Log Processing Pipeline
The current system does not feature an automated multi-stage log processing pipeline (collection, batching, compression, upload) as previously envisioned.

**Current Process:**
1.  **Event Emission**: Code instrumented with `tracing` macros (`info!`, `error!`, etc.) generates log events.
2.  **Filtering**: `tracing_subscriber::EnvFilter` (configured typically by the `RUST_LOG` environment variable or a default like "info" in `main.rs`) filters events based on their level and target.
3.  **Writing to File**: `tracing_appender::rolling::daily` writes the filtered log events to files in the specified directory (e.g., `./logs`). A new file is created daily (e.g., `rustbucket.log.YYYY-MM-DD`).
4.  **Output Format**: Logs are typically written in a human-readable text format, including timestamp, level, target, and the message. ANSI color codes are disabled for file logging.

(The sections for Collection Phase, Batching Phase, Compression Phase, and Upload Phase with their respective `.rs` files are removed as they don't reflect the current state.)

## Log Configuration

### Tracing Configuration (as in `main.rs`)
```rust
// Set up rolling logs
let file_appender = rolling::daily("logs", "rustbucket.log"); // Creates files like logs/rustbucket.log.YYYY-MM-DD
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

// Initialize tracing subscriber
tracing_subscriber::fmt() // Use fmt() for easier configuration
    .with_env_filter(EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()), // Allow RUST_LOG override, default to "info"
    ))
    .with_writer(non_blocking) // Write to the non-blocking daily rolling file appender
    .with_ansi(false) // Disable ANSI color codes for file logging
    .init();
info!("Tracing initialized, logging to daily files in 'logs' directory.");
```

### Config.toml Settings
The `Config.toml` file influences logging primarily through:
```toml
[general]
log_level = "info" # This can serve as a default if RUST_LOG env var is not set.
log_directory = "./logs" # Confirms the directory used by tracing_appender.
# upload_interval_secs, [aws] section are not currently used for logging.
```
The actual log level filtering is managed by `EnvFilter` which typically defaults to the `RUST_LOG` environment variable or the level specified in `main.rs` if `RUST_LOG` is not set. The `log_directory` from `Config.toml` is implicitly used by the `rolling::daily` setup in `main.rs`.

## Log Data Structure

### Application Logs (Example from `logs/rustbucket.log.YYYY-MM-DD`)
The format is determined by `tracing_subscriber::fmt` defaults when writing to a file (usually plain text, without ANSI colors).
```
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::main: Tracing initialized
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::main: Listening on 0.0.0.0:25
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::main: New connection on 127.0.0.1:XXXXX: 127.0.0.1:XXXXX
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::main: Actor attempted to connect to port 25 - SMTP
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::main: Actor input message: 220 mail.example.com ESMTP Postfix (Ubuntu)
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::handler: Received data: HELO client.example.com
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::chatgpt: We sent this to ChatGPT: ChatGPTRequest { model: "gpt-3.5-turbo", messages: [Message { role: "system", content: "..." }, Message { role: "system", content: "..." }, Message { role: "user", content: "HELO client.example.com" }] }
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::chatgpt: ChatGPT responded: 250 Ok\n
YYYY-MM-DDTHH:MM:SS.MSZ INFO rustbucket::handler: Response message: 250 Ok
```
(The "Attack Data Logs" as a separate structured format are not explicitly generated; all data is part of the main application log.)

## Storage Strategy

### Local Storage
- **Directory**: `./logs/` (as configured in `main.rs` for `rolling::daily`). This can be influenced by `Config.toml`'s `general.log_directory` if the code is adapted to read it for this purpose, but `main.rs` currently hardcodes "logs".
- **Rotation**: Daily rolling files (e.g., `rustbucket.log.YYYY-MM-DD`). `tracing_appender` handles the creation of new files.
- **Retention**: Manual cleanup is required for old log files. There is no automatic retention policy in place.
- **Format**: Human-readable plain text.

### Remote Storage
- Remote storage (e.g., AWS S3) is **not currently implemented**. The design for batching, compressing, and uploading logs is a future consideration.

## Performance Considerations

### Async Processing
- **Non-blocking I/O**: `tracing_appender::non_blocking` is used, which means log writing operations are performed on a separate thread, minimizing the impact on the main application threads.

### Resource Usage
- **Disk**: Log files will accumulate daily. The amount of disk space used will depend on log volume and frequency of manual cleanup. Rolling logs prevent a single file from growing indefinitely.
- **CPU**: `tracing` itself is designed to be efficient. The primary CPU overhead related to logging would be from formatting messages and I/O operations, mitigated by the non-blocking appender.
- (Considerations for memory/network/CPU for compression and upload are not applicable to the current implementation.)

## Security Considerations

### Data Sensitivity
- **Attack Data**: Logs will contain all data sent by connecting clients (potential attackers), which could include malicious payloads, reconnaissance attempts, or sensitive information if attackers input it.
- **PII Handling**: No built-in PII scrubbing or anonymization. Raw interaction data is logged.
- **Local Access**: File system permissions on the server where Rustbucket is running will determine who can access the local log files.

### Operational Security
- **Log Integrity**: No built-in tamper protection or signing for log files.
- (Considerations for API keys, transport security for S3 are not applicable currently.)

## Future Enhancements
(This section can largely remain, but it's important to frame these as enhancements to the *current, simpler* logging system.)

### Structured Logging
- **JSON Format**: Configure `tracing_subscriber::fmt()` to output logs in JSON for easier machine parsing and ingestion into log analysis platforms.
- **Schema Definition**: If using JSON, maintain a consistent field naming convention.
- **Metadata Enrichment**: Could potentially add more contextual information to logs (e.g., GeoIP for source IPs, though this would require additional dependencies and logic).

### Log Management & Remote Storage (Reintroducing previous concepts as future work)
- **Batch Processing**: Periodically process log files for archival or remote storage.
- **Compression**: Compress log files before archival/upload to save space.
- **Remote Upload**: Implement functionality to upload logs to a remote store like AWS S3, Azure Blob Storage, etc. This would require handling credentials and network operations securely.

### Real-time Processing & Alerting (If remote streaming is implemented)
- **Stream Processing**: If logs are streamed to a capable backend, real-time analysis could be performed.
- **Alerting**: Set up alerts for specific patterns or critical errors observed in the logs.

### Enhanced Storage
- **Database Integration**: For more advanced querying capabilities, logs could be sent to a database (e.g., Elasticsearch).
- **Retention Policies**: Implement automatic cleanup or archival of old logs based on age or size.
- **Backup Strategy**: If critical data is logged, ensure backups of the log storage.