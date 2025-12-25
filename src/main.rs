mod chatgpt;
mod config;
mod registration;
mod protocols;
mod s3_logger;
mod rate_limiter;
mod tarpit;

use tokio::net::TcpListener;
use tokio::task;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use tracing_appender::rolling;
use chatgpt::ChatGPT;
use config::{AppConfig, TarpitConfig};
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
async fn start_ssh_server(chatgpt: ChatGPT, llm_config: LlmEscalationConfig, rate_limiter: RateLimiterRef, tarpit_config: TarpitConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Generate SSH host key (Ed25519)
    let key = russh::keys::PrivateKey::random(&mut OsRng, russh::keys::Algorithm::Ed25519)
        .map_err(|e| format!("Failed to generate SSH host key: {}", e))?;
    info!(
        event_type = "operational",
        protocol = "SSH",
        key_type = "Ed25519",
        "Generated host key"
    );

    // Configure the SSH server
    let config = russh::server::Config {
        keys: vec![key],
        auth_rejection_time: std::time::Duration::from_secs(1),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let config = Arc::new(config);
    let mut server = SshHoneypotServer::new(chatgpt, llm_config, rate_limiter, tarpit_config);

    let ssh_port = std::env::var("SSH_PORT").unwrap_or_else(|_| "22".to_string()).parse().unwrap_or(22);
    info!(
        event_type = "operational",
        protocol = "SSH",
        port = ssh_port,
        "Starting SSH honeypot"
    );
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
    tarpit_config: TarpitConfig,
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
                    info!(
                        event_type = "connection",
                        client_ip = %client_addr,
                        reason = %reason,
                        "Connection rejected"
                    );
                    drop(stream);
                    continue;
                }

                println!("New connection on {} from {}", listener_addr, client_addr);
                let chatgpt = chatgpt.clone();
                let llm_escalation = llm_escalation.clone();
                let rate_limiter = rate_limiter.clone();
                let tarpit_config = tarpit_config.clone();

                task::spawn(async move {
                    // Apply initial response delay to simulate real server
                    rate_limiter.apply_response_delay().await;

                    match protocol {
                        Protocol::Smtp => {
                            info!(
                                event_type = "connection",
                                protocol = "SMTP",
                                client_ip = %ip,
                                "New connection"
                            );
                            let mut handler = SmtpHandler::new(chatgpt, llm_escalation, rate_limiter.clone(), tarpit_config);
                            handler.handle_connection(stream).await;
                        }
                        Protocol::Http => {
                            info!(
                                event_type = "connection",
                                protocol = "HTTP",
                                client_ip = %ip,
                                "New connection"
                            );
                            let mut handler = HttpHandler::new(chatgpt, llm_escalation, rate_limiter.clone(), tarpit_config);
                            handler.handle_connection(stream).await;
                        }
                        Protocol::Ftp => {
                            info!(
                                event_type = "connection",
                                protocol = "FTP",
                                client_ip = %ip,
                                "New connection"
                            );
                            let mut handler = FtpHandler::new(chatgpt, llm_escalation, rate_limiter.clone(), tarpit_config);
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

    // Set up hourly rolling logs for more frequent S3 uploads
    let file_appender = rolling::hourly(&app_config.general.log_directory, "rustbucket.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize tracing subscriber with JSON output for structured logging
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new(&app_config.general.log_level))
        .with_writer(non_blocking.clone())
        .json()
        .init();
    info!(event_type = "operational", "Tracing initialized");
    info!(event_type = "operational", "Configuration loaded from Config.toml");

    // Generate instance name for this Rustbucket
    let instance_name = generate_instance_name();
    info!(event_type = "operational", instance_name = %instance_name, "Instance started");

    // Register this instance first (to get S3 config from registry)
    let registry_s3_config = registration::register_instance(&app_config.registration).await;

    // Build S3 config - registry config takes precedence over local config
    let s3_config = if let Some(ref reg_config) = registry_s3_config {
        info!(
            event_type = "operational",
            component = "s3_logger",
            bucket = %reg_config.bucket,
            region = %reg_config.region,
            "Using S3 config from registry"
        );
        config::S3Config {
            bucket_name: Some(reg_config.bucket.clone()),
            region: Some(reg_config.region.clone()),
            prefix: reg_config.prefix.clone(),
            ..app_config.s3_logging.clone()
        }
    } else {
        app_config.s3_logging.clone()
    };

    // Initialize S3 logger with merged config
    match S3Logger::new(
        instance_name.clone(),
        s3_config,
        &app_config.general,
    ).await {
        Ok(s3_logger) => {
            if s3_logger.is_enabled() {
                info!(event_type = "operational", component = "s3_logger", "S3 logging is enabled");
                s3_logger.start_background_uploader().await;
            } else {
                info!(event_type = "operational", component = "s3_logger", "S3 logging is disabled");
            }
        }
        Err(e) => {
            error!(event_type = "operational", component = "s3_logger", error = %e, "Failed to initialize S3 logger");
        }
    }

    // Initialize shared rate limiter
    let rate_limiter: RateLimiterRef = Arc::new(RateLimiter::new(app_config.rate_limiting.clone()));

    // Get LLM config or use default
    let llm_config = app_config.llm.clone().unwrap_or_default();
    let tarpit_config = app_config.tarpit.clone();

    // Log tarpit status
    if tarpit_config.enabled {
        info!(
            event_type = "operational",
            component = "tarpit",
            base_delay_ms = tarpit_config.base_delay_ms,
            max_delay_ms = tarpit_config.max_delay_ms,
            progressive = tarpit_config.progressive,
            multiplier = tarpit_config.delay_multiplier,
            jitter_percent = tarpit_config.jitter_percent,
            "Tarpit enabled"
        );
    }

    // Start SSH server (uses russh for real SSH protocol)
    let chatgpt_for_ssh = ChatGPT::new(&llm_config).unwrap();
    let llm_escalation_for_ssh = LlmEscalationConfig::default();
    let rate_limiter_for_ssh = rate_limiter.clone();
    let tarpit_config_for_ssh = tarpit_config.clone();

    let ssh_handle = tokio::spawn(async move {
        if let Err(e) = start_ssh_server(chatgpt_for_ssh, llm_escalation_for_ssh, rate_limiter_for_ssh, tarpit_config_for_ssh).await {
            error!(event_type = "operational", protocol = "SSH", error = %e, "SSH server failed");
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
        let tarpit_config = tarpit_config.clone();
        let addr_clone = addr.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = start_listener(&addr, protocol, rate_limiter, llm_config, tarpit_config).await {
                error!(event_type = "operational", address = %addr_clone, error = %e, "Listener failed");
            }
        });
        handles.push(handle);
    }

    info!(event_type = "operational", "All listeners started. Honeypot is now running indefinitely.");

    // Wait for all listener tasks
    for handle in handles {
        let _ = handle.await;
    }

    error!(event_type = "operational", "All listeners have stopped. This should not happen.");
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
