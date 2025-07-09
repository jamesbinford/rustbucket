// src/registration.rs

use serde::{Deserialize, Serialize}; // For JSON serialization
use reqwest; // For HTTP requests
use tracing::{info, error, warn}; // For logging
use config; // For configuration management
use std::fs::File; // For file operations
use std::io::Write; // For file operations
// We'll need a way to read the configuration.
// Assuming a similar setup to other parts of the project, this might involve a custom config struct
// or using the `config` crate directly. For now, let's add a placeholder for config loading.
// use crate::config::AppConfig; // Placeholder - actual config loading might differ

// use rand::{distributions::Alphanumeric, Rng}; // No longer needed for placeholder

const DEFAULT_REGISTRY_URL: &str = "http://localhost:8080/register"; // Fallback if config fails
const TOKEN_FILE_PATH: &str = "token.txt";

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

#[derive(Deserialize, Debug)]
struct RegistrationResponse {
    token: String,
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
            if status == reqwest::StatusCode::OK { // HTTP 200
                match response.json::<RegistrationResponse>().await {
                    Ok(reg_response) => {
                        info!("Successfully received registration response with token.");
                        match File::create(TOKEN_FILE_PATH) {
                            Ok(mut file) => {
                                match file.write_all(reg_response.token.as_bytes()) {
                                    Ok(_) => {
                                        info!("Successfully wrote token to {}", TOKEN_FILE_PATH);
                                    }
                                    Err(e) => {
                                        error!("Failed to write token to {}: {}", TOKEN_FILE_PATH, e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to create token file {}: {}", TOKEN_FILE_PATH, e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse registration response JSON: {}. Full response text might provide clues.", e);
                        // Optionally, log the full response text if parsing fails, but be careful with sensitive data.
                        // let response_text = response.text().await.unwrap_or_else(|_| "Failed to read response body".to_string());
                        // error!("Full response text: {}", response_text);
                    }
                }
            } else {
                // Handle non-200 responses
                let response_text = response.text().await.unwrap_or_else(|_| "Failed to read response body".to_string());
                match status {
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

    // Helper function to clean up token.txt
    fn cleanup_token_file() {
        if std::path::Path::new(TOKEN_FILE_PATH).exists() {
            std::fs::remove_file(TOKEN_FILE_PATH).expect("Failed to remove token.txt during cleanup");
        }
    }

    #[tokio::test]
    async fn test_register_instance_runs_without_config_and_no_server() {
        // This test ensures the function runs without panicking when Config.toml is absent
        // and the default registry URL is likely unavailable.
        cleanup_token_file(); // Ensure no leftover token file

        // Ensure no Config.toml exists for this test
        if std::path::Path::new("Config.toml").exists() {
            std::fs::remove_file("Config.toml").expect("Failed to remove pre-existing Config.toml for test");
        }

        register_instance().await; // Should log errors but not panic

        // No specific assert on token.txt as server call is expected to fail.
        // Test passes if it doesn't panic and logs appropriately.
        cleanup_token_file(); // Clean up just in case, though not expected
    }

    #[tokio::test]
    async fn test_register_instance_success_writes_token() {
        // Attempt to initialize tracing for test output. Ignore error if already initialized.
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        // This test requires a mock HTTP server.
        // We'll use `mockito` for this. Add `mockito = "0.31"` to Cargo.toml [dev-dependencies]
        // For now, this test will be structured to use mockito.
        // If direct http mocking isn't immediately feasible, this shows the intent.

        cleanup_token_file();

        let mut server = mockito::Server::new_async().await;
        let mock_url = server.url();

        // Create a dummy Config.toml for this test to use the mock server
        let test_config_content = format!(
            r#"
[registration]
rustbucket_registry_url = "{}/test_register"
"#,
            mock_url
        );
        std::fs::write("Config.toml", test_config_content).expect("Failed to write test Config.toml");

        let mock_token = "test_token_12345";
        let response_body = serde_json::json!({ "token": mock_token }).to_string();

        let _m = server.mock("POST", "/test_register")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&response_body)
            .create_async()
            .await;

        register_instance().await;

        // Check if token.txt was created and contains the correct token
        assert!(std::path::Path::new(TOKEN_FILE_PATH).exists(), "token.txt was not created");
        let token_content = std::fs::read_to_string(TOKEN_FILE_PATH).expect("Failed to read token.txt");
        assert_eq!(token_content, mock_token, "token.txt does not contain the correct token");

        // Clean up
        cleanup_token_file();
        std::fs::remove_file("Config.toml").expect("Failed to remove test Config.toml");
        // Mock server is cleaned up when `server` goes out of scope.
    }
}
