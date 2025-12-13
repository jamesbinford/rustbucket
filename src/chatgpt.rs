use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use config::{Config, File, FileFormat};
use crate::prelude::*;
use crate::handler::ChatService; // Import the new trait

// Struct for loading configuration
#[derive(Debug, Deserialize)]
struct OpenAIConfig {
	static_messages: StaticMessages,
}

#[derive(Debug, Deserialize, Clone)]
struct StaticMessages {
	message1: String,
	message2: String,
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
	static_messages: StaticMessages,
	client: Client,
}

impl ChatGPT {
	const CONFIG_FILE: &'static str = "Config.toml";
	
	pub fn new() -> Result<ChatGPT, Box<dyn Error>> {
		Self::from_config(Self::CONFIG_FILE)
	}
	
	pub fn from_config(config_file: &str) -> Result<ChatGPT, Box<dyn Error>> {
		// Load configuration from the specified config file
		let settings = Config::builder()
			.add_source(File::from(std::path::Path::new(config_file)).format(FileFormat::Toml))
			.build()?;

		let llm_config_from_file: Option<OpenAIConfig> = settings.get("llm").ok();

		let api_key = std::env::var("CHATGPT_API_KEY")
			.map_err(|_| Box::new(std::io::Error::new(
				std::io::ErrorKind::NotFound,
				"ChatGPT API key not found in environment variable CHATGPT_API_KEY",
			)))?;

		let static_messages = llm_config_from_file
			.map(|conf| conf.static_messages)
			.ok_or_else(|| {
				Box::new(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					"Static messages not found in config file",
				))
			})?;
		
		Ok(ChatGPT {
			api_key,
			static_messages,
			client: Client::new(),
		})
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
			model: "gpt-3.5-turbo", //@todo Move this to config.rs
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
			return Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::Other,
				"Failed to get a successful response from ChatGPT",
			)));
		}
		//@todo Change the format of the log message to be more parseable.
		info!("We sent this to ChatGPT: {:?}", request_body);
		let response_json: ChatGPTResponse = response.json().await?;
		let reply = format!("{}\n", &response_json.choices[0].message.content);
		info!("ChatGPT responded: {}", reply);
		
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;
    use std::io::Write;
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
    fn test_openai_config_struct() {
        let messages = StaticMessages {
            message1: "You are a server".to_string(),
            message2: "Respond as a server".to_string(),
        };

        let config = OpenAIConfig {
            static_messages: messages,
        };

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
    fn test_from_config_missing_api_key() {
        let _guard = env_lock().lock().unwrap();

        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[llm]").unwrap();
        writeln!(temp_file, "[llm.static_messages]").unwrap();
        writeln!(temp_file, "message1 = \"Test message 1\"").unwrap();
        writeln!(temp_file, "message2 = \"Test message 2\"").unwrap();
        temp_file.flush().unwrap();

        // Ensure CHATGPT_API_KEY is not set
        env::remove_var("CHATGPT_API_KEY");

        let result = ChatGPT::from_config(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ChatGPT API key"));
    }

    #[test]
    fn test_from_config_missing_static_messages() {
        let _guard = env_lock().lock().unwrap();

        // Create a temporary config file without static messages
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[other]").unwrap();
        writeln!(temp_file, "key = \"value\"").unwrap();
        temp_file.flush().unwrap();

        // Set API key
        env::set_var("CHATGPT_API_KEY", "test_key");

        let result = ChatGPT::from_config(temp_file.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Static messages"));

        // Clean up
        env::remove_var("CHATGPT_API_KEY");
    }

    #[test]
    fn test_from_config_success() {
        let _guard = env_lock().lock().unwrap();

        // Create a proper config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[llm]").unwrap();
        writeln!(temp_file, "[llm.static_messages]").unwrap();
        writeln!(temp_file, "message1 = \"You are an Ubuntu Server.\"").unwrap();
        writeln!(temp_file, "message2 = \"Respond as an Ubuntu server would.\"").unwrap();
        temp_file.flush().unwrap();

        // Set API key
        env::set_var("CHATGPT_API_KEY", "sk-test123456789");

        let result = ChatGPT::from_config(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());

        let chatgpt = result.unwrap();
        assert_eq!(chatgpt.api_key, "sk-test123456789");
        assert_eq!(chatgpt.static_messages.message1, "You are an Ubuntu Server.");
        assert_eq!(chatgpt.static_messages.message2, "Respond as an Ubuntu server would.");

        // Clean up
        env::remove_var("CHATGPT_API_KEY");
    }

    #[test]
    fn test_chatgpt_clone() {
        let _guard = env_lock().lock().unwrap();

        env::set_var("CHATGPT_API_KEY", "test_key");

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[llm]").unwrap();
        writeln!(temp_file, "[llm.static_messages]").unwrap();
        writeln!(temp_file, "message1 = \"Message 1\"").unwrap();
        writeln!(temp_file, "message2 = \"Message 2\"").unwrap();
        temp_file.flush().unwrap();

        let chatgpt = ChatGPT::from_config(temp_file.path().to_str().unwrap()).unwrap();
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

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[llm]").unwrap();
        writeln!(temp_file, "[llm.static_messages]").unwrap();
        writeln!(temp_file, "message1 = \"System msg 1\"").unwrap();
        writeln!(temp_file, "message2 = \"System msg 2\"").unwrap();
        temp_file.flush().unwrap();

        let chatgpt = ChatGPT::from_config(temp_file.path().to_str().unwrap()).unwrap();

        // We can't easily test the actual API call without mocking,
        // but we can verify the struct was created correctly
        assert_eq!(chatgpt.static_messages.message1, "System msg 1");
        assert_eq!(chatgpt.static_messages.message2, "System msg 2");

        env::remove_var("CHATGPT_API_KEY");
    }
}

