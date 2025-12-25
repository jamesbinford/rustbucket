use super::ssh_shell::SshShellSimulator;
use super::LlmEscalationConfig;
use crate::llm::ChatService;
use crate::config::TarpitConfig;
use crate::fingerprint::ServerFingerprint;
use crate::rate_limiter::RateLimiterRef;
use crate::tarpit::Tarpit;
use russh::server::{Auth, Msg, Server as SshServer, Session};
use russh::{Channel, ChannelId, CryptoVec};
use russh::keys::{HashAlg, PublicKey};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error};

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod ssh_tests;

/// SSH Honeypot Server - creates handlers for each connection
pub struct SshHoneypotServer<C: ChatService + Clone + Send + Sync + 'static> {
    chat_service: C,
    llm_config: LlmEscalationConfig,
    rate_limiter: RateLimiterRef,
    tarpit_config: TarpitConfig,
    fingerprint: ServerFingerprint,
}

impl<C: ChatService + Clone + Send + Sync + 'static> SshHoneypotServer<C> {
    pub fn new(
        chat_service: C,
        llm_config: LlmEscalationConfig,
        rate_limiter: RateLimiterRef,
        tarpit_config: TarpitConfig,
        fingerprint: ServerFingerprint,
    ) -> Self {
        Self {
            chat_service,
            llm_config,
            rate_limiter,
            tarpit_config,
            fingerprint,
        }
    }
}

impl<C: ChatService + Clone + Send + Sync + 'static> SshServer for SshHoneypotServer<C> {
    type Handler = SshHoneypotHandler<C>;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        let addr = peer_addr.map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string());
        let ip = peer_addr.map(|a| a.ip());
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %addr,
            "New SSH connection"
        );

        SshHoneypotHandler::new(
            self.chat_service.clone(),
            self.llm_config.clone(),
            addr,
            ip,
            self.rate_limiter.clone(),
            self.tarpit_config.clone(),
            self.fingerprint.clone(),
        )
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        error!(
            event_type = "operational",
            protocol = "SSH",
            error = ?error,
            "SSH session error"
        );
    }
}

/// SSH Honeypot Handler - handles a single SSH connection
pub struct SshHoneypotHandler<C: ChatService + Send + Sync + 'static> {
    shell_simulator: Arc<Mutex<SshShellSimulator<C>>>,
    client_addr: String,
    client_ip: Option<IpAddr>,
    input_buffer: String,
    rate_limiter: RateLimiterRef,
    rate_limit_checked: bool,
    tarpit: Tarpit,
    fingerprint: ServerFingerprint,
}

impl<C: ChatService + Send + Sync + 'static> SshHoneypotHandler<C> {
    pub fn new(
        chat_service: C,
        llm_config: LlmEscalationConfig,
        client_addr: String,
        client_ip: Option<IpAddr>,
        rate_limiter: RateLimiterRef,
        tarpit_config: TarpitConfig,
        fingerprint: ServerFingerprint,
    ) -> Self {
        Self {
            shell_simulator: Arc::new(Mutex::new(SshShellSimulator::new(chat_service, llm_config, fingerprint.clone()))),
            client_addr,
            client_ip,
            input_buffer: String::new(),
            rate_limiter,
            rate_limit_checked: false,
            tarpit: Tarpit::new(tarpit_config),
            fingerprint,
        }
    }

    /// Check rate limit on first auth attempt (since new_client is sync)
    async fn check_rate_limit(&mut self) -> bool {
        if self.rate_limit_checked {
            return true; // Already checked and passed
        }
        self.rate_limit_checked = true;

        if let Some(ip) = self.client_ip {
            if let Err(reason) = self.rate_limiter.check_connection(ip).await {
                info!(
                    event_type = "operational",
                    protocol = "SSH",
                    client_ip = %self.client_addr,
                    reason = %reason,
                    "Rate limit exceeded"
                );
                return false;
            }
        }
        true
    }
}

