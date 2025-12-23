use config::{Config, File};
use serde::Deserialize;
use tracing::info;

/// General application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_directory")]
    pub log_directory: String,
    #[serde(default)]
    pub verbose: bool,
}

fn default_log_level() -> String { "info".to_string() }
fn default_log_directory() -> String { "./logs".to_string() }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_directory: default_log_directory(),
            verbose: false,
        }
    }
}

/// LLM (OpenAI/ChatGPT) configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_model")]
    pub model: String,
    pub static_messages: StaticMessages,
}

fn default_model() -> String { "gpt-3.5-turbo".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct StaticMessages {
    pub message1: String,
    pub message2: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            static_messages: StaticMessages {
                message1: "You are an Ubuntu Server.".to_string(),
                message2: "Respond as an Ubuntu server would. Do not break character.".to_string(),
            },
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_rate_limit_enabled")]
    pub enabled: bool,
    #[serde(default = "default_max_connections")]
    pub max_connections_per_ip: u32,
    #[serde(default = "default_rate")]
    pub connection_rate_per_minute: u32,
    #[serde(default = "default_ban_threshold")]
    pub ban_threshold: u32,
    #[serde(default = "default_ban_duration")]
    pub ban_duration_seconds: u64,
    #[serde(default = "default_delay")]
    pub response_delay_ms: (u64, u64),
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub blocklist: Vec<String>,
}

fn default_rate_limit_enabled() -> bool { true }
fn default_max_connections() -> u32 { 5 }
fn default_rate() -> u32 { 10 }
fn default_ban_threshold() -> u32 { 50 }
fn default_ban_duration() -> u64 { 3600 }
fn default_delay() -> (u64, u64) { (100, 300) }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: default_rate_limit_enabled(),
            max_connections_per_ip: default_max_connections(),
            connection_rate_per_minute: default_rate(),
            ban_threshold: default_ban_threshold(),
            ban_duration_seconds: default_ban_duration(),
            response_delay_ms: default_delay(),
            allowlist: Vec::new(),
            blocklist: Vec::new(),
        }
    }
}

/// Registration configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RegistrationConfig {
    pub rustbucket_registry_url: Option<String>,
}

/// S3 logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub bucket_name: Option<String>,
    pub region: Option<String>,
    pub prefix: Option<String>,
    #[serde(default = "default_upload_interval")]
    pub upload_interval_hours: u64,
    #[serde(default = "default_retry_interval")]
    pub retry_interval_hours: u64,
    #[serde(default)]
    pub delete_after_upload: bool,
}

fn default_upload_interval() -> u64 { 24 }
fn default_retry_interval() -> u64 { 24 }

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket_name: None,
            region: None,
            prefix: None,
            upload_interval_hours: default_upload_interval(),
            retry_interval_hours: default_retry_interval(),
            delete_after_upload: false,
        }
    }
}

/// Combined application configuration
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub llm: Option<LlmConfig>,
    pub rate_limiting: RateLimitConfig,
    pub registration: RegistrationConfig,
    pub s3_logging: S3Config,
}

impl AppConfig {
    /// Load configuration from Config.toml file
    /// Returns default values if file doesn't exist or sections are missing
    pub fn load() -> Self {
        Self::load_from_file("Config")
    }

    /// Load configuration from a specific file (for testing)
    pub fn load_from_file(config_name: &str) -> Self {
        let settings = Config::builder()
            .add_source(File::with_name(config_name).required(false))
            .build();

        match settings {
            Ok(config) => {
                let general: GeneralConfig = config.get("general").unwrap_or_default();
                let llm: Option<LlmConfig> = config.get("llm").ok();
                let rate_limiting: RateLimitConfig = config.get("rate_limiting").unwrap_or_default();
                let registration: RegistrationConfig = config.get("registration").unwrap_or_default();
                let s3_logging: S3Config = config.get("s3_logging").unwrap_or_default();

                info!(
                    "Configuration loaded: llm={}, rate_limiting={}, s3_logging={}",
                    llm.is_some(),
                    rate_limiting.enabled,
                    s3_logging.bucket_name.is_some()
                );

                AppConfig {
                    general,
                    llm,
                    rate_limiting,
                    registration,
                    s3_logging,
                }
            }
            Err(e) => {
                info!("Failed to load config file, using defaults: {}", e);
                AppConfig::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_general_config_default() {
        let config = GeneralConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.log_directory, "./logs");
        assert!(!config.verbose);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.model, "gpt-3.5-turbo");
        assert!(!config.static_messages.message1.is_empty());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_connections_per_ip, 5);
        assert_eq!(config.connection_rate_per_minute, 10);
        assert_eq!(config.ban_threshold, 50);
        assert_eq!(config.ban_duration_seconds, 3600);
        assert_eq!(config.response_delay_ms, (100, 300));
    }

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert!(config.bucket_name.is_none());
        assert_eq!(config.upload_interval_hours, 24);
        assert!(!config.delete_after_upload);
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        assert!(config.llm.is_none());
        assert!(config.rate_limiting.enabled);
        assert!(config.s3_logging.bucket_name.is_none());
    }

    #[test]
    fn test_app_config_load_missing_file() {
        let config = AppConfig::load_from_file("nonexistent_config");
        assert!(config.llm.is_none());
        assert!(config.rate_limiting.enabled);
    }
}
