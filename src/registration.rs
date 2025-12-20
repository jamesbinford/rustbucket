// src/registration.rs

use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::time::Duration;
use std::env;
use std::fs;
use std::path::Path;

/// File path for persisting instance identity across restarts
const IDENTITY_FILE: &str = ".rustbucket_identity";

#[derive(Debug, Deserialize)]
struct RegistrationConfig {
    rustbucket_registry_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    registration: Option<RegistrationConfig>,
}

/// Persistent identity for this rustbucket instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceIdentity {
    pub name: String,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_prefix: Option<String>,
}

/// System information collected for registration
#[derive(Debug, Clone)]
struct SystemInfo {
    ip_address: String,
    operating_system: String,
    cpu_usage: Option<String>,
    memory_usage: Option<String>,
    disk_space: Option<String>,
    uptime: Option<String>,
    connections: Option<String>,
}

/// S3 configuration returned by the registry
#[derive(Debug, Clone, Deserialize)]
struct S3ConfigResponse {
    s3_bucket: String,
    s3_region: String,
    s3_prefix: String,
}

/// Registration response from the registry
#[derive(Debug, Clone, Deserialize)]
struct RegistrationResponse {
    status: String,
    #[serde(default)]
    s3_config: Option<S3ConfigResponse>,
}

#[derive(Serialize, Clone, Debug)]
struct RegistrationPayload {
    name: String,
    ip_address: String,
    operating_system: String,
    token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu_usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_usage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_space: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uptime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connections: Option<String>,
}

/// Load registration URL from environment variables and config file
fn load_registration_url() -> Option<String> {
    // Check environment variable first
    if let Ok(url) = env::var("RUSTBUCKET_REGISTRY_URL") {
        info!("Using registry URL from environment variable: {}", url);
        return Some(url);
    }

    // Fallback to config file
    let config_result = config::Config::builder()
        .add_source(config::File::with_name("Config").required(false))
        .build()
        .and_then(|config_val| config_val.try_deserialize::<AppConfig>());

    match config_result {
        Ok(app_cfg) => {
            if let Some(url) = app_cfg.registration.and_then(|reg| reg.rustbucket_registry_url) {
                info!("Using registry URL from Config.toml: {}", url);
                Some(url)
            } else {
                info!("No rustbucket_registry_url configured. Registration is optional - skipping.");
                None
            }
        }
        Err(e) => {
            warn!("Failed to load configuration: {}. Skipping registration.", e);
            None
        }
    }
}

/// Collect system information for registration
async fn collect_system_info() -> SystemInfo {
    info!("Gathering system information...");

    SystemInfo {
        ip_address: get_public_ip().await,
        operating_system: get_operating_system(),
        cpu_usage: get_cpu_usage(),
        memory_usage: get_memory_usage(),
        disk_space: get_disk_space(),
        uptime: get_uptime(),
        connections: get_connections(),
    }
}

/// Send registration request to the registry
/// Returns S3 config if provided in the response
async fn send_registration_request(
    registry_url: &str,
    name: &str,
    token: &str,
    system_info: &SystemInfo,
) -> Option<S3ConfigResponse> {
    let payload = RegistrationPayload {
        name: name.to_string(),
        ip_address: system_info.ip_address.clone(),
        operating_system: system_info.operating_system.clone(),
        token: token.to_string(),
        cpu_usage: system_info.cpu_usage.clone(),
        memory_usage: system_info.memory_usage.clone(),
        disk_space: system_info.disk_space.clone(),
        uptime: system_info.uptime.clone(),
        connections: system_info.connections.clone(),
    };

    // Ensure URL ends with trailing slash for Django compatibility
    let normalized_url = if registry_url.ends_with('/') {
        registry_url.to_string()
    } else {
        format!("{}/", registry_url)
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");

    info!("Posting registration data to URL: {}", normalized_url);
    info!("Registration payload: {:?}", payload);

    match client.post(&normalized_url).json(&payload).send().await {
        Ok(response) => {
            let status = response.status();
            let response_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            match status {
                reqwest::StatusCode::OK => {
                    info!("Successfully registered instance '{}'. Server response: {}", name, response_text);

                    // Parse response to extract S3 config
                    match serde_json::from_str::<RegistrationResponse>(&response_text) {
                        Ok(reg_response) => {
                            if let Some(ref s3_config) = reg_response.s3_config {
                                info!(
                                    "Received S3 config from registry: bucket={}, region={}, prefix={}",
                                    s3_config.s3_bucket, s3_config.s3_region, s3_config.s3_prefix
                                );
                            }
                            reg_response.s3_config
                        }
                        Err(e) => {
                            warn!("Failed to parse registration response: {}. S3 config not available.", e);
                            None
                        }
                    }
                }
                reqwest::StatusCode::NOT_FOUND => {
                    error!("Registration failed: Bad URL (404 Not Found) for {}. Server response: {}", normalized_url, response_text);
                    None
                }
                reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                    error!("Registration failed: Server error (500) at {}. Server response: {}", normalized_url, response_text);
                    None
                }
                _ => {
                    warn!(
                        "Registration attempt to {} returned unexpected status: {}. Server response: {}",
                        normalized_url, status, response_text
                    );
                    None
                }
            }
        }
        Err(e) => {
            error!("Failed to send registration request to {}: {}", normalized_url, e);
            error!("Error details - is_timeout: {}, is_connect: {}, is_request: {}",
                e.is_timeout(), e.is_connect(), e.is_request());
            if let Some(url_err) = e.url() {
                error!("URL that caused the error: {}", url_err);
            }
            None
        }
    }
}


