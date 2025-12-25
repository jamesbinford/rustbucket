//! OpenAI/ChatGPT Provider

use super::{ChatService, Protocol, get_protocol_prompt};
use crate::config::LlmConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, error};

#[derive(Serialize, Debug)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize, Debug)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize, Debug)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: MessageResponse,
}

#[derive(Deserialize, Debug)]
struct MessageResponse {
    content: String,
}

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    config: LlmConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    /// API key must be set in OPENAI_API_KEY or CHATGPT_API_KEY environment variable
    pub fn new(config: &LlmConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Try OPENAI_API_KEY first, then fall back to CHATGPT_API_KEY for backwards compatibility
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("CHATGPT_API_KEY"))
            .map_err(|_| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "OpenAI API key not found. Set OPENAI_API_KEY or CHATGPT_API_KEY environment variable",
            )) as Box<dyn Error + Send + Sync>)?;

        Ok(Self {
            api_key,
            model: config.model.clone(),
            config: config.clone(),
            client: Client::new(),
        })
    }

    async fn send_request(&self, system_prompt: &str, user_message: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = "https://api.openai.com/v1/chat/completions";

        let messages = vec![
            Message {
                role: "system",
                content: system_prompt,
            },
            Message {
                role: "user",
                content: user_message,
            },
        ];

        let request_body = OpenAIRequest {
            model: &self.model,
            messages,
        };

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!(
                event_type = "llm",
                provider = "openai",
                error = %error_text,
                "OpenAI API error"
            );
            return Err(Box::new(std::io::Error::other(
                "Failed to get a successful response from OpenAI",
            )));
        }

        let response_json: OpenAIResponse = response.json().await?;
        let reply = format!("{}\n", &response_json.choices[0].message.content);

        info!(
            event_type = "llm",
            provider = "openai",
            model = %self.model,
            response = %reply.trim(),
            "OpenAI response received"
        );

        Ok(reply)
    }
}

#[async_trait::async_trait]
impl ChatService for OpenAIProvider {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        // Use static messages for backwards compatibility
        let system_prompt = format!(
            "{}\n{}",
            self.config.static_messages.message1,
            self.config.static_messages.message2
        );

        info!(
            event_type = "llm",
            provider = "openai",
            model = %self.model,
            user_message = %message,
            "OpenAI request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        let system_prompt = get_protocol_prompt(&self.config, protocol);

        info!(
            event_type = "llm",
            provider = "openai",
            model = %self.model,
            protocol = ?protocol,
            user_message = %message,
            "OpenAI protocol-specific request"
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
    fn test_openai_provider_missing_api_key() {
        let _guard = env_lock().lock().unwrap();
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("CHATGPT_API_KEY");

        let config = LlmConfig::default();
        let result = OpenAIProvider::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_openai_provider_with_openai_key() {
        let _guard = env_lock().lock().unwrap();
        env::remove_var("CHATGPT_API_KEY");
        env::set_var("OPENAI_API_KEY", "sk-test123");

        let config = LlmConfig::default();
        let result = OpenAIProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.api_key, "sk-test123");

        env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn test_openai_provider_fallback_to_chatgpt_key() {
        let _guard = env_lock().lock().unwrap();
        env::remove_var("OPENAI_API_KEY");
        env::set_var("CHATGPT_API_KEY", "sk-legacy123");

        let config = LlmConfig::default();
        let result = OpenAIProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.api_key, "sk-legacy123");

        env::remove_var("CHATGPT_API_KEY");
    }

    #[test]
    fn test_openai_request_serialization() {
        let messages = vec![
            Message {
                role: "system",
                content: "You are a helper",
            },
            Message {
                role: "user",
                content: "Hello",
            },
        ];

        let request = OpenAIRequest {
            model: "gpt-4o-mini",
            messages,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4o-mini"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
    }

    #[test]
    fn test_openai_response_deserialization() {
        let json = r#"{
            "choices": [
                {
                    "message": {
                        "content": "Test response"
                    }
                }
            ]
        }"#;

        let response: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices[0].message.content, "Test response");
    }
}
