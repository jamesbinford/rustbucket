use super::{ProtocolHandler, SessionState, LlmEscalationConfig};
use crate::handler::ChatService;
use crate::prelude::*;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::collections::HashSet;

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod ssh_tests;

/// SSH Protocol Handler
pub struct SshHandler<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_commands: HashSet<String>,
    pub(crate) hostname: String,
    pub(crate) username: String,
    pub(crate) current_dir: String,
    pub(crate) authenticated: bool,
}

impl<C: ChatService> SshHandler<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig) -> Self {
        let mut known_commands = HashSet::new();
        // Basic commands
        known_commands.insert("ls".to_string());
        known_commands.insert("ls -la".to_string());
        known_commands.insert("ls -l".to_string());
        known_commands.insert("pwd".to_string());
        known_commands.insert("whoami".to_string());
        known_commands.insert("id".to_string());
        known_commands.insert("uname".to_string());
        known_commands.insert("uname -a".to_string());
        known_commands.insert("cat /etc/passwd".to_string());
        known_commands.insert("cat /etc/shadow".to_string());
        known_commands.insert("w".to_string());
        known_commands.insert("ps".to_string());
        known_commands.insert("ps aux".to_string());
        known_commands.insert("netstat".to_string());
        known_commands.insert("netstat -an".to_string());
        known_commands.insert("ifconfig".to_string());
        known_commands.insert("ip addr".to_string());
        known_commands.insert("exit".to_string());
        known_commands.insert("quit".to_string());

        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_commands,
            hostname: "ubuntu-server".to_string(),
            username: "root".to_string(),
            current_dir: "/root".to_string(),
            authenticated: false,
        }
    }

    pub(crate) fn is_known_command(&self, cmd: &str) -> bool {
        let cmd_lower = cmd.trim().to_lowercase();

        // Exact match
        if self.known_commands.contains(&cmd_lower) {
            return true;
        }

        // Prefix matches for cd commands
        if cmd_lower.starts_with("cd ") || cmd_lower == "cd" {
            return true;
        }

        // Prefix matches for cat commands
        if cmd_lower.starts_with("cat ") {
            return true;
        }

        false
    }

    pub(crate) fn get_native_response(&mut self, cmd: &str) -> Option<String> {
        let cmd_lower = cmd.trim().to_lowercase();

        match cmd_lower.as_str() {
            "ls" | "ls -l" | "ls -la" => {
                Some("total 24\ndrwxr-xr-x 2 root root 4096 Dec 12 10:30 .\ndrwxr-xr-x 3 root root 4096 Dec 12 10:29 ..\n-rw-r--r-- 1 root root  220 Dec 12 10:29 .bash_logout\n-rw-r--r-- 1 root root 3526 Dec 12 10:29 .bashrc\n-rw-r--r-- 1 root root  807 Dec 12 10:29 .profile\n".to_string())
            }
            "pwd" => Some(format!("{}\n", self.current_dir)),
            "whoami" => Some(format!("{}\n", self.username)),
            "id" => Some("uid=0(root) gid=0(root) groups=0(root)\n".to_string()),
            "uname" | "uname -a" => {
                Some("Linux ubuntu-server 5.15.0-56-generic #62-Ubuntu SMP x86_64 GNU/Linux\n".to_string())
            }
            "cat /etc/passwd" => {
                Some("root:x:0:0:root:/root:/bin/bash\ndaemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\nbin:x:2:2:bin:/bin:/usr/sbin/nologin\nsys:x:3:3:sys:/dev:/usr/sbin/nologin\nsync:x:4:65534:sync:/bin:/bin/sync\n".to_string())
            }
            "cat /etc/shadow" => {
                Some("cat: /etc/shadow: Permission denied\n".to_string())
            }
            "w" => {
                Some(" 10:30:42 up  1:23,  1 user,  load average: 0.08, 0.12, 0.09\nUSER     TTY      FROM             LOGIN@   IDLE   JCPU   PCPU WHAT\nroot     pts/0    192.168.1.100    10:15    0.00s  0.04s  0.00s w\n".to_string())
            }
            "ps" | "ps aux" => {
                Some("USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND\nroot         1  0.0  0.1  16896  8432 ?        Ss   09:07   0:01 /sbin/init\nroot       123  0.0  0.0   5532  3216 ?        Ss   09:07   0:00 /usr/sbin/sshd\nroot       456  0.0  0.1   8932  4432 pts/0    Ss   10:15   0:00 -bash\n".to_string())
            }
            "netstat" | "netstat -an" => {
                Some("Active Internet connections (servers and established)\nProto Recv-Q Send-Q Local Address           Foreign Address         State\ntcp        0      0 0.0.0.0:22              0.0.0.0:*               LISTEN\ntcp        0      0 0.0.0.0:80              0.0.0.0:*               LISTEN\n".to_string())
            }
            "ifconfig" | "ip addr" => {
                Some("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500\n        inet 192.168.1.100  netmask 255.255.255.0  broadcast 192.168.1.255\n        ether 00:0c:29:3a:2f:4e  txqueuelen 1000  (Ethernet)\n".to_string())
            }
            "exit" | "quit" => {
                Some("logout\n".to_string())
            }
            _ => {
                // Handle cd commands
                if cmd_lower.starts_with("cd ") {
                    let path = cmd_lower.strip_prefix("cd ").unwrap().trim();
                    if path == ".." {
                        self.current_dir = "/".to_string();
                    } else if path.starts_with('/') {
                        self.current_dir = path.to_string();
                    } else {
                        self.current_dir = format!("{}/{}", self.current_dir, path);
                    }
                    return Some(String::new());
                }

                // Handle cat commands
                if cmd_lower.starts_with("cat ") {
                    return Some("cat: file not found\n".to_string());
                }

                None
            }
        }
    }

    async fn handle_authentication<S>(&mut self, stream: &mut S) -> Result<bool, std::io::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        // Send SSH banner
        stream.write_all(b"SSH-2.0-OpenSSH_8.9p1 Ubuntu-3ubuntu0.1\r\n").await?;

        // Prompt for username
        stream.write_all(b"login as: ").await?;

        let mut buffer = [0u8; 1024];
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            return Ok(false);
        }

        self.username = String::from_utf8_lossy(&buffer[0..n]).trim().to_string();
        info!("SSH login attempt - username: {}", self.username);

        // Prompt for password
        stream.write_all(b"Password: ").await?;

        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            return Ok(false);
        }

        let password = String::from_utf8_lossy(&buffer[0..n]).trim().to_string();
        info!("SSH login attempt - username: {}, password: {}", self.username, password);

        // Always "succeed" authentication
        stream.write_all(b"Welcome to Ubuntu 22.04.1 LTS (GNU/Linux 5.15.0-56-generic x86_64)\r\n\r\n").await?;
        stream.write_all(b"Last login: Thu Dec 12 09:15:32 2025 from 192.168.1.50\r\n").await?;

        self.authenticated = true;
        Ok(true)
    }

    pub(crate) fn get_prompt(&self) -> String {
        format!("{}@{}:{}# ", self.username, self.hostname, self.current_dir)
    }
}

