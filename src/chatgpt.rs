use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, error};
use crate::config::{LlmConfig, ProtocolPrompts};

/// Protocol types for protocol-specific LLM prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Ssh,
    Http,
    Ftp,
    Smtp,
}

/// Default protocol-specific prompts
const DEFAULT_SSH_PROMPT: &str = "You are an Ubuntu 22.04 server with bash shell. Respond exactly as a Linux terminal would, with realistic command output. Do not break character.";
const DEFAULT_HTTP_PROMPT: &str = "You are an Apache 2.4 web server on Ubuntu. Return realistic HTTP responses with proper headers and status codes. Do not break character.";
const DEFAULT_FTP_PROMPT: &str = "You are a vsftpd FTP server. Use proper FTP response codes (1xx-5xx) and realistic file listings. Do not break character.";
const DEFAULT_SMTP_PROMPT: &str = "You are a Postfix SMTP server. Follow RFC 5321 response codes and behave as a realistic mail server. Do not break character.";

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

#[derive(Serialize, Debug)]
struct ChatGPTRequest<'a> {
	model: &'a str,
	messages: Vec<Message<'a>>,
}

#[derive(Serialize, Debug)]
struct Message<'a> {
	role: &'a str,
	content: &'a str,
}

#[derive(Deserialize, Debug)]
struct ChatGPTResponse {
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
pub struct ChatGPT {
	api_key: String,
	model: String,
	static_messages: crate::config::StaticMessages,
	prompts: Option<ProtocolPrompts>,
	client: Client,
}

impl ChatGPT {
	/// Create a new ChatGPT instance from the provided LlmConfig
	/// API key must be set in CHATGPT_API_KEY environment variable
	pub fn new(llm_config: &LlmConfig) -> Result<ChatGPT, Box<dyn Error>> {
		let api_key = std::env::var("CHATGPT_API_KEY")
			.map_err(|_| Box::new(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				"ChatGPT API key not found in environment variable CHATGPT_API_KEY",
			)))?;

		Ok(ChatGPT {
			api_key,
			model: llm_config.model.clone(),
			static_messages: llm_config.static_messages.clone(),
			prompts: llm_config.prompts.clone(),
			client: Client::new(),
		})
	}

