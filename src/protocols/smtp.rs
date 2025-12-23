use super::{ProtocolHandler, SessionState, LlmEscalationConfig};
use crate::handler::ChatService;
use crate::prelude::*;
use crate::rate_limiter::RateLimiterRef;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::collections::HashSet;

#[cfg(test)]
#[path = "smtp_tests.rs"]
mod smtp_tests;

/// SMTP Protocol Handler
pub struct SmtpHandler<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_commands: HashSet<String>,
    pub(crate) mail_from: Option<String>,
    pub(crate) rcpt_to: Vec<String>,
    pub(crate) in_data_mode: bool,
    rate_limiter: RateLimiterRef,
}

impl<C: ChatService> SmtpHandler<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig, rate_limiter: RateLimiterRef) -> Self {
        let mut known_commands = HashSet::new();
        known_commands.insert("HELO".to_string());
        known_commands.insert("EHLO".to_string());
        known_commands.insert("MAIL FROM".to_string());
        known_commands.insert("RCPT TO".to_string());
        known_commands.insert("DATA".to_string());
        known_commands.insert("RSET".to_string());
        known_commands.insert("NOOP".to_string());
        known_commands.insert("QUIT".to_string());
        known_commands.insert("VRFY".to_string());
        known_commands.insert("EXPN".to_string());
        known_commands.insert("HELP".to_string());

        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_commands,
            mail_from: None,
            rcpt_to: Vec::new(),
            in_data_mode: false,
            rate_limiter,
        }
    }

    pub(crate) fn is_known_command(&self, cmd: &str) -> bool {
        let cmd_upper = cmd.trim().to_uppercase();

        for known_cmd in &self.known_commands {
            if cmd_upper.starts_with(known_cmd) {
                return true;
            }
        }

        false
    }

    pub(crate) fn get_native_response(&mut self, command: &str) -> Option<String> {
        let cmd_upper = command.trim().to_uppercase();

        if cmd_upper.starts_with("HELO") {
            Some("250 mail.example.com Hello, pleased to meet you\r\n".to_string())
        } else if cmd_upper.starts_with("EHLO") {
            Some("250-mail.example.com Hello\r\n250-PIPELINING\r\n250-SIZE 10240000\r\n250-VRFY\r\n250-ETRN\r\n250-STARTTLS\r\n250-AUTH PLAIN LOGIN\r\n250 8BITMIME\r\n".to_string())
        } else if cmd_upper.starts_with("MAIL FROM:") {
            let from_addr = cmd_upper
                .strip_prefix("MAIL FROM:")
                .unwrap_or("")
                .trim()
                .to_string();
            self.mail_from = Some(from_addr.clone());
            info!("SMTP MAIL FROM: {}", from_addr);
            Some("250 OK\r\n".to_string())
        } else if cmd_upper.starts_with("RCPT TO:") {
            let to_addr = cmd_upper
                .strip_prefix("RCPT TO:")
                .unwrap_or("")
                .trim()
                .to_string();
            self.rcpt_to.push(to_addr.clone());
            info!("SMTP RCPT TO: {}", to_addr);
            Some("250 OK\r\n".to_string())
        } else if cmd_upper.starts_with("DATA") {
            self.in_data_mode = true;
            Some("354 Start mail input; end with <CRLF>.<CRLF>\r\n".to_string())
        } else if cmd_upper.starts_with("RSET") {
            self.mail_from = None;
            self.rcpt_to.clear();
            self.in_data_mode = false;
            Some("250 OK\r\n".to_string())
        } else if cmd_upper.starts_with("NOOP") {
            Some("250 OK\r\n".to_string())
        } else if cmd_upper.starts_with("QUIT") {
            Some("221 Bye\r\n".to_string())
        } else if cmd_upper.starts_with("VRFY") {
            Some("252 Cannot VRFY user, but will accept message\r\n".to_string())
        } else if cmd_upper.starts_with("EXPN") {
            Some("502 Command not implemented\r\n".to_string())
        } else if cmd_upper.starts_with("HELP") {
            Some("214 For more info use HELP <topic>\r\n".to_string())
        } else {
            None
        }
    }
}

#[async_trait::async_trait]
impl<C: ChatService + Send + Sync> ProtocolHandler for SmtpHandler<C> {
    async fn handle_connection<S>(&mut self, mut stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        info!("SMTP handler started");

        // Send SMTP greeting banner
        let banner = "220 mail.example.com ESMTP Postfix (Ubuntu)\r\n";
        if let Err(e) = stream.write_all(banner.as_bytes()).await {
            error!("Failed to send SMTP banner: {}", e);
            return;
        }

        // Main command loop
        let mut buffer = [0u8; 4096];
        let mut email_data = String::new();

        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    info!("SMTP connection closed");
                    break;
                }
                Ok(n) => {
                    let command = String::from_utf8_lossy(&buffer[0..n]).trim().to_string();

                    if command.is_empty() {
                        continue;
                    }

                    info!("SMTP command received: {}", command);

                    self.session_state.commands_processed += 1;
                    self.session_state.last_command_time = Some(std::time::Instant::now());

                    // Handle DATA mode
                    if self.in_data_mode {
                        if command == "." || command == ".\r" {
                            // End of email data
                            info!("SMTP email data received:\n{}", email_data);
                            email_data.clear();
                            self.in_data_mode = false;
                            self.mail_from = None;
                            self.rcpt_to.clear();

                            let response = "250 OK: queued as 12345\r\n";
                            if let Err(e) = stream.write_all(response.as_bytes()).await {
                                error!("Failed to send SMTP response: {}", e);
                                break;
                            }
                            continue;
                        } else {
                            // Accumulate email data
                            email_data.push_str(&command);
                            email_data.push('\n');
                            continue;
                        }
                    }

                    // Check for QUIT command
                    if command.to_uppercase().starts_with("QUIT") {
                        let _ = stream.write_all(b"221 Bye\r\n").await;
                        break;
                    }

                    // Determine if we should use LLM or native response
                    let is_known = self.is_known_command(&command);
                    let use_llm = self.llm_config.should_use_llm(
                        &command,
                        is_known,
                        &self.session_state,
                    );

                    let response = if use_llm {
                        info!("SMTP: Escalating to LLM for command: {}", command);
                        self.session_state.llm_calls_made += 1;
                        match self.chat_service.send_message(&command).await {
                            Ok(resp) => {
                                // Format as SMTP response
                                format!("250 {}\r\n", resp.trim())
                            }
                            Err(e) => {
                                error!("LLM error: {}", e);
                                "500 Command unrecognized\r\n".to_string()
                            }
                        }
                    } else if let Some(native_resp) = self.get_native_response(&command) {
                        info!("SMTP: Using native response for command: {}", command);
                        native_resp
                    } else {
                        info!("SMTP: Unknown command, incrementing counter: {}", command);
                        self.session_state.unknown_commands_count += 1;
                        "500 Command unrecognized\r\n".to_string()
                    };

                    // Apply response delay for realism
                    self.rate_limiter.apply_response_delay().await;

                    // Send response
                    if let Err(e) = stream.write_all(response.as_bytes()).await {
                        error!("Failed to send SMTP response: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("SMTP read error: {}", e);
                    break;
                }
            }
        }

        info!(
            "SMTP session ended. Commands: {}, LLM calls: {}, Duration: {:?}",
            self.session_state.commands_processed,
            self.session_state.llm_calls_made,
            self.session_state.session_duration()
        );
    }
}
