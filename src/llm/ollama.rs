//! Ollama Provider (local/self-hosted LLMs)

use super::{ChatService, Protocol, get_protocol_prompt};
use crate::config::LlmConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, error};

const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL: &str = "llama3.2";

#[derive(Serialize, Debug)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    stream: bool,
}

#[derive(Serialize, Debug)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    host: String,
    model: String,
    config: LlmConfig,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    /// No API key required - connects to local Ollama instance
    pub fn new(config: &LlmConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let host = config.ollama_host.clone()
            .unwrap_or_else(|| DEFAULT_OLLAMA_HOST.to_string());

        // Use configured model or default to llama3.2
        let model = if config.model.starts_with("llama") ||
                       config.model.starts_with("mistral") ||
                       config.model.starts_with("codellama") ||
                       config.model.starts_with("phi") ||
                       config.model.starts_with("gemma") ||
                       config.model.starts_with("qwen") {
            config.model.clone()
        } else {
            DEFAULT_OLLAMA_MODEL.to_string()
        };

        info!(
            event_type = "llm",
            provider = "ollama",
            host = %host,
            model = %model,
            "Ollama provider initialized"
        );

        Ok(Self {
            host,
            model,
            config: config.clone(),
            client: Client::new(),
        })
    }

    async fn send_request(&self, system_prompt: &str, user_message: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/api/chat", self.host);

        let messages = vec![
            OllamaMessage {
                role: "system",
                content: system_prompt,
            },
            OllamaMessage {
                role: "user",
                content: user_message,
            },
        ];

        let request_body = OllamaRequest {
            model: &self.model,
            messages,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!(
                event_type = "llm",
                provider = "ollama",
                error = %error_text,
                "Ollama API error"
            );
            return Err(Box::new(std::io::Error::other(
                "Failed to get a successful response from Ollama",
            )));
        }

        let response_json: OllamaResponse = response.json().await?;
        let reply = format!("{}\n", response_json.message.content);

        info!(
            event_type = "llm",
            provider = "ollama",
            model = %self.model,
            response = %reply.trim(),
            "Ollama response received"
        );

        Ok(reply)
    }
}

#[async_trait::async_trait]
impl ChatService for OllamaProvider {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        let system_prompt = format!(
            "{}\n{}",
            self.config.static_messages.message1,
            self.config.static_messages.message2
        );

        info!(
            event_type = "llm",
            provider = "ollama",
            model = %self.model,
            user_message = %message,
            "Ollama request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        let system_prompt = get_protocol_prompt(&self.config, protocol);

        info!(
            event_type = "llm",
            provider = "ollama",
            model = %self.model,
            protocol = ?protocol,
            user_message = %message,
            "Ollama protocol-specific request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_provider_default_host() {
        let config = LlmConfig::default();
        let result = OllamaProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.host, DEFAULT_OLLAMA_HOST);
        assert_eq!(provider.model, DEFAULT_OLLAMA_MODEL);
    }

    #[test]
    fn test_ollama_provider_custom_host() {
        let mut config = LlmConfig::default();
        config.ollama_host = Some("http://192.168.1.100:11434".to_string());
        config.model = "mistral".to_string();

        let result = OllamaProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.host, "http://192.168.1.100:11434");
        assert_eq!(provider.model, "mistral");
    }

    #[test]
    fn test_ollama_request_serialization() {
        let messages = vec![
            OllamaMessage {
                role: "system",
                content: "You are a helper",
            },
            OllamaMessage {
                role: "user",
                content: "Hello",
            },
        ];

        let request = OllamaRequest {
            model: "llama3.2",
            messages,
            stream: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("llama3.2"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
        assert!(json.contains("\"stream\":false"));
    }

    #[test]
    fn test_ollama_response_deserialization() {
        let json = r#"{
            "message": {
                "role": "assistant",
                "content": "Test response"
            }
        }"#;

        let response: OllamaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.message.content, "Test response");
    }
}
