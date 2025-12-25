use super::{SessionState, LlmEscalationConfig, Protocol};
use crate::chatgpt::ChatService;
use crate::fingerprint::ServerFingerprint;
use std::collections::HashSet;
use tracing::{info, error};

const KNOWN_SSH_COMMANDS: &[&str] = &[
    "ls", "ls -la", "ls -l", "pwd", "whoami", "id",
    "uname", "uname -a", "cat /etc/passwd", "cat /etc/shadow",
    "w", "ps", "ps aux", "netstat", "netstat -an",
    "ifconfig", "ip addr", "exit", "quit",
];

/// SSH Shell Simulator - handles command parsing and response generation
/// This is the post-authentication shell that simulates a real Linux environment
pub struct SshShellSimulator<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_commands: HashSet<String>,
    fingerprint: ServerFingerprint,
    pub hostname: String,
    pub username: String,
    pub current_dir: String,
}

impl<C: ChatService> SshShellSimulator<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig, fingerprint: ServerFingerprint) -> Self {
        let hostname = fingerprint.hostname.clone();
        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_commands: KNOWN_SSH_COMMANDS.iter().map(|s| s.to_string()).collect(),
            fingerprint,
            hostname,
            username: "root".to_string(),
            current_dir: "/root".to_string(),
        }
    }

    /// Set the username (captured during authentication)
    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }

    /// Check if a command is in our known command set
    pub fn is_known_command(&self, cmd: &str) -> bool {
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

    /// Get a native (hardcoded) response for a known command
    pub fn get_native_response(&mut self, cmd: &str) -> Option<String> {
        let cmd_lower = cmd.trim().to_lowercase();

        match cmd_lower.as_str() {
            "ls" | "ls -l" | "ls -la" => {
                Some("total 24\ndrwxr-xr-x 2 root root 4096 Dec 12 10:30 .\ndrwxr-xr-x 3 root root 4096 Dec 12 10:29 ..\n-rw-r--r-- 1 root root  220 Dec 12 10:29 .bash_logout\n-rw-r--r-- 1 root root 3526 Dec 12 10:29 .bashrc\n-rw-r--r-- 1 root root  807 Dec 12 10:29 .profile\n".to_string())
            }
            "pwd" => Some(format!("{}\n", self.current_dir)),
            "whoami" => Some(format!("{}\n", self.username)),
            "id" => Some(format!("uid=0({}) gid=0(root) groups=0(root)\n", self.username)),
            "uname" | "uname -a" => {
                Some(self.fingerprint.uname_output.clone())
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

    /// Get the shell prompt string
    pub fn get_prompt(&self) -> String {
        format!("{}@{}:{}# ", self.username, self.hostname, self.current_dir)
    }

    /// Check if this is an exit command
    pub fn is_exit_command(&self, cmd: &str) -> bool {
        let cmd_lower = cmd.trim().to_lowercase();
        cmd_lower == "exit" || cmd_lower == "quit"
    }

    /// Process a command and return the response
    /// This handles the full flow: known commands, LLM escalation, and unknown command handling
    pub async fn process_command(&mut self, command: &str) -> String {
        let command = command.trim();

        if command.is_empty() {
            return String::new();
        }

        info!(
            event_type = "command",
            protocol = "SSH",
            command = %command,
            "Command received"
        );

        self.session_state.commands_processed += 1;
        self.session_state.last_command_time = Some(std::time::Instant::now());

        // Determine if we should use LLM or native response
        let is_known = self.is_known_command(command);
        let use_llm = self.llm_config.should_use_llm(
            command,
            is_known,
            &self.session_state,
        );

        let response = if use_llm {
            info!(
                event_type = "llm",
                protocol = "SSH",
                command = %command,
                decision = "escalate",
                "LLM escalation"
            );
            self.session_state.llm_calls_made += 1;
            match self.chat_service.send_protocol_message(command, Protocol::Ssh).await {
                Ok(resp) => resp,
                Err(e) => {
                    error!(
                        event_type = "llm",
                        protocol = "SSH",
                        error = %e,
                        "LLM error"
                    );
                    format!("bash: {}: command not found\n", command)
                }
            }
        } else if let Some(native_resp) = self.get_native_response(command) {
            info!(
                event_type = "response",
                protocol = "SSH",
                command = %command,
                response_type = "native",
                "Using native response"
            );
            native_resp
        } else {
            info!(
                event_type = "command",
                protocol = "SSH",
                command = %command,
                "Unknown command"
            );
            self.session_state.unknown_commands_count += 1;
            format!("bash: {}: command not found\n", command)
        };

        response
    }

    /// Get session statistics for logging
    pub fn get_session_stats(&self) -> (u32, u32, std::time::Duration) {
        (
            self.session_state.commands_processed,
            self.session_state.llm_calls_made,
            self.session_state.session_duration(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[derive(Clone)]
    struct MockChatService;

    #[async_trait]
    impl ChatService for MockChatService {
        async fn send_message(&self, _message: &str) -> Result<String, String> {
            Ok("mock response\n".to_string())
        }
    }

    fn test_fingerprint() -> ServerFingerprint {
        ServerFingerprint::default_static()
    }

    #[test]
    fn test_shell_simulator_creation() {
        let chat_service = MockChatService;
        let llm_config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(chat_service, llm_config, test_fingerprint());

        assert_eq!(simulator.username, "root");
        assert_eq!(simulator.hostname, "ubuntu-server");
        assert_eq!(simulator.current_dir, "/root");
    }

    #[test]
    fn test_is_known_command() {
        let chat_service = MockChatService;
        let llm_config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(chat_service, llm_config, test_fingerprint());

        assert!(simulator.is_known_command("ls"));
        assert!(simulator.is_known_command("pwd"));
        assert!(simulator.is_known_command("cd /tmp"));
        assert!(simulator.is_known_command("cat /etc/passwd"));
        assert!(!simulator.is_known_command("unknown_command"));
    }

    #[test]
    fn test_get_prompt() {
        let chat_service = MockChatService;
        let llm_config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(chat_service, llm_config, test_fingerprint());

        assert_eq!(simulator.get_prompt(), "root@ubuntu-server:/root# ");

        simulator.set_username("attacker".to_string());
        assert_eq!(simulator.get_prompt(), "attacker@ubuntu-server:/root# ");
    }

    #[test]
    fn test_is_exit_command() {
        let chat_service = MockChatService;
        let llm_config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(chat_service, llm_config, test_fingerprint());

        assert!(simulator.is_exit_command("exit"));
        assert!(simulator.is_exit_command("quit"));
        assert!(simulator.is_exit_command("EXIT"));
        assert!(!simulator.is_exit_command("ls"));
    }
}
