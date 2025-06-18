// src/registration.rs
use serde::{Serialize, Deserialize};
use reqwest;
use tracing::{info, error, warn};
use config; // For configuration loading
use rand::{distributions::Alphanumeric, Rng};

const DEFAULT_REGISTRY_URL: &str = "http://localhost:8080/register";

#[derive(Debug, Deserialize)]
struct RegistrationConfig {
    rustbucket_registry_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    registration: Option<RegistrationConfig>,
}

#[derive(Serialize)]
struct RegistrationPayload<'a> {
    name: &'a str,
    token: &'a str,
}

fn generate_name() -> String {
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("rustbucket-{}", random_suffix)
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

pub async fn register_instance() {
    info!("Attempting to register instance...");

    let config_result: Result<AppConfig, config::ConfigError> = config::Config::builder()
        .add_source(config::File::with_name("Config").required(false))
        .build()
        .and_then(|config_val| config_val.try_deserialize());

    let registry_url = match config_result {
        Ok(app_cfg) => app_cfg
            .registration
            .and_then(|reg_cfg| reg_cfg.rustbucket_registry_url)
            .unwrap_or_else(|| {
                warn!("'rustbucket_registry_url' not found in Config.toml. Using default: {}", DEFAULT_REGISTRY_URL);
                DEFAULT_REGISTRY_URL.to_string()
            }),
        Err(e) => {
            error!("Failed to load configuration: {}. Using default: {}", e, DEFAULT_REGISTRY_URL);
            DEFAULT_REGISTRY_URL.to_string()
        }
    };

    let name = generate_name();
    let token = generate_token();
    info!("Generated name: {}, token: <sensitive>", name); // Avoid logging token directly in real scenarios

    let payload = RegistrationPayload {
        name: &name,
        token: &token,
    };

    let client = reqwest::Client::new();
    info!("Posting registration data to URL: {}", registry_url);

    match client.post(&registry_url).json(&payload).send().await {
        Ok(response) => {
            let status = response.status();
            match response.text().await {
                Ok(body) => {
                    match status {
                        reqwest::StatusCode::OK => {
                            info!("Successfully registered instance with name: {}. Server response: {}", name, body);
                        }
                        reqwest::StatusCode::NOT_FOUND => {
                            error!("Registration failed: Bad URL (404 Not Found) for {}. Server response: {}", registry_url, body);
                        }
                        reqwest::StatusCode::INTERNAL_SERVER_ERROR => { // Typo: INTERNAL_SERVER_ERROR
                            error!("Registration failed: Server error (500 Internal Server Error) at {}. Server response: {}", registry_url, body);
                        }
                        _ => {
                            warn!("Registration attempt to {} returned unexpected status: {}. Server response: {}", registry_url, status, body);
                        }
                    }
                }
                Err(e) => {
                     error!("Failed to read response body from {}: {}. Status was: {}", registry_url, e, status);
                }
            }
        }
        Err(e) => {
            error!("Failed to send registration request to {}: {}", registry_url, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs; // For file operations in test

    #[test]
    fn test_generate_name_format_and_length() {
        let name = generate_name();
        assert!(name.starts_with("rustbucket-"));
        assert_eq!(name.len(), "rustbucket-".len() + 8);
    }

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 32);
    }

    #[tokio::test]
    async fn test_register_instance_runs_with_dummy_config() {
        // Create a dummy Config.toml for this test to control the URL
        let test_config_content = r#"
[registration]
rustbucket_registry_url = "http://localhost:30000/test_register_should_not_exist"
# Using a port that's unlikely to be in use
"#;
        fs::write("Config.toml", test_config_content).expect("Failed to write test Config.toml");

        register_instance().await; // Function should handle errors gracefully

        fs::remove_file("Config.toml").expect("Failed to remove test Config.toml");
        // Test passes if it doesn't panic and handles potential connection error logging.
    }
}