#[async_trait::async_trait]
impl<C: ChatService + Send + Sync> ProtocolHandler for SshHandler<C> {
    fn protocol_name(&self) -> &str {
        "SSH"
    }

    async fn handle_connection<S>(&mut self, mut stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        info!("SSH handler started");

        // Handle authentication
        match self.handle_authentication(&mut stream).await {
            Ok(true) => info!("SSH authentication successful"),
            Ok(false) => {
                info!("SSH connection closed during authentication");
                return;
            }
            Err(e) => {
                error!("SSH authentication error: {}", e);
                return;
            }
        }

        // Send initial prompt
        if let Err(e) = stream.write_all(self.get_prompt().as_bytes()).await {
            error!("Failed to send SSH prompt: {}", e);
            return;
        }

        // Main command loop
        let mut buffer = [0u8; 1024];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => {
                    info!("SSH connection closed");
                    break;
                }
                Ok(n) => {
                    let command = String::from_utf8_lossy(&buffer[0..n]).trim().to_string();

                    if command.is_empty() {
                        continue;
                    }

                    info!("SSH command received: {}", command);

                    self.session_state.commands_processed += 1;
                    self.session_state.last_command_time = Some(std::time::Instant::now());

                    // Check for exit commands
                    if command.to_lowercase() == "exit" || command.to_lowercase() == "quit" {
                        let _ = stream.write_all(b"logout\r\n").await;
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
                        info!("SSH: Escalating to LLM for command: {}", command);
                        self.session_state.llm_calls_made += 1;
                        match self.chat_service.send_message(&command).await {
                            Ok(resp) => resp,
                            Err(e) => {
                                error!("LLM error: {}", e);
                                format!("bash: {}: command not found\n", command)
                            }
                        }
                    } else if let Some(native_resp) = self.get_native_response(&command) {
                        info!("SSH: Using native response for command: {}", command);
                        native_resp
                    } else {
                        info!("SSH: Unknown command, incrementing counter: {}", command);
                        self.session_state.unknown_commands_count += 1;
                        format!("bash: {}: command not found\n", command)
                    };

                    // Send response
                    if let Err(e) = stream.write_all(response.as_bytes()).await {
                        error!("Failed to send SSH response: {}", e);
                        break;
                    }

                    // Send prompt
                    if let Err(e) = stream.write_all(self.get_prompt().as_bytes()).await {
                        error!("Failed to send SSH prompt: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("SSH read error: {}", e);
                    break;
                }
            }
        }

        info!(
            "SSH session ended. Commands: {}, LLM calls: {}, Duration: {:?}",
            self.session_state.commands_processed,
            self.session_state.llm_calls_made,
            self.session_state.session_duration()
        );
    }
}