/// Load existing identity from file or create a new one.
/// This ensures the same identity is used across restarts.
fn load_or_create_identity() -> InstanceIdentity {
    let identity_path = Path::new(IDENTITY_FILE);

    // Try to load existing identity
    if identity_path.exists() {
        match fs::read_to_string(identity_path) {
            Ok(contents) => {
                match serde_json::from_str::<InstanceIdentity>(&contents) {
                    Ok(identity) => {
                        info!(
                            name = %identity.name,
                            "Loaded existing instance identity from {}",
                            IDENTITY_FILE
                        );
                        return identity;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse identity file, generating new identity: {}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read identity file, generating new identity: {}",
                    e
                );
            }
        }
    }

    // Generate new identity
    let identity = InstanceIdentity {
        name: generate_name(),
        token: generate_token(),
        s3_bucket: None,
        s3_region: None,
        s3_prefix: None,
    };

    // Save to file for next restart
    match serde_json::to_string_pretty(&identity) {
        Ok(json) => {
            if let Err(e) = fs::write(identity_path, json) {
                warn!("Failed to save identity file: {}", e);
            } else {
                info!(
                    name = %identity.name,
                    "Created and saved new instance identity to {}",
                    IDENTITY_FILE
                );
            }
        }
        Err(e) => {
            warn!("Failed to serialize identity: {}", e);
        }
    }

    identity
}

/// Save identity to file
fn save_identity(identity: &InstanceIdentity) {
    let identity_path = Path::new(IDENTITY_FILE);
    match serde_json::to_string_pretty(identity) {
        Ok(json) => {
            if let Err(e) = fs::write(identity_path, json) {
                warn!("Failed to save identity file: {}", e);
            } else {
                info!(
                    name = %identity.name,
                    "Saved instance identity to {}",
                    IDENTITY_FILE
                );
            }
        }
        Err(e) => {
            warn!("Failed to serialize identity: {}", e);
        }
    }
}

/// Register instance and return identity with S3 config if available
pub async fn register_instance() -> InstanceIdentity {
    info!("Checking registration configuration...");

    // Load or create persistent identity (survives restarts)
    let mut identity = load_or_create_identity();
    info!(
        name = %identity.name,
        "Using instance identity"
    );

    // Load registry URL
    let registry_url = match load_registration_url() {
        Some(url) => url,
        None => {
            info!("No registry URL configured. Skipping registration.");
            return identity;
        }
    };

    // Collect system information
    let system_info = collect_system_info().await;

    // Attempt registration
    info!("Attempting to register instance with URL: {}", registry_url);
    let s3_config = send_registration_request(
        &registry_url,
        &identity.name,
        &identity.token,
        &system_info,
    )
    .await;

    // Update identity with S3 config if received from registry
    if let Some(config) = s3_config {
        identity.s3_bucket = Some(config.s3_bucket);
        identity.s3_region = Some(config.s3_region);
        identity.s3_prefix = Some(config.s3_prefix);

        // Save updated identity to file
        save_identity(&identity);
        info!("Updated identity with S3 configuration from registry");
    }

    identity
}

fn generate_name() -> String {
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8) // Generate an 8-character random suffix
        .map(char::from)
        .collect();
    format!("rustbucket-{}", random_suffix)
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32) // Generate a 32-character random token
        .map(char::from)
        .collect()
}

async fn get_public_ip() -> String {
    // Try to get the public IP address by querying an external service
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Try multiple services in case one is down
    let services = vec![
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://ifconfig.me/ip",
    ];

    for service in services {
        if let Ok(response) = client.get(service).send().await {
            if let Ok(text) = response.text().await {
                let ip = text.trim().to_string();
                if !ip.is_empty() {
                    info!("Retrieved public IP address: {}", ip);
                    return ip;
                }
            }
        }
    }

    warn!("Failed to retrieve public IP address, using placeholder");
    "0.0.0.0".to_string()
}

fn get_operating_system() -> String {
    // Get OS information from the standard library
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("{} ({})", os, arch)
}

fn get_cpu_usage() -> Option<String> {
    // For now, return None - would require sysinfo crate or platform-specific code
    None
}

fn get_memory_usage() -> Option<String> {
    // For now, return None - would require sysinfo crate or platform-specific code
    None
}

fn get_disk_space() -> Option<String> {
    // For now, return None - would require sysinfo crate or platform-specific code
    None
}

fn get_uptime() -> Option<String> {
    // For now, return None - uptime would be tracked from service start
    None
}

