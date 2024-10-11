mod chatgpt;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use chatgpt::ChatGPT;
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration and create a shared ChatGPT instance
    let chatgpt = Arc::new(ChatGPT::new("config.toml")?);
    
    // Create a TCP listener bound to a specific address and port
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Honeypot listening on 127.0.0.1:8080");
    
    loop {
        // Accept an incoming connection
        let (mut socket, addr) = listener.accept().await?;
        println!("Accepted connection from {:?}", addr);
        
        // Clone the ChatGPT instance to be used in the async task
        let chatgpt_clone = chatgpt.clone();
        
        // Spawn a new task to handle the connection
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            
            // Read data from the socket
            match socket.read(&mut buffer).await {
                Ok(n) if n == 0 => return, // Connection was closed
                Ok(n) => {
                    // Convert the buffer data to a string
                    let received_data = String::from_utf8_lossy(&buffer[..n]);
                    println!("Received: {}", received_data);
                    
                    // Send the captured data to ChatGPT for a response
                    let response = chatgpt_clone
                        .send_message(&received_data)
                        .await
                        .unwrap_or_else(|e| format!("Error from ChatGPT: {}", e));
                    
                    println!("ChatGPT says: {}", response);
                    
                    // Send the response back to the client
                    if let Err(e) = socket.write_all(response.as_bytes()).await {
                        eprintln!("Failed to write to socket: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                }
            }
        });
    }
}