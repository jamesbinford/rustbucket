//! Configuration for the application
use serde::Deserialize;
use figment::{Figment, providers::{Toml,Format}};

// Structs
#[derive(Debug, Deserialize)]
pub struct Config {
	pub general: General,
	pub ports: Ports,
	pub chatgpt: ChatGPT,
}

#[derive(Debug, Deserialize)]
pub struct General {
	pub log_level: String,
	pub log_directory: String,
	pub verbose: bool,
}

#[derive(Debug, Deserialize)]
pub struct Ports {
	pub ssh: Service,
	pub http: Service,
	pub ftp: Service,
}

#[derive(Debug, Deserialize)]
pub struct Service {
	pub enabled: bool,
	pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct ChatGPT {
	pub api_key: String,
}

impl Config {
	pub fn new() -> Self {
		Figment::new()
			.merge(Toml::file("Config.toml"))
			.extract()
			.expect("Failed to load configuration")
	}
}