mod handler;
mod prelude;
mod chatgpt;
mod registration;

use crate::prelude::*;
use std::path::Path;
use tracing::{info, error, warn};
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling;
use handler::handle_client;
use chatgpt::ChatGPT;



async fn start_listener(addr: &str) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;    
    // Retrieve the actual address and port the listener is bound to
    let listener_addr = listener.local_addr()?;
    println!("Listening on {}", listener_addr);
    // Instantiate ChatGPT
    let chatgpt = ChatGPT::new().unwrap();
    
    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                let port = client_addr.port();
                println!("New connection on {}: {}", client_addr, client_addr);
                // Spawn a new task to handle the connection asynchronously
                let chatgpt = chatgpt.clone();
                task::spawn(async move {
                    match listener_addr.port() {                        
                        25 => {
                            // Handle connection for port 25
                            info!("Actor attempted to connect to port 25 - SMTP");
                            //@todo: Implement a more realistic SMTP response and don't send this message to ChatGPT
                            let message = "220 mail.example.com ESMTP Postfix (Ubuntu)".to_string();
                            info!("Actor input message: {}", message);
                            handle_client(stream, message, &chatgpt).await;
                        }
                        80 => {
                            // Handle connection for port 80
                            info!("Actor attempted to connect to port 80 - HTTP");
                            //@todo: Implement a more realistic HTTP response and don't send this message to ChatGPT
                            let message = "GET / HTTP/1.1\nHost: example.com".to_string();
                            info!("Actor input message: {}", message);
                            handle_client(stream, message, &chatgpt).await;
                        }
                        21 => {
                            // Handle connection for port 21
                            info!("Actor attempted to connect to port 21 - FTP");
                            //@todo: Implement a more realistic FTP response and don't send this message to ChatGPT
                            let message = "220 (vsFTPd 3.0.3)".to_string();
                            info!("Actor input message: {}", message);
                            handle_client(stream, message, &chatgpt).await;
                        }
                        _ => {
                            // We know our Security Groups are misconfigured if we hit this message.
                            // Open Security Groups should map 1:1 with the ports in this match statement.
                            error!("Actor connected to an unexpected port.");
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

fn check_token_exists() -> bool {
    // Assuming Cargo.lock is in the current working directory's parent if running from target/debug or target/release
    // Or in the current working directory if running with `cargo run`
    let cargo_lock_path = Path::new("Cargo.lock");
    let token_path = cargo_lock_path.with_file_name("token.txt");

    if !token_path.exists() {
        warn!("token.txt not found at {}", token_path.display());
        false
    } else {
        info!("token.txt found at {}", token_path.display());
        true
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    // Set up rolling logs
    let file_appender = rolling::daily("logs", "rustbucket.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    // Initialize tracing subscriber
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new("info"))
        .with_writer(non_blocking.clone()) // Clone for subscriber
        .with_ansi(false)
        .init();
    info!("Tracing initialized");

    // Check for token.txt
    if !check_token_exists() {
        // Output to console as tracing might not be fully set up or listened to yet
        // and this is a critical piece of information for the user.
        println!("CRITICAL: token.txt not found. Rustbucket may not function as expected.");
        // Depending on requirements, might exit here:
        // std::process::exit(1);
    }
    
    // Create tasks for each listener on different ports
    let ports = vec!["0.0.0.0:25", "0.0.0.0:23", "0.0.0.0:21", "0.0.0.0:80"];
    
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    // Helper function to create a dummy file for testing
    fn create_dummy_file(path: &Path) {
        let mut file = File::create(path).expect("Failed to create dummy file");
        writeln!(file, "dummy content").expect("Failed to write to dummy file");
    }

    // Helper function to remove a dummy file after testing
    fn remove_dummy_file(path: &Path) {
        if path.exists() {
            fs::remove_file(path).expect("Failed to remove dummy file");
        }
    }

    #[test]
    fn test_check_token_exists_when_token_present() {
        let token_path = Path::new("token.txt");
        // Ensure no pre-existing token.txt from other tests or runs
        remove_dummy_file(&token_path);
        create_dummy_file(&token_path);

        assert!(check_token_exists(), "check_token_exists should return true when token.txt is present");

        remove_dummy_file(&token_path); // Clean up
    }

    #[test]
    fn test_check_token_exists_when_token_absent() {
        let token_path = Path::new("token.txt");
        remove_dummy_file(&token_path); // Ensure it's absent

        assert!(!check_token_exists(), "check_token_exists should return false when token.txt is absent");
    }
}
