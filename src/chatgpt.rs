use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Struct for loading configuration
#[derive(Debug, Deserialize)]
struct OpenAIConfig {
	api_key: String,
	static_messages: StaticMessages,
}

#[derive(Debug, Deserialize)]
struct StaticMessages {
	message1: String,
	message2: String,
	message3: String,
}

#[derive(Serialize)]
struct ChatGPTRequest<'a> {
	model: &'a str,
	messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
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
	role: String,
	content: String,
}

pub struct ChatGPT {
	api_key: String,
	static_messages: StaticMessages,
	client: Client,
}

impl ChatGPT {
	pub fn new(config_file: &str) -> Result<ChatGPT, Box<dyn Error>> {
		// Load configuration from config.toml
		let settings = Config::builder()
			.add_source(File::with_name(config_file))
			.build()?;
		
		let openai_config: OpenAIConfig = settings.try_deserialize()?;
		
		Ok(ChatGPT {
			api_key: openai_config.api_key,
			static_messages: openai_config.static_messages,
			client: Client::new(),
		})
	}
	
	pub async fn send_message(
		&self,
		user_message: &str,
	) -> Result<String, Box<dyn Error>> {
		let url = "https://api.openai.com/v1/chat/completions";
		
		// Prepare the initial static messages
		let mut messages = vec![
			Message {
				role: "system",
				content: &self.static_messages.message1,
			},
			Message {
				role: "system",
				content: &self.static_messages.message2,
			},
			Message {
				role: "system",
				content: &self.static_messages.message3,
			},
			Message {
				role: "user",
				content: user_message,
			},
		];
		
		let request_body = ChatGPTRequest {
			model: "gpt-3.5-turbo",
			messages,
		};
		
		let response = self
			.client
			.post(url)
			.header("Authorization", format!("Bearer {}", self.api_key))
			.json(&request_body)
			.send
