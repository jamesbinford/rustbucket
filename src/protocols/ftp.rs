use super::{ProtocolHandler, SessionState, LlmEscalationConfig};
use crate::handler::ChatService;
use crate::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::collections::HashSet;

#[cfg(test)]
#[path = "ftp_tests.rs"]
mod ftp_tests;

/// FTP Protocol Handler
pub struct FtpHandler<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_commands: HashSet<String>,
    pub(crate) current_dir: String,
    pub(crate) username: Option<String>,
    pub(crate) authenticated: bool,
}

impl<C: ChatService> FtpHandler<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig) -> Self {
        let mut known_commands = HashSet::new();
        known_commands.insert("USER".to_string());
        known_commands.insert("PASS".to_string());
        known_commands.insert("SYST".to_string());
        known_commands.insert("PWD".to_string());
        known_commands.insert("CWD".to_string());
        known_commands.insert("CDUP".to_string());
        known_commands.insert("LIST".to_string());
        known_commands.insert("NLST".to_string());
        known_commands.insert("RETR".to_string());
        known_commands.insert("STOR".to_string());
        known_commands.insert("DELE".to_string());
        known_commands.insert("MKD".to_string());
        known_commands.insert("RMD".to_string());
        known_commands.insert("QUIT".to_string());
        known_commands.insert("TYPE".to_string());
        known_commands.insert("PASV".to_string());
        known_commands.insert("PORT".to_string());

        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_commands,
            current_dir: "/".to_string(),
            username: None,
            authenticated: false,
        }
    }

    pub(crate) fn is_known_command(&self, cmd: &str) -> bool {
        let cmd_upper = cmd.trim().to_uppercase();
        let cmd_parts: Vec<&str> = cmd_upper.split_whitespace().collect();

        if cmd_parts.is_empty() {
            return false;
        }

        self.known_commands.contains(cmd_parts[0])
    }

    pub(crate) fn get_native_response(&mut self, command: &str) -> Option<String> {
        let cmd_upper = command.trim().to_uppercase();
        let parts: Vec<&str> = cmd_upper.split_whitespace().collect();

        if parts.is_empty() {
            return Some("500 Unknown command\r\n".to_string());
        }

        match parts[0] {
            "USER" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                self.username = Some(parts[1].to_string());
                info!("FTP USER: {}", parts[1]);
                Some("331 Please specify the password\r\n".to_string())
            }
            "PASS" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                let password = parts[1..].join(" ");
                info!("FTP PASS: {} (username: {:?})", password, self.username);
                self.authenticated = true;
                Some("230 Login successful\r\n".to_string())
            }
            "SYST" => {
                Some("215 UNIX Type: L8\r\n".to_string())
            }
            "PWD" => {
                Some(format!("257 \"{}\" is the current directory\r\n", self.current_dir))
            }
            "CWD" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                let path = parts[1];
                if path == ".." {
                    self.current_dir = "/".to_string();
                } else if path.starts_with('/') {
                    self.current_dir = path.to_string();
                } else {
                    if self.current_dir == "/" {
                        self.current_dir = format!("/{}", path);
                    } else {
                        self.current_dir = format!("{}/{}", self.current_dir, path);
                    }
                }
                Some(format!("250 Directory successfully changed to {}\r\n", self.current_dir))
            }
            "CDUP" => {
                self.current_dir = "/".to_string();
                Some("250 Directory successfully changed\r\n".to_string())
            }
            "LIST" | "NLST" => {
                // Return fake directory listing
                Some("150 Here comes the directory listing\r\n-rw-r--r--    1 ftp      ftp          1234 Dec 12 10:30 file1.txt\n-rw-r--r--    1 ftp      ftp          5678 Dec 12 10:31 file2.txt\ndrwxr-xr-x    2 ftp      ftp          4096 Dec 12 10:29 documents\n226 Directory send OK\r\n".to_string())
            }
            "RETR" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some(format!("550 Failed to open file {}\r\n", parts[1]))
            }
            "STOR" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                info!("FTP STOR attempt: {}", parts[1]);
                Some("553 Could not create file\r\n".to_string())
            }
            "DELE" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some("250 Delete operation successful\r\n".to_string())
            }
            "MKD" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some(format!("257 \"{}\" created\r\n", parts[1]))
            }
            "RMD" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some("250 Remove directory operation successful\r\n".to_string())
            }
            "TYPE" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some(format!("200 Switching to {} mode\r\n", parts[1]))
            }
            "PASV" => {
                // Return fake PASV response (won't actually work but looks realistic)
                Some("227 Entering Passive Mode (192,168,1,100,195,149)\r\n".to_string())
            }
            "PORT" => {
                if parts.len() < 2 {
                    return Some("501 Syntax error in parameters or arguments\r\n".to_string());
                }
                Some("200 PORT command successful\r\n".to_string())
            }
            "QUIT" => {
                Some("221 Goodbye\r\n".to_string())
            }
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl<C: ChatService + Send + Sync> ProtocolHandler for FtpHandler<C> {
    async fn handle_connection<S>(&mut self, mut stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        info!("FTP handler started");

        // Send FTP welcome banner
        let banner = "220 (vsFTPd 3.0.3)\r\n";
        if let Err(e) = stream.write_all(banner.as_bytes()).await {
            error!("Failed to send FTP banner: {}", e);
            return;
        }

        // Main command loop
        let mut buffer = [0u8; 1024];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    info!("FTP connection closed");
                    break;
                }
                Ok(n) => {
                    let command = String::from_utf8_lossy(&buffer[0..n]).trim().to_string();

                    if command.is_empty() {
                        continue;
                    }

                    info!("FTP command received: {}", command);

                    self.session_state.commands_processed += 1;
                    self.session_state.last_command_time = Some(std::time::Instant::now());

                    // Check for QUIT command
                    if command.to_uppercase().starts_with("QUIT") {
                        let _ = stream.write_all(b"221 Goodbye\r\n").await;
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
                        info!("FTP: Escalating to LLM for command: {}", command);
                        self.session_state.llm_calls_made += 1;
                        match self.chat_service.send_message(&command).await {
                            Ok(resp) => {
                                // Format as FTP response
                                format!("200 {}\r\n", resp.trim())
                            }
                            Err(e) => {
                                error!("LLM error: {}", e);
                                "500 Unknown command\r\n".to_string()
                            }
                        }
                    } else if let Some(native_resp) = self.get_native_response(&command) {
                        info!("FTP: Using native response for command: {}", command);
                        native_resp
                    } else {
                        info!("FTP: Unknown command, incrementing counter: {}", command);
                        self.session_state.unknown_commands_count += 1;
                        "500 Unknown command\r\n".to_string()
                    };

                    // Send response
                    if let Err(e) = stream.write_all(response.as_bytes()).await {
                        error!("Failed to send FTP response: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("FTP read error: {}", e);
                    break;
                }
            }
        }

        info!(
            "FTP session ended. Commands: {}, LLM calls: {}, Duration: {:?}",
            self.session_state.commands_processed,
            self.session_state.llm_calls_made,
            self.session_state.session_duration()
        );
    }
}
