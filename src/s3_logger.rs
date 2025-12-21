use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use config::{Config, File};
use serde::Deserialize;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{info, error, warn};

/// S3 logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub bucket_name: Option<String>,
    pub region: Option<String>,
    pub prefix: Option<String>,
    pub upload_interval_hours: u64,
    pub retry_interval_hours: u64,
    pub delete_after_upload: bool,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket_name: None,
            region: None,
            prefix: None,
            upload_interval_hours: 24,
            retry_interval_hours: 24,
            delete_after_upload: false,
        }
    }
}

/// S3 Logger for uploading daily logs
pub struct S3Logger {
    config: S3Config,
    client: Option<S3Client>,
    instance_name: String,
    log_directory: String,
}

impl S3Logger {
    /// Create a new S3Logger instance
    pub async fn new(instance_name: String) -> Result<Self, String> {
        // Load configuration from file
        let settings = Config::builder()
            .add_source(File::with_name("Config").required(false))
            .build()
            .map_err(|e| e.to_string())?;

        let mut config: S3Config = settings
            .get("s3_logging")
            .unwrap_or_else(|_| S3Config::default());

        // Override with environment variables if present
        if let Ok(bucket) = std::env::var("S3_BUCKET_NAME") {
            config.bucket_name = Some(bucket);
        }

        if let Ok(region) = std::env::var("S3_REGION") {
            config.region = Some(region);
        }

        if let Ok(prefix) = std::env::var("S3_PREFIX") {
            config.prefix = Some(prefix);
        }

        if let Ok(delete) = std::env::var("S3_DELETE_AFTER_UPLOAD") {
            config.delete_after_upload = delete.to_lowercase() == "true";
        }

        // Get log directory from config or use default
        let log_directory = settings
            .get_string("general.log_directory")
            .unwrap_or_else(|_| "./logs".to_string());

        // Initialize S3 client if bucket is configured
        let has_bucket = matches!(&config.bucket_name, Some(name) if !name.is_empty());
        let client = if has_bucket {
            Some(Self::create_s3_client(&config).await?)
        } else {
            None
        };

        Ok(Self {
            config,
            client,
            instance_name,
            log_directory,
        })
    }

    /// Create AWS S3 client
    async fn create_s3_client(config: &S3Config) -> Result<S3Client, String> {
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest());

        // Set region if specified (non-empty)
        if let Some(region) = &config.region {
            if !region.is_empty() {
                aws_config_builder = aws_config_builder.region(
                    aws_config::Region::new(region.clone())
                );
            }
        }