	/// Get the system prompt for a specific protocol
	/// Falls back to static_messages if no protocol-specific prompt is configured
	fn get_protocol_prompt(&self, protocol: Protocol) -> String {
		// First check if protocol-specific prompts are configured
		if let Some(ref prompts) = self.prompts {
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

	/// Send a message with a protocol-specific system prompt
	pub async fn send_protocol_message_impl(
		&self,
		user_message: &str,
		protocol: Protocol,
	) -> Result<String, Box<dyn Error>> {
		let url = "https://api.openai.com/v1/chat/completions";

		let system_prompt = self.get_protocol_prompt(protocol);

		let messages = vec![
			Message {
				role: "system",
				content: &system_prompt,
			},
			Message {
				role: "user",
				content: user_message,
			},
		];

		let request_body = ChatGPTRequest {
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
			error!("Error response from ChatGPT: {}", error_text);
			return Err(Box::new(std::io::Error::other(
				"Failed to get a successful response from ChatGPT",
			)));
		}
		info!(
			model = %self.model,
			protocol = ?protocol,
			user_message = %user_message,
			"ChatGPT protocol-specific request sent"
		);
		let response_json: ChatGPTResponse = response.json().await?;
		let reply = format!("{}\n", &response_json.choices[0].message.content);
		info!(
			response = %reply.trim(),
			"ChatGPT response received"
		);

		Ok(reply.to_string())
	}
	
	pub async fn send_message(
		&self,
		user_message: &str,
	) -> Result<String, Box<dyn Error>> {
		let url = "https://api.openai.com/v1/chat/completions";
		
		// We prompt ChatGPT with several messages before we deliver the user's
		// input. Our goal is to make ChatGPT respond as if it were an Ubuntu
		// server. ChatGPT does this well about 60% of the time so far.
		// Since most "users" that connect to this rustbucket are bots
		// this is an acceptable hit rate.
		let messages = vec![
			Message {
				role: "system",
				content: &self.static_messages.message1,
			},
			Message {
				role: "system",
				content: &self.static_messages.message2,
			},
			Message {
				role: "user",
				content: user_message,
			},
		];
		
		let request_body = ChatGPTRequest {
			model: &self.model,
			messages,
		};
		
		// Send our request to ChatGPT.
		let response = self
			.client
			.post(url)
			.header("Authorization", format!("Bearer {}", self.api_key))
			.json(&request_body)
			.send()
			.await?;
		
		if !response.status().is_success() {
			// If our ChatGPT request was not successful, log and return an error.
			// Most likely issues: invalid API key, rate limiting, quota exceeded, etc.
			let error_text = response.text().await?;
			error!("Error response from ChatGPT: {}", error_text);
			return Err(Box::new(std::io::Error::other(
				"Failed to get a successful response from ChatGPT",
			)));
		}
		info!(
			model = %self.model,
			user_message = %user_message,
			"ChatGPT request sent"
		);
		let response_json: ChatGPTResponse = response.json().await?;
		let reply = format!("{}\n", &response_json.choices[0].message.content);
		info!(
			response = %reply.trim(),
			"ChatGPT response received"
		);
		
		Ok(reply.to_string())
	}
}

#[async_trait::async_trait]
impl ChatService for ChatGPT {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        // Call the inherent send_message method
        match ChatGPT::send_message(self, message).await {
            Ok(response) => Ok(response),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn send_protocol_message(&self, message: &str, protocol: Protocol) -> Result<String, String> {
        // Call the protocol-specific implementation
        match self.send_protocol_message_impl(message, protocol).await {
            Ok(response) => Ok(response),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LlmConfig, StaticMessages};
    use std::env;
    use std::sync::{Mutex, OnceLock};

    // Mutex to ensure tests that modify environment variables run serially
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_static_messages_struct() {
        let messages = StaticMessages {
            message1: "Message 1".to_string(),
            message2: "Message 2".to_string(),
        };

        assert_eq!(messages.message1, "Message 1");
        assert_eq!(messages.message2, "Message 2");
    }

    #[test]
    fn test_llm_config_struct() {
        let messages = StaticMessages {
            message1: "You are a server".to_string(),
            message2: "Respond as a server".to_string(),
        };

        let config = LlmConfig {
            model: "gpt-4".to_string(),
            static_messages: messages,
            prompts: None,
        };

        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.static_messages.message1, "You are a server");
        assert_eq!(config.static_messages.message2, "Respond as a server");
    }

    #[test]
    fn test_chatgpt_request_serialization() {
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

        let request = ChatGPTRequest {
            model: "gpt-3.5-turbo",
            messages,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-3.5-turbo"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
        assert!(json.contains("You are a helper"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_chatgpt_response_deserialization() {
        let json = r#"{
            "choices": [
                {
                    "message": {
                        "content": "Test response"
                    }
                }
            ]
        }"#;

        let response: ChatGPTResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices[0].message.content, "Test response");
    }

    #[test]
    fn test_new_missing_api_key() {
        let _guard = env_lock().lock().unwrap();

        // Ensure CHATGPT_API_KEY is not set
        env::remove_var("CHATGPT_API_KEY");

        let llm_config = LlmConfig::default();
        let result = ChatGPT::new(&llm_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ChatGPT API key"));
    }

    #[test]
    fn test_new_success() {
        let _guard = env_lock().lock().unwrap();

        // Set API key
        env::set_var("CHATGPT_API_KEY", "sk-test123456789");

        let llm_config = LlmConfig {
            model: "gpt-4".to_string(),
            static_messages: StaticMessages {
                message1: "You are an Ubuntu Server.".to_string(),
                message2: "Respond as an Ubuntu server would.".to_string(),
            },
            prompts: None,
        };

        let result = ChatGPT::new(&llm_config);
        assert!(result.is_ok());

        let chatgpt = result.unwrap();
        assert_eq!(chatgpt.api_key, "sk-test123456789");
        assert_eq!(chatgpt.model, "gpt-4");
        assert_eq!(chatgpt.static_messages.message1, "You are an Ubuntu Server.");
        assert_eq!(chatgpt.static_messages.message2, "Respond as an Ubuntu server would.");

        // Clean up
        env::remove_var("CHATGPT_API_KEY");
    }

    #[test]
    fn test_chatgpt_clone() {
        let _guard = env_lock().lock().unwrap();

        env::set_var("CHATGPT_API_KEY", "test_key");

        let llm_config = LlmConfig {
            model: "gpt-3.5-turbo".to_string(),
            static_messages: StaticMessages {
                message1: "Message 1".to_string(),
                message2: "Message 2".to_string(),
            },
            prompts: None,
        };

        let chatgpt = ChatGPT::new(&llm_config).unwrap();
        let cloned = chatgpt.clone();

        assert_eq!(chatgpt.api_key, cloned.api_key);
        assert_eq!(chatgpt.static_messages.message1, cloned.static_messages.message1);
        assert_eq!(chatgpt.static_messages.message2, cloned.static_messages.message2);

        env::remove_var("CHATGPT_API_KEY");
    }

    #[tokio::test]
    async fn test_send_message_request_structure() {
        let _guard = env_lock().lock().unwrap();

        env::set_var("CHATGPT_API_KEY", "sk-test123");

        let llm_config = LlmConfig {
            model: "gpt-3.5-turbo".to_string(),
            static_messages: StaticMessages {
                message1: "System msg 1".to_string(),
                message2: "System msg 2".to_string(),
            },
            prompts: None,
        };

        let chatgpt = ChatGPT::new(&llm_config).unwrap();

        // We can't easily test the actual API call without mocking,
        // but we can verify the struct was created correctly
        assert_eq!(chatgpt.static_messages.message1, "System msg 1");
        assert_eq!(chatgpt.static_messages.message2, "System msg 2");

        env::remove_var("CHATGPT_API_KEY");
    }
}

