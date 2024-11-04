mod chatgpt;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task;
use tracing::{info, error, debug};
use tracing_subscriber::{fmt, EnvFilter};
use tracing_appender::rolling;
use chatgpt::ChatGPT;

async fn handle_client(mut stream: tokio::net::TcpStream, message: String) {
    let mut buffer = [0; 1024];
    
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                tracing::info!("Connection closed");
                break;
            }
            Ok(n) => {
                // Pass user input to ChatGPT, parse the GPT response and send it back to the user
                // @TODO: Implement ChatGPT API
                let received_data = String::from_utf8_lossy(&buffer[0..n]);
                let response_message = format!("{}", message);
                tracing::info!("Received data: {}", received_data);
                tracing::info!("Response message: {}", response_message);
                
                if let Err(e) = stream.write_all(response_message.as_bytes()).await {
                    println!("Failed to send data: {}", e);
                    tracing::info!("Failed to write data.");
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

async fn start_listener(addr: &str) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;    
    // Retrieve the actual address and port the listener is bound to
    let listener_addr = listener.local_addr()?;
    println!("Listening on {}", listener_addr);
    
    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                let port = client_addr.port();
                println!("New connection on {}: {}", client_addr, client_addr);
                // Spawn a new task to handle the connection asynchronously
                task::spawn(async move {
                    match listener_addr.port() {                        
                        25 => {
                            // Handle connection for port 25
                            info!("Actor attempted to connect to port 25 - SMTP");
                            let message = "220 mail.example.com ESMTP Postfix (Ubuntu)".to_string();
                            info!("Actor input message: {}", message);
                            handle_client(stream, message).await;
                        }
                        80 => {
                            // Handle connection for port 80
                            tracing::info!("Actor attempted to connect to port 80 - HTTP");
                            let message = "GET / HTTP/1.1\nHost: example.com".to_string();
                            tracing::info!("Actor input message: {}", message);
                            handle_client(stream, message).await;
                        }
                        21 => {
                            // Handle connection for port 21
                            tracing::info!("Actor attempted to connect to port 21 - FTP");
                            let message = "220 (vsFTPd 3.0.3)".to_string();
                            tracing::info!("Actor input message: {}", message);
                            handle_client(stream, message).await;
                        }
                        _ => {
                            println!("Unexpected port: {}", port);
                        }
                    }
                });
            }
            Err(e) => {
                println!("Failed to accept connection: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    // Set up rolling logs
    let file_appender = rolling::daily("logs", "rustbucket.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    // Initialize tracing subscriber
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    tracing::info!("Tracing initialized");
    
    // Create tasks for each listener on different ports
    let ports = vec!["0.0.0.0:25", "0.0.0.0:23", "0.0.0.0:21"];
    
    let mut handles = vec![];
    
    for port in ports {
        let handle = tokio::spawn(async move {
            start_listener(port).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all listeners to finish (this will run indefinitely)
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Flush logs before shutdown
    drop(_guard);
    Ok(())
}
