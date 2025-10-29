// src/registration.rs

use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::time::Duration;
use std::env;

#[derive(Debug, Deserialize)]
struct RegistrationConfig {
    rustbucket_registry_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    registration: Option<RegistrationConfig>,
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
async fn send_registration_request(
    registry_url: &str,
    name: &str,
    token: &str,
    system_info: &SystemInfo,
) -> bool {
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

    let client = reqwest::Client::new();
    info!("Posting registration data to URL: {}", registry_url);

    match client.post(registry_url).json(&payload).send().await {
        Ok(response) => {
            let status = response.status();
            let response_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            match status {
                reqwest::StatusCode::OK => {
                    info!("Successfully registered instance '{}'. Server response: {}", name, response_text);
                    true
                }
                reqwest::StatusCode::NOT_FOUND => {
                    error!("Registration failed: Bad URL (404 Not Found) for {}. Server response: {}", registry_url, response_text);
                    false
                }
                reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                    error!("Registration failed: Server error (500) at {}. Server response: {}", registry_url, response_text);
                    false
                }
                _ => {
                    warn!(
                        "Registration attempt to {} returned unexpected status: {}. Server response: {}",
                        registry_url, status, response_text
                    );
                    false
                }
            }
        }
        Err(e) => {
            error!("Failed to send registration request to {}: {}", registry_url, e);
            false
        }
    }
}


pub async fn register_instance() {
    info!("Checking registration configuration...");

    // Load registry URL
    let registry_url = match load_registration_url() {
        Some(url) => url,
        None => {
            info!("No registry URL configured. Skipping registration.");
            return;
        }
    };

    // Generate instance identity
    let instance_name = generate_name();
    let instance_token = generate_token();
    info!("Generated instance name: {}", instance_name);
    info!("Generated instance token: {}", instance_token);

    // Collect system information
    let system_info = collect_system_info().await;

    // Attempt registration
    info!("Attempting to register instance with URL: {}", registry_url);
    send_registration_request(
        &registry_url,
        &instance_name,
        &instance_token,
        &system_info,
    )
    .await;
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
}