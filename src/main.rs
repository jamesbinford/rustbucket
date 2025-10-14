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
use tokio::signal;



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

    // Register this instance (optional)
    let health_check_handle = registration::register_instance().await;
    
    // Create tasks for each listener on different ports
    let ports = vec!["0.0.0.0:25", "0.0.0.0:23", "0.0.0.0:21", "0.0.0.0:80"];
    
    let mut handles = vec![];
    
    for port in ports {
        let handle = tokio::spawn(async move {
            if let Err(e) = start_listener(port).await {
                error!("Listener for {} failed: {}", port, e);
            }
        });
        handles.push(handle);
    }
    
    // Wait for shutdown signal
    #[cfg(unix)]
    {
        let mut term_signal = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
            }
            _ = term_signal.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown...");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");
            }
        }
    }
    
    // Gracefully shutdown health check if it exists
    if let Some(handle) = health_check_handle {
        info!("Shutting down health check...");
        handle.shutdown().await;
    }
    
    // Abort all listener tasks
    info!("Shutting down listeners...");
    for handle in handles {
        handle.abort();
    }
    
    info!("Shutdown complete");
    
    // Flush logs before shutdown
    drop(_guard);
    Ok(())
}