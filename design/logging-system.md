# Logging System Design

## Overview
Rustbucket implements a comprehensive logging system for capturing, processing, and storing honeypot interaction data. The system handles both real-time operational logging and attack data collection for analysis.

## Logging Architecture

### Two-Tier Logging System

#### 1. Application Logging (Tracing-based)
```rust
// Operational logs using tracing crate
info!("Actor attempted to connect to port 25 - SMTP");
error!("Actor connected to an unexpected port");
info!("Received data: {}", received_data);
info!("Response message: {}", response_message);
```

#### 2. Attack Data Collection (File-based)
```rust
// Manual log collection for batch processing
log_collector::collect_log("This is a sample log", log_file);
```

## Log Processing Pipeline

### 1. Collection Phase (`log_collector.rs`)
```rust
pub fn collect_log(log_message: &str, log_file: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .unwrap();
    
    writeln!(file, "{}", log_message).unwrap();
}
```
- **Function**: Append structured log entries to files
- **Format**: Plain text, one entry per line
- **File Handling**: Auto-create files, append-only mode
- **Error Handling**: Panic on file operation failures

### 2. Batching Phase (`log_batcher.rs`)
```rust
pub async fn start_batching_process() {
    loop {
        // Collect logs
        log_collector::collect_log("sample", "logs/batch.log");
        
        // Compress
        log_compressor::compress_logs("logs/batch.log", "logs/batch.gz");
        
        // Upload
        log_uploader::upload_to_s3("logs/batch.gz", bucket, &s3_key).await;
        
        // Wait for next interval
        sleep(interval).await;
    }
}
```
- **Scheduling**: Configurable interval (default: 1 hour)
- **Coordination**: Orchestrates collect → compress → upload pipeline
- **Configuration**: Reads AWS settings and timing from Config.toml
- **Error Handling**: Logs upload failures, continues processing

### 3. Compression Phase (`log_compressor.rs`)
```rust
pub fn compress_logs(input_file: &str, output_file: &str) -> io::Result<()> {
    let input = File::open(input_file)?;
    let mut encoder = GzEncoder::new(File::create(output_file)?, Compression::default());
    io::copy(&mut input.take(10_000_000), &mut encoder)?;
    encoder.finish()?;
}
```
- **Algorithm**: Gzip compression (flate2 crate)
- **Size Limit**: 10MB maximum input file size
- **Compression Level**: Default compression ratio
- **Output**: Creates `.gz` files for upload

### 4. Upload Phase (`log_uploader.rs`)
```rust
pub async fn upload_to_s3(file_path: &str, bucket: &str, key: &str) -> Result<(), Error> {
    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);
    
    let body = ByteStream::from_path(Path::new(file_path)).await?;
    
    client.put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;
}
```
- **Destination**: AWS S3 storage
- **Authentication**: Environment-based AWS credentials
- **Key Generation**: `{app_id}/batch.gz` format
- **Error Handling**: Returns detailed AWS SDK errors

## Log Configuration

### Tracing Configuration
```rust
// Daily rolling logs
let file_appender = rolling::daily("logs", "rustbucket.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

let subscriber = tracing_subscriber::fmt::Subscriber::builder()
    .with_env_filter(EnvFilter::new("info"))
    .with_writer(non_blocking)
    .with_ansi(false)
    .init();
```

### Config.toml Settings
```toml
[general]
log_level = "info"
log_directory = "./logs"
upload_interval_secs = 3600

[aws]
app_id = "unique-identifier"
s3_bucket = "log-storage-bucket"
```

## Log Data Structure

### Application Logs
```
2024-01-01T12:00:00Z INFO Actor attempted to connect to port 25 - SMTP
2024-01-01T12:00:01Z INFO Received data: HELO attacker.com
2024-01-01T12:00:02Z INFO Response message: 250 Hello
2024-01-01T12:00:03Z INFO ChatGPT responded: 250 Hello
```

### Attack Data Logs
```
Connection: 192.168.1.100:54321 -> 0.0.0.0:25
Timestamp: 2024-01-01T12:00:00Z
Input: HELO attacker.com
Response: 250 Hello
Duration: 30s
```

## Storage Strategy

### Local Storage
- **Directory**: `./logs/` (configurable)
- **Rotation**: Daily files via tracing-appender
- **Retention**: Manual cleanup required
- **Format**: Human-readable text files

### Remote Storage
- **Platform**: AWS S3
- **Structure**: `{app_id}/{timestamp}/batch.gz`
- **Compression**: Gzip for reduced storage costs
- **Access**: Programmatic via AWS SDK

## Performance Considerations

### Async Processing
- **Non-blocking I/O**: Tracing uses async file writers
- **Background Tasks**: Log processing doesn't block connections
- **Batch Processing**: Reduces individual upload overhead

### Resource Usage
- **Memory**: Bounded by compression buffer (10MB limit)
- **Disk**: Rolling logs prevent unbounded growth
- **Network**: Compressed uploads reduce bandwidth usage
- **CPU**: Gzip compression adds processing overhead

## Security Considerations

### Data Sensitivity
- **Attack Data**: May contain sensitive information from attackers
- **PII Handling**: No built-in PII scrubbing or anonymization
- **Access Control**: S3 bucket permissions control access
- **Retention**: No automatic expiration policies

### Operational Security
- **API Keys**: AWS credentials via environment variables
- **Log Integrity**: No tamper protection or signing
- **Transport Security**: HTTPS for S3 uploads
- **Local Access**: File system permissions protect local logs

## Future Enhancements

### Structured Logging
- **JSON Format**: Machine-readable log entries
- **Schema Definition**: Consistent field naming and types
- **Metadata Enrichment**: GeoIP, threat intel integration

### Real-time Processing
- **Stream Processing**: Real-time analysis instead of batch
- **Alerting**: Immediate notification of high-value attacks
- **Analytics**: Real-time dashboards and metrics

### Enhanced Storage
- **Database Integration**: Structured storage for querying
- **Retention Policies**: Automatic cleanup based on age/size
- **Backup Strategy**: Redundant storage across regions