impl<C: ChatService + Send + Sync + 'static> russh::server::Handler for SshHoneypotHandler<C> {
    type Error = russh::Error;

    /// Handle password authentication - always accept and log credentials
    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        // Check rate limit on first auth attempt
        if !self.check_rate_limit().await {
            return Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            });
        }

        // Apply tarpit delay (or fallback to rate_limiter delay)
        if self.tarpit.is_enabled() {
            self.tarpit.apply_delay().await;
        } else {
            self.rate_limiter.apply_response_delay().await;
        }

        info!(
            event_type = "auth",
            protocol = "SSH",
            client_ip = %self.client_addr,
            username = %user,
            password = %password,
            auth_method = "password",
            "Authentication attempt"
        );

        // Update the shell simulator with the captured username
        let mut simulator = self.shell_simulator.lock().await;
        simulator.set_username(user.to_string());

        Ok(Auth::Accept)
    }

    /// Handle public key authentication - always accept and log key fingerprint
    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        // Check rate limit on first auth attempt
        if !self.check_rate_limit().await {
            return Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            });
        }

        // Apply tarpit delay (or fallback to rate_limiter delay)
        if self.tarpit.is_enabled() {
            self.tarpit.apply_delay().await;
        } else {
            self.rate_limiter.apply_response_delay().await;
        }

        let fingerprint = public_key.fingerprint(HashAlg::Sha256);
        info!(
            event_type = "auth",
            protocol = "SSH",
            client_ip = %self.client_addr,
            username = %user,
            auth_method = "publickey",
            key_type = %public_key.algorithm(),
            fingerprint = %fingerprint,
            "Authentication attempt"
        );

        // Update the shell simulator with the captured username
        let mut simulator = self.shell_simulator.lock().await;
        simulator.set_username(user.to_string());

        Ok(Auth::Accept)
    }

    /// Handle channel open requests (shell sessions)
    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %self.client_addr,
            "Channel opened"
        );
        Ok(true)
    }

    /// Handle shell requests - send welcome message and prompt
    async fn shell_request(
        &mut self,
        channel_id: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %self.client_addr,
            "Shell request"
        );

        // Send welcome banner using fingerprint
        session.data(channel_id, CryptoVec::from_slice(self.fingerprint.ssh_banner.as_bytes()))?;

        // Send initial prompt
        let simulator = self.shell_simulator.lock().await;
        let prompt = simulator.get_prompt();
        session.data(channel_id, CryptoVec::from_slice(prompt.as_bytes()))?;

        Ok(())
    }

    /// Handle exec requests (single command execution)
    async fn exec_request(
        &mut self,
        channel_id: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let command = String::from_utf8_lossy(data).to_string();
        info!(
            event_type = "command",
            protocol = "SSH",
            client_ip = %self.client_addr,
            command = %command,
            "Exec request"
        );

        // Process the command
        let mut simulator = self.shell_simulator.lock().await;
        let response = simulator.process_command(&command).await;

        // Send response
        session.data(channel_id, CryptoVec::from_slice(response.as_bytes()))?;

        // Close the channel after exec
        session.close(channel_id)?;

        Ok(())
    }

    /// Handle PTY requests (terminal allocation)
    async fn pty_request(
        &mut self,
        channel_id: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %self.client_addr,
            terminal = %term,
            width = col_width,
            height = row_height,
            "PTY request"
        );
        session.channel_success(channel_id)?;
        Ok(())
    }

    /// Handle environment variable requests - log and accept for realism
    async fn env_request(
        &mut self,
        channel_id: ChannelId,
        variable_name: &str,
        variable_value: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "command",
            protocol = "SSH",
            client_ip = %self.client_addr,
            env_name = %variable_name,
            env_value = %variable_value,
            "Environment variable set"
        );
        session.channel_success(channel_id)?;
        Ok(())
    }

    /// Handle window size change requests
    async fn window_change_request(
        &mut self,
        channel_id: ChannelId,
        col_width: u32,
        row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %self.client_addr,
            width = col_width,
            height = row_height,
            "Window size change"
        );
        session.channel_success(channel_id)?;
        Ok(())
    }

    /// Handle subsystem requests (e.g., SFTP) - reject as not configured
    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "command",
            protocol = "SSH",
            client_ip = %self.client_addr,
            subsystem = %name,
            "Subsystem request"
        );
        // Reject - realistic for a server without SFTP/other subsystems configured
        session.channel_failure(channel_id)?;
        Ok(())
    }

    /// Handle incoming data from the client
    async fn data(
        &mut self,
        channel_id: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Convert data to string, handling partial UTF-8
        let input = String::from_utf8_lossy(data);

        for ch in input.chars() {
            match ch {
                // Enter key - process command
                '\r' | '\n' => {
                    // Echo newline
                    session.data(channel_id, CryptoVec::from_slice(b"\r\n"))?;

                    if self.input_buffer.is_empty() {
                        // Empty command - just send prompt
                        let simulator = self.shell_simulator.lock().await;
                        let prompt = simulator.get_prompt();
                        session.data(channel_id, CryptoVec::from_slice(prompt.as_bytes()))?;
                    } else {
                        let command = std::mem::take(&mut self.input_buffer);
                        let mut simulator = self.shell_simulator.lock().await;

                        // Check for exit command
                        if simulator.is_exit_command(&command) {
                            session.data(channel_id, CryptoVec::from_slice(b"logout\r\n"))?;
                            session.close(channel_id)?;
                            return Ok(());
                        }

                        // Process command and send response
                        let response = simulator.process_command(&command).await;
                        let response = response.replace('\n', "\r\n");
                        session.data(channel_id, CryptoVec::from_slice(response.as_bytes()))?;

                        // Send prompt
                        let prompt = simulator.get_prompt();
                        session.data(channel_id, CryptoVec::from_slice(prompt.as_bytes()))?;
                    }
                }
                // Backspace
                '\x7f' | '\x08' => {
                    if !self.input_buffer.is_empty() {
                        self.input_buffer.pop();
                        // Echo backspace: move back, space, move back
                        session.data(channel_id, CryptoVec::from_slice(b"\x08 \x08"))?;
                    }
                }
                // Ctrl+C - send ^C and reset buffer
                '\x03' => {
                    self.input_buffer.clear();
                    session.data(channel_id, CryptoVec::from_slice(b"^C\r\n"))?;
                    let simulator = self.shell_simulator.lock().await;
                    let prompt = simulator.get_prompt();
                    session.data(channel_id, CryptoVec::from_slice(prompt.as_bytes()))?;
                }
                // Ctrl+D - EOF/logout
                '\x04' => {
                    if self.input_buffer.is_empty() {
                        session.data(channel_id, CryptoVec::from_slice(b"logout\r\n"))?;
                        session.close(channel_id)?;
                        return Ok(());
                    }
                }
                // Regular character - add to buffer and echo
                _ if ch.is_ascii() && !ch.is_control() => {
                    self.input_buffer.push(ch);
                    session.data(channel_id, CryptoVec::from_slice(&[ch as u8]))?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle channel close
    async fn channel_close(
        &mut self,
        _channel_id: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!(
            event_type = "connection",
            protocol = "SSH",
            client_ip = %self.client_addr,
            "Channel closed"
        );

        // Release rate limit connection tracking
        if let Some(ip) = self.client_ip {
            self.rate_limiter.release_connection(ip).await;
        }

        // Log session statistics
        let simulator = self.shell_simulator.lock().await;
        let (commands, llm_calls, duration) = simulator.get_session_stats();
        info!(
            event_type = "session",
            protocol = "SSH",
            client_ip = %self.client_addr,
            commands_processed = commands,
            llm_calls_made = llm_calls,
            duration_secs = duration.as_secs(),
            "Session ended"
        );

        Ok(())
    }
}
