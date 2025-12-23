mod handler;
mod prelude;
mod chatgpt;
mod config;
mod registration;
mod protocols;
mod s3_logger;
mod rate_limiter;

use crate::prelude::*;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling;
use chatgpt::ChatGPT;
use config::AppConfig;
use protocols::{ProtocolHandler, LlmEscalationConfig};
use protocols::ssh::SshHoneypotServer;
use protocols::http::HttpHandler;
use protocols::ftp::FtpHandler;
use protocols::smtp::SmtpHandler;
use s3_logger::S3Logger;
use rate_limiter::{RateLimiter, RateLimiterRef};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rand::rngs::OsRng;
use std::sync::Arc;
use russh::server::Server as _;

/// Start the SSH honeypot server using russh (real SSH protocol)
async fn start_ssh_server(chatgpt: ChatGPT, llm_config: LlmEscalationConfig, rate_limiter: RateLimiterRef) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Generate SSH host key (Ed25519)
    let key = russh::keys::PrivateKey::random(&mut OsRng, russh::keys::Algorithm::Ed25519)
        .map_err(|e| format!("Failed to generate SSH host key: {}", e))?;
    info!("Generated SSH host key (Ed25519)");

    // Configure the SSH server
    let config = russh::server::Config {
        keys: vec![key],
        auth_rejection_time: std::time::Duration::from_secs(1),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let config = Arc::new(config);
    let mut server = SshHoneypotServer::new(chatgpt, llm_config, rate_limiter);

    let ssh_port = std::env::var("SSH_PORT").unwrap_or_else(|_| "22".to_string()).parse().unwrap_or(22);
    info!("Starting SSH honeypot on 0.0.0.0:{}", ssh_port);
    server.run_on_address(config, ("0.0.0.0", ssh_port)).await?;

    Ok(())
}

/// Protocol types for the honeypot
#[derive(Clone, Copy, Debug)]
enum Protocol {
    Smtp,
    Ftp,
    Http,
}

/// Start a TCP listener for non-SSH protocols (HTTP, FTP, SMTP)
async fn start_listener(
    addr: &str,
    protocol: Protocol,
    rate_limiter: RateLimiterRef,
    llm_config: config::LlmConfig,
) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let listener_addr = listener.local_addr()?;
    println!("Listening on {} ({:?})", listener_addr, protocol);

    // Instantiate ChatGPT with config
    let chatgpt = ChatGPT::new(&llm_config).unwrap();
    let llm_escalation = LlmEscalationConfig::default();

    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                let ip = client_addr.ip();

                // Check rate limit before accepting connection
                if let Err(reason) = rate_limiter.check_connection(ip).await {
                    info!("Connection rejected from {}: {}", client_addr, reason);
                    drop(stream);
                    continue;
                }

                println!("New connection on {} from {}", listener_addr, client_addr);
                let chatgpt = chatgpt.clone();
                let llm_escalation = llm_escalation.clone();
                let rate_limiter = rate_limiter.clone();

                task::spawn(async move {
                    // Apply initial response delay to simulate real server
                    rate_limiter.apply_response_delay().await;

                    match protocol {
                        Protocol::Smtp => {
                            info!("Actor attempted to connect - SMTP");
                            let mut handler = SmtpHandler::new(chatgpt, llm_escalation, rate_limiter.clone());
                            handler.handle_connection(stream).await;
                        }
                        Protocol::Http => {
                            info!("Actor attempted to connect - HTTP");
                            let mut handler = HttpHandler::new(chatgpt, llm_escalation, rate_limiter.clone());
                            handler.handle_connection(stream).await;
                        }
                        Protocol::Ftp => {
                            info!("Actor attempted to connect - FTP");
                            let mut handler = FtpHandler::new(chatgpt, llm_escalation, rate_limiter.clone());
                            handler.handle_connection(stream).await;
                        }
                    }

                    // Release connection when handler completes
                    rate_limiter.release_connection(ip).await;
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
    // Load configuration once at startup
    let app_config = AppConfig::load();

    // Set up rolling logs
    let file_appender = rolling::daily(&app_config.general.log_directory, "rustbucket.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize tracing subscriber
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new(&app_config.general.log_level))
        .with_writer(non_blocking.clone())
        .with_ansi(false)
        .init();
    info!("Tracing initialized");
    info!("Configuration loaded from Config.toml");

    // Generate instance name for this Rustbucket
    let instance_name = generate_instance_name();
    info!("Instance name: {}", instance_name);

    // Initialize S3 logger
    match S3Logger::new(
        instance_name.clone(),
        app_config.s3_logging.clone(),
        &app_config.general,
    ).await {
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

    // Register this instance (optional)
    registration::register_instance(&app_config.registration).await;

    // Initialize shared rate limiter
    let rate_limiter: RateLimiterRef = Arc::new(RateLimiter::new(app_config.rate_limiting.clone()));

    // Get LLM config or use default
    let llm_config = app_config.llm.clone().unwrap_or_default();

    // Start SSH server (uses russh for real SSH protocol)
    let chatgpt_for_ssh = ChatGPT::new(&llm_config).unwrap();
    let llm_escalation_for_ssh = LlmEscalationConfig::default();
    let rate_limiter_for_ssh = rate_limiter.clone();

    let ssh_handle = tokio::spawn(async move {
        if let Err(e) = start_ssh_server(chatgpt_for_ssh, llm_escalation_for_ssh, rate_limiter_for_ssh).await {
            error!("SSH server failed: {}", e);
        }
    });

    // Start other protocol listeners (SMTP, HTTP, FTP) with configurable ports
    let smtp_port = std::env::var("SMTP_PORT").unwrap_or_else(|_| "25".to_string());
    let ftp_port = std::env::var("FTP_PORT").unwrap_or_else(|_| "21".to_string());
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "80".to_string());

    let listeners = vec![
        (format!("0.0.0.0:{}", smtp_port), Protocol::Smtp),
        (format!("0.0.0.0:{}", ftp_port), Protocol::Ftp),
        (format!("0.0.0.0:{}", http_port), Protocol::Http),
    ];
    let mut handles = vec![ssh_handle];

    for (addr, protocol) in listeners {
        let rate_limiter = rate_limiter.clone();
        let llm_config = llm_config.clone();
        let addr_clone = addr.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = start_listener(&addr, protocol, rate_limiter, llm_config).await {
                error!("Listener for {} failed: {}", addr_clone, e);
            }
        });
        handles.push(handle);
    }

    info!("All listeners started. Honeypot is now running indefinitely.");

    // Wait for all listener tasks
    for handle in handles {
        let _ = handle.await;
    }

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
