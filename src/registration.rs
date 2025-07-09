// src/registration.rs

use serde::{Deserialize, Serialize}; // For JSON serialization
use reqwest; // For HTTP requests
use tracing::{info, error, warn}; // For logging
use config; // For configuration management
// We'll need a way to read the configuration.
// Assuming a similar setup to other parts of the project, this might involve a custom config struct
// or using the `config` crate directly. For now, let's add a placeholder for config loading.
// use crate::config::AppConfig; // Placeholder - actual config loading might differ

// use rand::{distributions::Alphanumeric, Rng}; // No longer needed for placeholder

const DEFAULT_REGISTRY_URL: &str = "http://localhost:8080/register"; // Fallback if config fails

#[derive(Debug, Deserialize)]
struct RegistrationConfig {
    rustbucket_registry_url: Option<String>, // Option to handle if not set
}

#[derive(Debug, Deserialize)]
struct AppConfig { // A struct to represent the top-level config structure
    registration: Option<RegistrationConfig>,
}

#[derive(Serialize)]
struct RegistrationPayload<'a> {
    message: &'a str,
}

// Placeholder for the main public function
pub async fn register_instance() {
    info!("Attempting to register instance...");

    // 1. Load configuration
    let config_result: Result<AppConfig, config::ConfigError> = config::Config::builder()
        .add_source(config::File::with_name("Config").required(false)) // Assuming Config.toml
        .build()
        .and_then(|config_val| config_val.try_deserialize());

    let registry_url = match config_result {
        Ok(app_cfg) => app_cfg
            .registration
            .and_then(|reg_cfg| reg_cfg.rustbucket_registry_url)
            .unwrap_or_else(|| {
                warn!("'rustbucket_registry_url' not found in Config.toml or section [registration] missing. Using default: {}", DEFAULT_REGISTRY_URL);
                DEFAULT_REGISTRY_URL.to_string()
            }),
        Err(e) => {
            error!("Failed to load configuration: {}. Using default: {}", e, DEFAULT_REGISTRY_URL);
            DEFAULT_REGISTRY_URL.to_string()
        }
    };

    // 2. Create payload
    let payload = RegistrationPayload {
        message: "Instance registration placeholder",
    };

    // 3. Make HTTP POST request
    let client = reqwest::Client::new();
    info!("Posting registration data to URL: {}", registry_url);

    match client.post(&registry_url).json(&payload).send().await {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await.unwrap_or_else(|_| "Failed to read response body".to_string()); // Get response body for logging if needed

            match status {
                reqwest::StatusCode::OK => { // HTTP 200
                    info!("Successfully sent registration data. Server response: {}", response_text);
                }
                reqwest::StatusCode::NOT_FOUND => { // HTTP 404
                    error!("Registration failed: Bad URL (404 Not Found) for {}. Server response: {}", registry_url, response_text);
                }
                reqwest::StatusCode::INTERNAL_SERVER_ERROR => { // HTTP 500
                    error!("Registration failed: Server error (500 Internal Server Error) at {}. Server response: {}", registry_url, response_text);
                }
                _ => {
                    warn!(
                        "Registration attempt to {} returned unexpected status: {}. Server response: {}",
                        registry_url, status, response_text
                    );
                }
            }
        }
        Err(e) => {
            error!("Failed to send registration request to {}: {}", registry_url, e);
        }
    }
}

// fn generate_name() -> String {
//     let random_suffix: String = rand::thread_rng()
//         .sample_iter(&Alphanumeric)
//         .take(8) // Generate an 8-character random suffix
//         .map(char::from)
//         .collect();
//     format!("rustbucket-{}", random_suffix)
// }

// fn generate_token() -> String {
//     rand::thread_rng()
//         .sample_iter(&Alphanumeric)
//         .take(32) // Generate a 32-character random token
//         .map(char::from)
//         .collect()
// }

#[cfg(test)]
mod tests {
    use super::*;

    // Removed tests for generate_name and generate_token as they are no longer used.

    #[tokio::test]
    async fn test_register_instance_runs() {
        // This is a very basic test to ensure the function can be called
        // and completes without panicking, assuming default or no config.
        // More comprehensive testing would require mocking HTTP requests and config.
        // For now, we rely on the default URL if config is missing.
        // Note: This test might make an actual HTTP call if not carefully managed
        // or if the default URL is reachable. For true unit testing,
        // this would need a mock HTTP server or reqwest mocking.
        // Given the constraints, we'll accept this basic check.

        // To prevent actual HTTP calls during tests, we can shadow the function
        // or use a feature flag, but for now, let's ensure it just runs.
        // We'll add a specific test config file that points to a non-existent local port
        // to ensure network calls fail fast and predictably if they happen.

        // Create a dummy Config.toml for this test
        let test_config_content = r#"
[registration]
rustbucket_registry_url = "http://localhost:12345/test_register"
"#;
        std::fs::write("Config.toml", test_config_content).expect("Failed to write test Config.toml");

        register_instance().await;

        // Clean up the dummy Config.toml
        std::fs::remove_file("Config.toml").expect("Failed to remove test Config.toml");

        // No explicit assert, test passes if it doesn't panic.
        // Log messages can be inspected if test output is captured.
    }
}
