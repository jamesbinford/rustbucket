mod handler;
mod prelude;
mod chatgpt;
mod registration;
mod protocols;
mod s3_logger;

use crate::prelude::*;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling;
use chatgpt::ChatGPT;
use protocols::{ProtocolHandler, LlmEscalationConfig};
use protocols::ssh::SshHandler;
use protocols::http::HttpHandler;
use protocols::ftp::FtpHandler;
use protocols::smtp::SmtpHandler;
use s3_logger::S3Logger;
use rand::distributions::Alphanumeric;
use rand::Rng;



async fn start_listener(addr: &str) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    // Retrieve the actual address and port the listener is bound to
    let listener_addr = listener.local_addr()?;
    println!("Listening on {}", listener_addr);
    // Instantiate ChatGPT
    let chatgpt = ChatGPT::new().unwrap();

    // Create LLM escalation config
    let llm_config = LlmEscalationConfig::default();

    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                println!("New connection on {} from {}", listener_addr, client_addr);
                // Spawn a new task to handle the connection asynchronously
                let chatgpt = chatgpt.clone();
                let llm_config = llm_config.clone();

                task::spawn(async move {
                    match listener_addr.port() {
                        22 => {
                            // SSH honeypot
                            info!("Actor attempted to connect to port 22 - SSH");
                            let mut handler = SshHandler::new(chatgpt, llm_config);
                            handler.handle_connection(stream).await;
                        }
                        25 => {
                            // SMTP honeypot
                            info!("Actor attempted to connect to port 25 - SMTP");
                            let mut handler = SmtpHandler::new(chatgpt, llm_config);
                            handler.handle_connection(stream).await;
                        }
                        80 => {
                            // HTTP honeypot
                            info!("Actor attempted to connect to port 80 - HTTP");
                            let mut handler = HttpHandler::new(chatgpt, llm_config);
                            handler.handle_connection(stream).await;
                        }
                        21 => {
                            // FTP honeypot
                            info!("Actor attempted to connect to port 21 - FTP");
                            let mut handler = FtpHandler::new(chatgpt, llm_config);
                            handler.handle_connection(stream).await;
                        }
                        _ => {
                            // We know our Security Groups are misconfigured if we hit this message.
                            // Open Security Groups should map 1:1 with the ports in this match statement.
                            error!("Actor connected to an unexpected port: {}", listener_addr.port());
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
        .with_writer(non_blocking.clone())
        .with_ansi(false)
        .init();
    info!("Tracing initialized");

    // Generate instance name for this Rustbucket
    let instance_name = generate_instance_name();
    info!("Instance name: {}", instance_name);

    // Initialize S3 logger
    match S3Logger::new(instance_name.clone()).await {
        Ok(s3_logger) => {
            if s3_logger.is_enabled() {
                info!("S3 logging is enabled");
                s3_logger.start_background_uploader().await;
            } else {
                info!("S3 logging is disabled");
            }
        }
        Err(e) => {
            error!("Failed to initialize S3 logger: {}. Continuing without S3 logging.", e);
        }
    }

    // Register this instance (optional).
    // Identity is persisted in .rustbucket_identity so the same name/token
    // is used across restarts.
    registration::register_instance().await;

    // Create tasks for each listener on different ports
    let ports = vec!["0.0.0.0:22", "0.0.0.0:25", "0.0.0.0:21", "0.0.0.0:80"];

    let mut handles = vec![];

    for port in ports {
        let handle = tokio::spawn(async move {
            if let Err(e) = start_listener(port).await {
                error!("Listener for {} failed: {}", port, e);
            }
        });
        handles.push(handle);
    }

    info!("All listeners started. Honeypot is now running indefinitely.");

    // Wait for all listener tasks (they run forever, so this keeps the program alive)
    for handle in handles {
        let _ = handle.await;
    }

    // This point should never be reached unless all listeners fail
    error!("All listeners have stopped. This should not happen.");
    Ok(())
}

/// Generate a unique instance name for this Rustbucket
fn generate_instance_name() -> String {
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("rustbucket-{}", random_suffix)
}