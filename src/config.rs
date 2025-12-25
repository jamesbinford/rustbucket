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

/// LLM provider selection
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    #[default]
    #[serde(alias = "openai")]
    OpenAI,
    #[serde(alias = "claude", alias = "anthropic")]
    Claude,
    #[serde(alias = "gemini", alias = "google")]
    Gemini,
    #[serde(alias = "ollama")]
    Ollama,
}

/// LLM configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// Which LLM provider to use (default: openai)
    #[serde(default)]
    pub provider: LlmProvider,
    #[serde(default = "default_model")]
    pub model: String,
    pub static_messages: StaticMessages,
    /// Protocol-specific prompts (optional, falls back to static_messages)
    #[serde(default)]
    pub prompts: Option<ProtocolPrompts>,
    /// For Ollama: host URL (default: http://localhost:11434)
    #[serde(default)]
    pub ollama_host: Option<String>,
}

fn default_model() -> String { "gpt-4o-mini".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct StaticMessages {
    pub message1: String,
    pub message2: String,
}

/// Protocol-specific system prompts
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProtocolPrompts {
    pub ssh: Option<String>,
    pub http: Option<String>,
    pub ftp: Option<String>,
    pub smtp: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::default(),
            model: default_model(),
            static_messages: StaticMessages {
                message1: "You are an Ubuntu Server.".to_string(),
                message2: "Respond as an Ubuntu server would. Do not break character.".to_string(),
            },
            prompts: None,
            ollama_host: None,
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
    pub api_key: Option<String>,
}

/// Tarpit configuration for wasting attacker time
#[derive(Debug, Clone, Deserialize)]
pub struct TarpitConfig {
    #[serde(default = "default_tarpit_enabled")]
    pub enabled: bool,
    /// Base delay in milliseconds before each response
    #[serde(default = "default_base_delay")]
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (cap for progressive delays)
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,
    /// Whether delays increase over the session
    #[serde(default = "default_progressive")]
    pub progressive: bool,
    /// Multiplier for progressive delays (e.g., 1.2 = 20% slower each time)
    #[serde(default = "default_multiplier")]
    pub delay_multiplier: f64,
    /// Random jitter percentage (0-100) to make delays less predictable
    #[serde(default = "default_jitter")]
    pub jitter_percent: u32,
}

fn default_tarpit_enabled() -> bool { false }
fn default_base_delay() -> u64 { 100 }
fn default_max_delay() -> u64 { 5000 }
fn default_progressive() -> bool { true }
fn default_multiplier() -> f64 { 1.2 }
fn default_jitter() -> u32 { 20 }

impl Default for TarpitConfig {
    fn default() -> Self {
        Self {
            enabled: default_tarpit_enabled(),
            base_delay_ms: default_base_delay(),
            max_delay_ms: default_max_delay(),
            progressive: default_progressive(),
            delay_multiplier: default_multiplier(),
            jitter_percent: default_jitter(),
        }
    }
}

/// S3 logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct S3Config {
    pub bucket_name: Option<String>,
    pub region: Option<String>,
    pub prefix: Option<String>,
    #[serde(default = "default_upload_interval")]
    pub upload_interval_hours: u64,
    /// Optional: upload interval in minutes (overrides hours if set)
    #[serde(default)]
    pub upload_interval_minutes: Option<u64>,
    #[serde(default = "default_retry_interval")]
    pub retry_interval_hours: u64,
    #[serde(default)]
    pub delete_after_upload: bool,
}

fn default_upload_interval() -> u64 { 1 }
fn default_retry_interval() -> u64 { 24 }

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket_name: None,
            region: None,
            prefix: None,
            upload_interval_hours: 1, // Upload hourly to match hourly log rotation
            upload_interval_minutes: None,
            retry_interval_hours: default_retry_interval(),
            delete_after_upload: false,
        }
    }
}

/// Fingerprint configuration for dynamic banner generation
#[derive(Debug, Clone, Deserialize)]
pub struct FingerprintConfig {
    #[serde(default = "default_fingerprint_enabled")]
    pub enabled: bool,
    /// Override hostname (if None, will be randomly generated)
    #[serde(default)]
    pub hostname: Option<String>,
    /// Override HTTP server header (e.g., "Apache/2.4.57 (Ubuntu)")
    #[serde(default)]
    pub http_server: Option<String>,
    /// Override FTP version (e.g., "vsFTPd 3.0.5")
    #[serde(default)]
    pub ftp_version: Option<String>,
    /// Override SMTP hostname (e.g., "mail.example.com")
    #[serde(default)]
    pub smtp_hostname: Option<String>,
}

fn default_fingerprint_enabled() -> bool { true }

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            enabled: default_fingerprint_enabled(),
            hostname: None,
            http_server: None,
            ftp_version: None,
            smtp_hostname: None,
        }
    }
}

/// Combined application configuration
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub llm: Option<LlmConfig>,
    pub rate_limiting: RateLimitConfig,
    pub tarpit: TarpitConfig,
    pub registration: RegistrationConfig,
    pub s3_logging: S3Config,
    pub fingerprint: FingerprintConfig,
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
                let tarpit: TarpitConfig = config.get("tarpit").unwrap_or_default();
                let registration: RegistrationConfig = config.get("registration").unwrap_or_default();
                let s3_logging: S3Config = config.get("s3_logging").unwrap_or_default();
                let fingerprint: FingerprintConfig = config.get("fingerprint").unwrap_or_default();

                info!(
                    "Configuration loaded: llm={}, rate_limiting={}, tarpit={}, s3_logging={}, fingerprint={}",
                    llm.is_some(),
                    rate_limiting.enabled,
                    tarpit.enabled,
                    s3_logging.bucket_name.is_some(),
                    fingerprint.enabled
                );

                AppConfig {
                    general,
                    llm,
                    rate_limiting,
                    tarpit,
                    registration,
                    s3_logging,
                    fingerprint,
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
        assert_eq!(config.provider, LlmProvider::OpenAI);
        assert_eq!(config.model, "gpt-4o-mini");
        assert!(!config.static_messages.message1.is_empty());
        assert!(config.ollama_host.is_none());
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
        assert_eq!(config.upload_interval_hours, 1); // Hourly to match log rotation
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
