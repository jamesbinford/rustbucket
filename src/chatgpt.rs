use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use config::{Config, File};
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
	
	pub fn from_config(_config_file: &str) -> Result<ChatGPT, Box<dyn Error>> {
		// Load configuration from the specified config file
		// Load configuration from the specified config file
		let settings = Config::builder()
			.add_source(File::with_name(Self::CONFIG_FILE)) // Config file is required
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
