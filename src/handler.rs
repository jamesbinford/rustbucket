use crate::prelude::*;
use crate::chatgpt::ChatGPT;
pub async fn handle_client(mut stream: tokio::net::TcpStream, message: String, chatgpt: &ChatGPT) {
	let mut buffer = [0; 1024];
	loop {
		match stream.read(&mut buffer).await {
			Ok(0) => {
				info!("Connection closed");
				break;
			}
			Ok(n) => {
				// Pass user input to ChatGPT, parse the GPT response and send it back to the user
				let received_data = String::from_utf8_lossy(&buffer[0..n]);
				let response = chatgpt.send_message(&received_data).await.unwrap_or_else(|_| "Error processing request".to_string());
				let response_message = format!("{}", response);
				info!("Received data: {}", received_data);
				info!("Response message: {}", response_message);
				
				if let Err(e) = stream.write_all(response_message.as_bytes()).await {
					println!("Failed to send data: {}", e);
					info!("Failed to write data.");
					break;
				}
			}
			Err(e) => {
				tracing::info!("Failed to read from stream: {}", e);
				break;
			}
		}
	}
}