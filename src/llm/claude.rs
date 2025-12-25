//! Anthropic Claude Provider

use super::{ChatService, Protocol, get_protocol_prompt};
use crate::config::LlmConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, error};

const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Serialize, Debug)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Serialize, Debug)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize, Debug)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize, Debug)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    config: LlmConfig,
    client: Client,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    /// API key must be set in ANTHROPIC_API_KEY environment variable
    pub fn new(config: &LlmConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Anthropic API key not found. Set ANTHROPIC_API_KEY environment variable",
            )) as Box<dyn Error + Send + Sync>)?;

        // Use configured model or default to claude-3-haiku
        let model = if config.model.starts_with("claude") {
            config.model.clone()
        } else {
            "claude-3-haiku-20240307".to_string()
        };

        Ok(Self {
            api_key,
            model,
            config: config.clone(),
            client: Client::new(),
        })
    }

    async fn send_request(&self, system_prompt: &str, user_message: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = "https://api.anthropic.com/v1/messages";

        let messages = vec![
            ClaudeMessage {
                role: "user",
                content: user_message,
            },
        ];

        let request_body = ClaudeRequest {
            model: &self.model,
            max_tokens: DEFAULT_MAX_TOKENS,
            system: system_prompt,
            messages,
        };

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!(
                event_type = "llm",
                provider = "claude",
                error = %error_text,
                "Claude API error"
            );
            return Err(Box::new(std::io::Error::other(
                "Failed to get a successful response from Claude",
            )));
        }

        let response_json: ClaudeResponse = response.json().await?;
        let reply = if let Some(content) = response_json.content.first() {
            format!("{}\n", content.text)
        } else {
            "\n".to_string()
        };

        info!(
            event_type = "llm",
            provider = "claude",
            model = %self.model,
            response = %reply.trim(),
            "Claude response received"
        );

        Ok(reply)
    }
}

#[async_trait::async_trait]
impl ChatService for ClaudeProvider {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        let system_prompt = format!(
            "{}\n{}",
            self.config.static_messages.message1,
            self.config.static_messages.message2
        );

        info!(
            event_type = "llm",
            provider = "claude",
            model = %self.model,
            user_message = %message,
            "Claude request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        let system_prompt = get_protocol_prompt(&self.config, protocol);

        info!(
            event_type = "llm",
            provider = "claude",
            model = %self.model,
            protocol = ?protocol,
            user_message = %message,
            "Claude protocol-specific request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_claude_provider_missing_api_key() {
        let _guard = env_lock().lock().unwrap();
        env::remove_var("ANTHROPIC_API_KEY");

        let config = LlmConfig::default();
        let result = ClaudeProvider::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_claude_provider_with_api_key() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-test123");

        let config = LlmConfig::default();
        let result = ClaudeProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.api_key, "sk-ant-test123");
        // Should use default Claude model since config has OpenAI model
        assert!(provider.model.starts_with("claude"));

        env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_claude_request_serialization() {
        let messages = vec![
            ClaudeMessage {
                role: "user",
                content: "Hello",
            },
        ];

        let request = ClaudeRequest {
            model: "claude-3-haiku-20240307",
            max_tokens: 1024,
            system: "You are a helper",
            messages,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-3-haiku"));
        assert!(json.contains("max_tokens"));
        assert!(json.contains("system"));
    }

    #[test]
    fn test_claude_response_deserialization() {
        let json = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "Test response"
                }
            ]
        }"#;

        let response: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.content[0].text, "Test response");
    }
}
