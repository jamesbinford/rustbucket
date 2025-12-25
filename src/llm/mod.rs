//! LLM Provider Module
//!
//! Supports multiple LLM providers:
//! - OpenAI (GPT-4, GPT-4o-mini, etc.)
//! - Claude (Anthropic)
//! - Gemini (Google)
//! - Ollama (local/self-hosted)

mod openai;
mod claude;
mod gemini;
mod ollama;

pub use openai::OpenAIProvider;
pub use claude::ClaudeProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;

use crate::config::{LlmConfig, LlmProvider as ConfigLlmProvider};
use std::error::Error;

/// Protocol types for protocol-specific LLM prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Ssh,
    Http,
    Ftp,
    Smtp,
}

/// Default protocol-specific prompts
pub const DEFAULT_SSH_PROMPT: &str = "You are an Ubuntu 22.04 server with bash shell. Respond exactly as a Linux terminal would, with realistic command output. Do not break character.";
pub const DEFAULT_HTTP_PROMPT: &str = "You are an Apache 2.4 web server on Ubuntu. Return realistic HTTP responses with proper headers and status codes. Do not break character.";
pub const DEFAULT_FTP_PROMPT: &str = "You are a vsftpd FTP server. Use proper FTP response codes (1xx-5xx) and realistic file listings. Do not break character.";
pub const DEFAULT_SMTP_PROMPT: &str = "You are a Postfix SMTP server. Follow RFC 5321 response codes and behave as a realistic mail server. Do not break character.";

/// ChatService trait for sending messages to an LLM backend
#[async_trait::async_trait]
pub trait ChatService: Clone + Send + Sync {
    /// Send a message using the default/global prompt
    async fn send_message(&self, message: &str) -> Result<String, String>;

    /// Send a message with protocol-specific prompt
    async fn send_protocol_message(&self, message: &str, _protocol: Protocol) -> Result<String, String> {
        // Default implementation delegates to send_message for backwards compatibility
        self.send_message(message).await
    }
}

/// Unified LLM provider that wraps all supported providers
/// This enum allows using different providers through a single type
#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI(OpenAIProvider),
    Claude(ClaudeProvider),
    Gemini(GeminiProvider),
    Ollama(OllamaProvider),
}

impl LlmProvider {
    /// Create an LLM provider based on configuration
    pub fn new(config: &LlmConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        match config.provider {
            ConfigLlmProvider::OpenAI => Ok(LlmProvider::OpenAI(OpenAIProvider::new(config)?)),
            ConfigLlmProvider::Claude => Ok(LlmProvider::Claude(ClaudeProvider::new(config)?)),
            ConfigLlmProvider::Gemini => Ok(LlmProvider::Gemini(GeminiProvider::new(config)?)),
            ConfigLlmProvider::Ollama => Ok(LlmProvider::Ollama(OllamaProvider::new(config)?)),
        }
    }
}

#[async_trait::async_trait]
impl ChatService for LlmProvider {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        match self {
            LlmProvider::OpenAI(p) => p.send_message(message).await,
            LlmProvider::Claude(p) => p.send_message(message).await,
            LlmProvider::Gemini(p) => p.send_message(message).await,
            LlmProvider::Ollama(p) => p.send_message(message).await,
        }
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        match self {
            LlmProvider::OpenAI(p) => p.send_protocol_message(message, protocol).await,
            LlmProvider::Claude(p) => p.send_protocol_message(message, protocol).await,
            LlmProvider::Gemini(p) => p.send_protocol_message(message, protocol).await,
            LlmProvider::Ollama(p) => p.send_protocol_message(message, protocol).await,
        }
    }
}

/// Helper to get the protocol-specific prompt
pub fn get_protocol_prompt(config: &LlmConfig, protocol: Protocol) -> String {
    // First check if protocol-specific prompts are configured
    if let Some(ref prompts) = config.prompts {
        let prompt = match protocol {
            Protocol::Ssh => prompts.ssh.as_ref(),
            Protocol::Http => prompts.http.as_ref(),
            Protocol::Ftp => prompts.ftp.as_ref(),
            Protocol::Smtp => prompts.smtp.as_ref(),
        };
        if let Some(p) = prompt {
            return p.clone();
        }
    }

    // Fall back to default prompts for the protocol
    match protocol {
        Protocol::Ssh => DEFAULT_SSH_PROMPT.to_string(),
        Protocol::Http => DEFAULT_HTTP_PROMPT.to_string(),
        Protocol::Ftp => DEFAULT_FTP_PROMPT.to_string(),
        Protocol::Smtp => DEFAULT_SMTP_PROMPT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProtocolPrompts;

    #[test]
    fn test_get_protocol_prompt_defaults() {
        let config = LlmConfig::default();

        assert!(get_protocol_prompt(&config, Protocol::Ssh).contains("Ubuntu"));
        assert!(get_protocol_prompt(&config, Protocol::Http).contains("Apache"));
        assert!(get_protocol_prompt(&config, Protocol::Ftp).contains("vsftpd"));
        assert!(get_protocol_prompt(&config, Protocol::Smtp).contains("Postfix"));
    }

    #[test]
    fn test_get_protocol_prompt_custom() {
        let mut config = LlmConfig::default();
        config.prompts = Some(ProtocolPrompts {
            ssh: Some("Custom SSH prompt".to_string()),
            http: None,
            ftp: None,
            smtp: None,
        });

        assert_eq!(get_protocol_prompt(&config, Protocol::Ssh), "Custom SSH prompt");
        assert!(get_protocol_prompt(&config, Protocol::Http).contains("Apache")); // Falls back to default
    }
}
