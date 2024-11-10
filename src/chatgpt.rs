use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use config::{Config, File};

// Struct for loading configuration
#[derive(Debug, Deserialize)]
struct OpenAIConfig {
	api_key: String,
	static_messages: StaticMessages,
}

#[derive(Debug, Deserialize, Clone)]
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
			.add_source(File::with_name(config_file))
			.build()?;
		
		let openai_config: OpenAIConfig = settings.get::<OpenAIConfig>("openai")?;
		
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
			.send()
			.await?;
		
		let response_json: ChatGPTResponse = response.json().await?;
		let reply = &response_json.choices[0].message.content;
		
		Ok(reply.to_string())
	}
}
