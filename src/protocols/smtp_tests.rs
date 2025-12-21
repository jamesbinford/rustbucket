// Tests for SMTP protocol handler
#[cfg(test)]
mod tests {
    use crate::protocols::smtp::SmtpHandler;
    use crate::protocols::LlmEscalationConfig;
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
    fn test_smtp_handler_creation() {
        let mock_chat = MockChatService {
            response: "mock response".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SmtpHandler::new(mock_chat, config);

        assert!(handler.mail_from.is_none());
        assert!(handler.rcpt_to.is_empty());
        assert!(!handler.in_data_mode);
    }

    #[test]
    fn test_smtp_is_known_command_basic() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SmtpHandler::new(mock_chat, config);

        assert!(handler.is_known_command("HELO example.com"));
        assert!(handler.is_known_command("EHLO example.com"));
        assert!(handler.is_known_command("QUIT"));
    }

    #[test]
    fn test_smtp_is_known_command_mail_flow() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SmtpHandler::new(mock_chat, config);

        assert!(handler.is_known_command("MAIL FROM:<test@example.com>"));
        assert!(handler.is_known_command("RCPT TO:<user@example.com>"));
        assert!(handler.is_known_command("DATA"));
    }

    #[test]
    fn test_smtp_is_known_command_other() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SmtpHandler::new(mock_chat, config);

        assert!(handler.is_known_command("RSET"));
        assert!(handler.is_known_command("NOOP"));
        assert!(handler.is_known_command("VRFY user"));
        assert!(handler.is_known_command("HELP"));
    }

    #[test]
    fn test_smtp_is_known_command_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = SmtpHandler::new(mock_chat, config);

        assert!(!handler.is_known_command("UNKNOWN"));
        assert!(!handler.is_known_command("EXPLOIT"));
    }

    #[test]
    fn test_smtp_get_native_response_helo() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("HELO example.com");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250"));
        assert!(resp.contains("Hello"));
    }

    #[test]
    fn test_smtp_get_native_response_ehlo() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("EHLO example.com");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250"));
        assert!(resp.contains("PIPELINING"));
    }

    #[test]
    fn test_smtp_get_native_response_mail_from() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("MAIL FROM:<sender@example.com>");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250 OK"));
        assert_eq!(handler.mail_from, Some("<SENDER@EXAMPLE.COM>".to_string())); // SMTP uppercases
    }

    #[test]
    fn test_smtp_get_native_response_rcpt_to() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("RCPT TO:<user@example.com>");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250 OK"));
        assert_eq!(handler.rcpt_to.len(), 1);
        assert_eq!(handler.rcpt_to[0], "<USER@EXAMPLE.COM>"); // SMTP uppercases
    }

    #[test]
    fn test_smtp_get_native_response_data() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("DATA");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("354"));
        assert!(resp.contains("Start mail input"));
        assert!(handler.in_data_mode);
    }

    #[test]
    fn test_smtp_get_native_response_rset() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        // Set some state
        handler.mail_from = Some("sender@example.com".to_string());
        handler.rcpt_to.push("user@example.com".to_string());
        handler.in_data_mode = true;

        let response = handler.get_native_response("RSET");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250 OK"));

        // State should be reset
        assert!(handler.mail_from.is_none());
        assert!(handler.rcpt_to.is_empty());
        assert!(!handler.in_data_mode);
    }

    #[test]
    fn test_smtp_get_native_response_noop() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("NOOP");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("250 OK"));
    }

    #[test]
    fn test_smtp_get_native_response_quit() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("QUIT");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("221"));
        assert!(resp.contains("Bye"));
    }

    #[test]
    fn test_smtp_get_native_response_vrfy() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("VRFY user");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("252"));
    }

    #[test]
    fn test_smtp_get_native_response_expn() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("EXPN list");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("502"));
    }

    #[test]
    fn test_smtp_get_native_response_help() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("HELP");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("214"));
    }

    #[test]
    fn test_smtp_get_native_response_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = SmtpHandler::new(mock_chat, config);

        let response = handler.get_native_response("UNKNOWN");
        assert!(response.is_none());
    }
}
