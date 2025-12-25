// Tests for FTP protocol handler
#[cfg(test)]
mod tests {
    use crate::protocols::ftp::FtpHandler;
    use crate::protocols::LlmEscalationConfig;
    use crate::chatgpt::ChatService;
    use crate::config::TarpitConfig;
    use crate::fingerprint::ServerFingerprint;
    use crate::rate_limiter::RateLimiter;
    use std::sync::Arc;

    fn test_rate_limiter() -> Arc<RateLimiter> {
        Arc::new(RateLimiter::default())
    }

    fn test_tarpit_config() -> TarpitConfig {
        TarpitConfig::default()
    }

    fn test_fingerprint() -> ServerFingerprint {
        ServerFingerprint::default_static()
    }

    fn new_test_handler(mock_chat: MockChatService, config: LlmEscalationConfig) -> FtpHandler<MockChatService> {
        FtpHandler::new(mock_chat, config, test_rate_limiter(), test_tarpit_config(), test_fingerprint())
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
    fn test_ftp_handler_creation() {
        let mock_chat = MockChatService {
            response: "mock response".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert_eq!(handler.current_dir, "/");
        assert!(handler.username.is_none());
        assert!(!handler.authenticated);
    }

    #[test]
    fn test_ftp_is_known_command_basic() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.is_known_command("USER admin"));
        assert!(handler.is_known_command("PASS password"));
        assert!(handler.is_known_command("QUIT"));
        assert!(handler.is_known_command("PWD"));
    }

    #[test]
    fn test_ftp_is_known_command_file_operations() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.is_known_command("LIST"));
        assert!(handler.is_known_command("RETR file.txt"));
        assert!(handler.is_known_command("STOR file.txt"));
        assert!(handler.is_known_command("DELE file.txt"));
    }

    #[test]
    fn test_ftp_is_known_command_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(!handler.is_known_command("UNKNOWN"));
        assert!(!handler.is_known_command("EXPLOIT"));
    }

    #[test]
    fn test_ftp_get_native_response_user() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("USER admin");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("331"));
        assert!(resp.contains("password"));
        assert_eq!(handler.username, Some("ADMIN".to_string())); // FTP uppercases usernames
    }

    #[test]
    fn test_ftp_get_native_response_pass() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("PASS secret");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("230"));
        assert!(resp.contains("Login successful"));
        assert!(handler.authenticated);
    }

    #[test]
    fn test_ftp_get_native_response_pwd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("PWD");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("257"));
        assert!(resp.contains("/"));
    }

    #[test]
    fn test_ftp_get_native_response_cwd() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        assert_eq!(handler.current_dir, "/");

        let response = handler.get_native_response("CWD /tmp");
        assert!(response.is_some());
        assert_eq!(handler.current_dir, "/TMP"); // FTP uppercases paths

        let response = handler.get_native_response("CDUP");
        assert!(response.is_some());
        assert_eq!(handler.current_dir, "/");
    }

    #[test]
    fn test_ftp_get_native_response_list() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("LIST");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("150"));
        assert!(resp.contains("226"));
        assert!(resp.contains("file1.txt"));
    }

    #[test]
    fn test_ftp_get_native_response_retr() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("RETR file.txt");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("550"));
    }

    #[test]
    fn test_ftp_get_native_response_stor() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("STOR file.txt");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("553"));
    }

    #[test]
    fn test_ftp_get_native_response_type() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("TYPE I");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("200"));
        assert!(resp.contains("Switching to I mode"));
    }

    #[test]
    fn test_ftp_get_native_response_pasv() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("PASV");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("227"));
        assert!(resp.contains("Passive Mode"));
    }

    #[test]
    fn test_ftp_get_native_response_quit() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("QUIT");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("221"));
        assert!(resp.contains("Goodbye"));
    }

    #[test]
    fn test_ftp_get_native_response_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("UNKNOWN");
        assert!(response.is_none());
    }
}