        let aws_config = aws_config_builder.load().await;
        Ok(S3Client::new(&aws_config))
    }

    /// Check if S3 logging is enabled (bucket name is configured)
    pub fn is_enabled(&self) -> bool {
        matches!(&self.config.bucket_name, Some(name) if !name.is_empty())
            && self.client.is_some()
    }

    /// Start the S3 log uploader background task
    pub async fn start_background_uploader(self) {
        if !self.is_enabled() {
            info!("S3 logging is disabled or not configured");
            return;
        }

        info!("Starting S3 log uploader background task");
        info!("S3 Bucket: {:?}", self.config.bucket_name);
        info!("Instance: {}", self.instance_name);
        info!("Upload interval: {} hours", self.config.upload_interval_hours);
        info!("Delete after upload: {}", self.config.delete_after_upload);

        tokio::spawn(async move {
            let upload_interval = Duration::from_secs(self.config.upload_interval_hours * 3600);
            let retry_interval = Duration::from_secs(self.config.retry_interval_hours * 3600);

            loop {
                // Wait for the upload interval
                sleep(upload_interval).await;

                info!("S3 uploader: Starting log upload check");

                // Find and upload log files
                let upload_result = self.find_and_upload_logs().await;

                let should_retry = match upload_result {
                    Ok(uploaded_count) => {
                        if uploaded_count > 0 {
                            info!("S3 uploader: Successfully uploaded {} log file(s)", uploaded_count);
                        } else {
                            info!("S3 uploader: No log files to upload");
                        }
                        false
                    }
                    Err(e) => {
                        error!("S3 uploader: Failed to upload logs: {}", e);
                        warn!("S3 uploader: Will retry in {} hours", self.config.retry_interval_hours);
                        true
                    }
                };

                // Wait for retry interval if upload failed
                if should_retry {
                    sleep(retry_interval).await;
                }
            }
        });
    }

    /// Find and upload log files to S3
    async fn find_and_upload_logs(&self) -> Result<usize, String> {
        let log_dir = Path::new(&self.log_directory);

        if !log_dir.exists() {
            return Ok(0);
        }

        let mut uploaded_count = 0;

        // Read directory entries
        let mut entries = tokio::fs::read_dir(log_dir).await.map_err(|e| e.to_string())?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();

            // Only process .log files
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }

            // Check if this is a rotated (old) log file, not the current one
            // Daily rolling logs are named like: rustbucket.log, rustbucket.log.2025-12-12
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip the current log file (rustbucket.log without date suffix)
            if filename == "rustbucket.log" {
                continue;
            }

            // Check if file is at least 5 minutes old (to ensure it's fully rotated)
            if let Ok(metadata) = tokio::fs::metadata(&path).await {
                if let Ok(modified) = metadata.modified() {
                    let age = SystemTime::now().duration_since(modified).unwrap_or(Duration::from_secs(0));
                    if age < Duration::from_secs(300) {
                        // Skip files newer than 5 minutes
                        continue;
                    }
                }
            }

            // Upload this log file
            match self.upload_log_file(&path).await {
                Ok(_) => {
                    info!("S3 uploader: Successfully uploaded {}", path.display());
                    uploaded_count += 1;

                    // Delete the local file after successful upload if configured
                    if self.config.delete_after_upload {
                        match tokio::fs::remove_file(&path).await {
                            Ok(_) => {
                                info!("S3 uploader: Deleted local file {}", path.display());
                            }
                            Err(e) => {
                                warn!("S3 uploader: Failed to delete local log file {}: {}. File will remain on disk.", path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("S3 uploader: Failed to upload {}: {}", path.display(), e);
                    // Continue trying other files even if one fails
                }
            }
        }

        Ok(uploaded_count)
    }

    /// Upload a single log file to S3
    async fn upload_log_file(&self, file_path: &Path) -> Result<(), String> {
        let client = self.client.as_ref()
            .ok_or("S3 client not initialized")?;

        let bucket_name = self.config.bucket_name.as_ref()
            .ok_or("S3 bucket name not configured".to_string())?;

        // Read the file
        let body = ByteStream::from_path(file_path).await.map_err(|e| e.to_string())?;

        // Construct S3 key: [prefix/]instance_name/filename
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename".to_string())?;

        let s3_key = if let Some(prefix) = &self.config.prefix {
            format!("{}/{}/{}", prefix.trim_end_matches('/'), self.instance_name, filename)
        } else {
            format!("{}/{}", self.instance_name, filename)
        };

        info!("S3 uploader: Uploading {} to s3://{}/{}", file_path.display(), bucket_name, s3_key);

        // Upload to S3
        client
            .put_object()
            .bucket(bucket_name)
            .key(&s3_key)
            .body(body)
            .content_type("text/plain")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Manually trigger an upload (useful for testing)
    pub async fn upload_now(&self) -> Result<usize, String> {
        if !self.is_enabled() {
            return Err("S3 logging is not enabled".to_string());
        }

        self.find_and_upload_logs().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    // Mutex to ensure tests that modify environment variables run serially
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert!(config.bucket_name.is_none());
        assert!(config.region.is_none());
        assert!(config.prefix.is_none());
        assert_eq!(config.upload_interval_hours, 24);
        assert_eq!(config.retry_interval_hours, 24);
        assert!(!config.delete_after_upload);
    }

    #[test]
    fn test_s3_config_clone() {
        let config = S3Config {
            bucket_name: Some("test-bucket".to_string()),
            region: Some("us-east-1".to_string()),
            prefix: Some("logs".to_string()),
            upload_interval_hours: 12,
            retry_interval_hours: 6,
            delete_after_upload: true,
        };

        let cloned = config.clone();
        assert_eq!(config.bucket_name, cloned.bucket_name);
        assert_eq!(config.region, cloned.region);
        assert_eq!(config.prefix, cloned.prefix);
        assert_eq!(config.upload_interval_hours, cloned.upload_interval_hours);
        assert_eq!(config.retry_interval_hours, cloned.retry_interval_hours);
        assert_eq!(config.delete_after_upload, cloned.delete_after_upload);
    }

    #[test]
    fn test_s3_key_generation_without_prefix() {
        let instance_name = "rustbucket-abc123";
        let filename = "rustbucket.log.2025-12-12";

        let key = format!("{}/{}", instance_name, filename);
        assert_eq!(key, "rustbucket-abc123/rustbucket.log.2025-12-12");
    }

    #[test]
    fn test_s3_key_generation_with_prefix() {
        let instance_name = "rustbucket-abc123";
        let filename = "rustbucket.log.2025-12-12";
        let prefix = "honeypot-logs";

        let key_with_prefix = format!("{}/{}/{}", prefix, instance_name, filename);
        assert_eq!(key_with_prefix, "honeypot-logs/rustbucket-abc123/rustbucket.log.2025-12-12");
    }

    #[test]
    fn test_s3_key_generation_with_trailing_slash() {
        let instance_name = "rustbucket-xyz789";
        let filename = "rustbucket.log.2025-12-13";
        let prefix = "production/";

        let key = format!("{}/{}/{}", prefix.trim_end_matches('/'), instance_name, filename);
        assert_eq!(key, "production/rustbucket-xyz789/rustbucket.log.2025-12-13");
    }

    #[tokio::test]
    async fn test_s3_logger_new_no_bucket() {
        let _guard = env_lock().lock().unwrap();

        // Clear environment variables
        env::remove_var("S3_BUCKET_NAME");
        env::remove_var("S3_REGION");

        let instance_name = "test-instance".to_string();
        let result = S3Logger::new(instance_name).await;

        // Should succeed even if S3 is not configured
        assert!(result.is_ok());

        let logger = result.unwrap();
        assert!(!logger.is_enabled());
    }

    #[tokio::test]
    async fn test_s3_logger_new_with_env_vars() {
        let _guard = env_lock().lock().unwrap();

        env::set_var("S3_BUCKET_NAME", "test-bucket");
        env::set_var("S3_REGION", "us-west-2");
        env::set_var("S3_PREFIX", "test-prefix");
        env::set_var("S3_DELETE_AFTER_UPLOAD", "true");

        let instance_name = "test-instance".to_string();
        let result = S3Logger::new(instance_name.clone()).await;

        assert!(result.is_ok());
        let logger = result.unwrap();

        assert_eq!(logger.instance_name, instance_name);
        assert_eq!(logger.config.bucket_name, Some("test-bucket".to_string()));
        assert_eq!(logger.config.region, Some("us-west-2".to_string()));
        assert_eq!(logger.config.prefix, Some("test-prefix".to_string()));
        assert!(logger.config.delete_after_upload);

        // Clean up
        env::remove_var("S3_BUCKET_NAME");
        env::remove_var("S3_REGION");
        env::remove_var("S3_PREFIX");
        env::remove_var("S3_DELETE_AFTER_UPLOAD");
    }

    #[tokio::test]
    async fn test_s3_logger_is_enabled_false_when_no_bucket() {
        let _guard = env_lock().lock().unwrap();

        env::remove_var("S3_BUCKET_NAME");

        let instance_name = "test-instance".to_string();
        let logger = S3Logger::new(instance_name).await.unwrap();

        // Should not be enabled without bucket name
        assert!(!logger.is_enabled());
    }

    #[tokio::test]
    async fn test_s3_logger_upload_now_when_no_bucket() {
        let _guard = env_lock().lock().unwrap();

        env::remove_var("S3_BUCKET_NAME");

        let instance_name = "test-instance".to_string();
        let logger = S3Logger::new(instance_name).await.unwrap();

        let result = logger.upload_now().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not enabled"));
    }

    #[test]
    fn test_s3_config_deserialize() {
        let toml_str = r#"
            bucket_name = "my-bucket"
            region = "us-east-1"
            prefix = "logs"
            upload_interval_hours = 6
            retry_interval_hours = 12
            delete_after_upload = true
        "#;

        let config: S3Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.bucket_name, Some("my-bucket".to_string()));
        assert_eq!(config.region, Some("us-east-1".to_string()));
        assert_eq!(config.prefix, Some("logs".to_string()));
        assert_eq!(config.upload_interval_hours, 6);
        assert_eq!(config.retry_interval_hours, 12);
        assert!(config.delete_after_upload);
    }

    #[test]
    fn test_s3_config_deserialize_minimal() {
        let toml_str = r#"
            bucket_name = ""
            region = "us-east-1"
            prefix = ""
            upload_interval_hours = 24
            retry_interval_hours = 24
            delete_after_upload = false
        "#;

        let config: S3Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.bucket_name, Some("".to_string()));
        assert_eq!(config.upload_interval_hours, 24);
        assert!(!config.delete_after_upload);
    }
}

