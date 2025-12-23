use super::{ProtocolHandler, SessionState, LlmEscalationConfig, CommandLoopHandler, CommandResult, run_command_loop};
use crate::chatgpt::ChatService;
use crate::rate_limiter::RateLimiterRef;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::{info, error};
use std::collections::HashSet;

#[cfg(test)]
#[path = "smtp_tests.rs"]
mod smtp_tests;

const KNOWN_SMTP_COMMANDS: &[&str] = &[
    "HELO", "EHLO", "MAIL FROM", "RCPT TO", "DATA",
    "RSET", "NOOP", "QUIT", "VRFY", "EXPN", "HELP",
];

/// SMTP Protocol Handler
pub struct SmtpHandler<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_commands: HashSet<String>,
    pub(crate) mail_from: Option<String>,
    pub(crate) rcpt_to: Vec<String>,
    pub(crate) in_data_mode: bool,
    pub(crate) email_data: String,
    rate_limiter: RateLimiterRef,
}

impl<C: ChatService> SmtpHandler<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig, rate_limiter: RateLimiterRef) -> Self {
        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_commands: KNOWN_SMTP_COMMANDS.iter().map(|s| s.to_string()).collect(),
            mail_from: None,
            rcpt_to: Vec::new(),
            in_data_mode: false,
            email_data: String::new(),
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

impl<C: ChatService> CommandLoopHandler for SmtpHandler<C> {
    fn session_state(&self) -> &SessionState {
        &self.session_state
    }

    fn session_state_mut(&mut self) -> &mut SessionState {
        &mut self.session_state
    }

    fn llm_config(&self) -> &LlmEscalationConfig {
        &self.llm_config
    }

    fn is_known_command(&self, cmd: &str) -> bool {
        // Delegate to inherent method
        SmtpHandler::is_known_command(self, cmd)
    }

    fn get_native_response(&mut self, command: &str) -> Option<String> {
        // Delegate to inherent method
        SmtpHandler::get_native_response(self, command)
    }

    fn default_error_response(&self) -> &'static str {
        "500 Command unrecognized\r\n"
    }

    fn format_llm_response(&self, response: &str) -> String {
        format!("250 {}\r\n", response.trim())
    }

    fn protocol_name(&self) -> &'static str {
        "SMTP"
    }

    fn pre_process_command(&mut self, command: &str) -> Option<(CommandResult, Option<String>)> {
        // Handle DATA mode
        if self.in_data_mode {
            if command == "." || command == ".\r" {
                // End of email data
                info!("SMTP email data received:\n{}", self.email_data);
                self.email_data.clear();
                self.in_data_mode = false;
                self.mail_from = None;
                self.rcpt_to.clear();
                return Some((CommandResult::Continue, Some("250 OK: queued as 12345\r\n".to_string())));
            } else {
                // Accumulate email data
                self.email_data.push_str(command);
                self.email_data.push('\n');
                return Some((CommandResult::SkipResponse, None));
            }
        }

        // Check for QUIT command
        if command.to_uppercase().starts_with("QUIT") {
            return Some((CommandResult::Exit, Some("221 Bye\r\n".to_string())));
        }

        None
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

        // Use shared command loop (clone to avoid borrow conflicts)
        let chat_service = self.chat_service.clone();
        let rate_limiter = self.rate_limiter.clone();
        run_command_loop(self, &chat_service, &rate_limiter, &mut stream, 4096).await;
    }
}