fn get_connections() -> Option<String> {
    // For now, return None - would require tracking active connections
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registration_payload_serialization() {
        let payload = RegistrationPayload {
            name: "rustbucket-abc12345".to_string(),
            ip_address: "192.168.1.100".to_string(),
            operating_system: "linux (x86_64)".to_string(),
            token: "test_token_32_chars_long_string".to_string(),
            cpu_usage: Some("25%".to_string()),
            memory_usage: Some("512MB".to_string()),
            disk_space: Some("50GB".to_string()),
            uptime: Some("3600".to_string()),
            connections: Some("5".to_string()),
        };

        let json_result = serde_json::to_string(&payload);
        assert!(json_result.is_ok(), "Payload should serialize to JSON");

        let json_str = json_result.unwrap();
        assert!(json_str.contains("rustbucket-abc12345"), "JSON should contain the instance name");
        assert!(json_str.contains("test_token_32_chars_long_string"), "JSON should contain the token");
        assert!(json_str.contains("192.168.1.100"), "JSON should contain the IP address");
        assert!(json_str.contains("linux (x86_64)"), "JSON should contain the operating system");
    }

    #[test]
    fn test_generate_name_format() {
        let name = generate_name();
        assert!(name.starts_with("rustbucket-"), "Generated name should start with rustbucket-");
        assert_eq!(name.len(), 19, "Generated name should be 19 characters long (rustbucket- + 8 chars)");

        let suffix = &name[11..];
        assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()), "Name suffix should be alphanumeric");
    }

    #[test]
    fn test_generate_token_format() {
        let token = generate_token();
        assert_eq!(token.len(), 32, "Generated token should be 32 characters long");
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric()), "Token should be alphanumeric");
    }

    #[test]
    fn test_generate_functions_uniqueness() {
        let name1 = generate_name();
        let name2 = generate_name();
        let token1 = generate_token();
        let token2 = generate_token();

        assert_ne!(name1, name2, "Generated names should be unique");
        assert_ne!(token1, token2, "Generated tokens should be unique");
    }

    #[test]
    fn test_system_info_collection() {
        let os = get_operating_system();
        assert!(!os.is_empty(), "Operating system string should not be empty");
        assert!(os.contains("("), "Operating system should include architecture");
    }

    #[test]
    fn test_instance_identity_serialization() {
        let identity = InstanceIdentity {
            name: "rustbucket-test1234".to_string(),
            token: "abcdef1234567890abcdef1234567890".to_string(),
            s3_bucket: None,
            s3_region: None,
            s3_prefix: None,
        };

        let json = serde_json::to_string(&identity).unwrap();
        assert!(json.contains("rustbucket-test1234"));
        assert!(json.contains("abcdef1234567890abcdef1234567890"));
        // S3 fields should be omitted when None
        assert!(!json.contains("s3_bucket"));

        let parsed: InstanceIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, identity.name);
        assert_eq!(parsed.token, identity.token);
    }

    #[test]
    fn test_instance_identity_with_s3_config() {
        let identity = InstanceIdentity {
            name: "rustbucket-test1234".to_string(),
            token: "abcdef1234567890abcdef1234567890".to_string(),
            s3_bucket: Some("my-bucket".to_string()),
            s3_region: Some("us-west-2".to_string()),
            s3_prefix: Some("logs/rustbucket-test1234/".to_string()),
        };

        let json = serde_json::to_string(&identity).unwrap();
        assert!(json.contains("my-bucket"));
        assert!(json.contains("us-west-2"));
        assert!(json.contains("logs/rustbucket-test1234/"));

        let parsed: InstanceIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.s3_bucket, Some("my-bucket".to_string()));
        assert_eq!(parsed.s3_region, Some("us-west-2".to_string()));
        assert_eq!(parsed.s3_prefix, Some("logs/rustbucket-test1234/".to_string()));
    }

    #[test]
    fn test_instance_identity_deserialization() {
        // Test backward compatibility - old format without S3 fields
        let json = r#"{"name":"rustbucket-abcd1234","token":"token123456789012345678901234"}"#;
        let identity: InstanceIdentity = serde_json::from_str(json).unwrap();

        assert_eq!(identity.name, "rustbucket-abcd1234");
        assert_eq!(identity.token, "token123456789012345678901234");
        assert_eq!(identity.s3_bucket, None);
        assert_eq!(identity.s3_region, None);
        assert_eq!(identity.s3_prefix, None);
    }

    #[test]
    fn test_registration_response_parsing() {
        // Test parsing response with S3 config
        let json = r#"{"status":"success","s3_config":{"s3_bucket":"honeypot-logs","s3_region":"us-east-1","s3_prefix":"logs/rustbucket-test/"}}"#;
        let response: RegistrationResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "success");
        assert!(response.s3_config.is_some());
        let s3_config = response.s3_config.unwrap();
        assert_eq!(s3_config.s3_bucket, "honeypot-logs");
        assert_eq!(s3_config.s3_region, "us-east-1");
        assert_eq!(s3_config.s3_prefix, "logs/rustbucket-test/");
    }

    #[test]
    fn test_registration_response_without_s3() {
        // Test parsing response without S3 config (backward compatibility)
        let json = r#"{"status":"success"}"#;
        let response: RegistrationResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "success");
        assert!(response.s3_config.is_none());
    }
}