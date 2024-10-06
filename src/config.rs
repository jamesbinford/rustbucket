//! Configuration for the application

//. Dependencies
use std::fs;
use std::io::Error as IoError;

//. Structs
#[derive(Debug)]
pub struct Config {
	pub ssh: u16,
}

impl Config {
	pub fn new() -> Self {
		
	}
}