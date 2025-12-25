//! Google Gemini Provider

use super::{ChatService, Protocol, get_protocol_prompt};
use crate::config::LlmConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, error};

#[derive(Serialize, Debug)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
    #[serde(rename = "systemInstruction", skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent<'a>>,
}

#[derive(Serialize, Debug)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize, Debug)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize, Debug)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize, Debug)]
struct ResponsePart {
    text: String,
}

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
    model: String,
    config: LlmConfig,
    client: Client,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    /// API key must be set in GEMINI_API_KEY environment variable
    pub fn new(config: &LlmConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Gemini API key not found. Set GEMINI_API_KEY environment variable",
            )) as Box<dyn Error + Send + Sync>)?;

        // Use configured model or default to gemini-1.5-flash
        let model = if config.model.starts_with("gemini") {
            config.model.clone()
        } else {
            "gemini-1.5-flash".to_string()
        };

        Ok(Self {
            api_key,
            model,
            config: config.clone(),
            client: Client::new(),
        })
    }

    async fn send_request(&self, system_prompt: &str, user_message: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model,
            self.api_key
        );

        let request_body = GeminiRequest {
            contents: vec![
                GeminiContent {
                    parts: vec![GeminiPart { text: user_message }],
                },
            ],
            system_instruction: Some(GeminiContent {
                parts: vec![GeminiPart { text: system_prompt }],
            }),
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
                provider = "gemini",
                error = %error_text,
                "Gemini API error"
            );
            return Err(Box::new(std::io::Error::other(
                "Failed to get a successful response from Gemini",
            )));
        }

        let response_json: GeminiResponse = response.json().await?;
        let reply = if let Some(candidate) = response_json.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                format!("{}\n", part.text)
            } else {
                "\n".to_string()
            }
        } else {
            "\n".to_string()
        };

        info!(
            event_type = "llm",
            provider = "gemini",
            model = %self.model,
            response = %reply.trim(),
            "Gemini response received"
        );

        Ok(reply)
    }
}

#[async_trait::async_trait]
impl ChatService for GeminiProvider {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        let system_prompt = format!(
            "{}\n{}",
            self.config.static_messages.message1,
            self.config.static_messages.message2
        );

        info!(
            event_type = "llm",
            provider = "gemini",
            model = %self.model,
            user_message = %message,
            "Gemini request"
        );

        self.send_request(&system_prompt, message)
            .await
            .map_err(|e| e.to_string())
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        let system_prompt = get_protocol_prompt(&self.config, protocol);

        info!(
            event_type = "llm",
            provider = "gemini",
            model = %self.model,
            protocol = ?protocol,
            user_message = %message,
            "Gemini protocol-specific request"
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
    fn test_gemini_provider_missing_api_key() {
        let _guard = env_lock().lock().unwrap();
        env::remove_var("GEMINI_API_KEY");

        let config = LlmConfig::default();
        let result = GeminiProvider::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_gemini_provider_with_api_key() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("GEMINI_API_KEY", "AIza-test123");

        let config = LlmConfig::default();
        let result = GeminiProvider::new(&config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.api_key, "AIza-test123");
        assert!(provider.model.starts_with("gemini"));

        env::remove_var("GEMINI_API_KEY");
    }

    #[test]
    fn test_gemini_request_serialization() {
        let request = GeminiRequest {
            contents: vec![
                GeminiContent {
                    parts: vec![GeminiPart { text: "Hello" }],
                },
            ],
            system_instruction: Some(GeminiContent {
                parts: vec![GeminiPart { text: "You are a helper" }],
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("contents"));
        assert!(json.contains("systemInstruction"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_gemini_response_deserialization() {
        let json = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "text": "Test response"
                            }
                        ]
                    }
                }
            ]
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates[0].content.parts[0].text, "Test response");
    }
}
