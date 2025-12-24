// Tests for HTTP protocol handler
#[cfg(test)]
mod tests {
    use crate::protocols::http::HttpHandler;
    use crate::protocols::LlmEscalationConfig;
    use crate::chatgpt::ChatService;
    use crate::config::TarpitConfig;
    use crate::rate_limiter::RateLimiter;
    use std::sync::Arc;

    fn test_rate_limiter() -> Arc<RateLimiter> {
        Arc::new(RateLimiter::default())
    }

    fn test_tarpit_config() -> TarpitConfig {
        TarpitConfig::default()
    }

    fn new_test_handler(mock_chat: MockChatService, config: LlmEscalationConfig) -> HttpHandler<MockChatService> {
        HttpHandler::new(mock_chat, config, test_rate_limiter(), test_tarpit_config())
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
    fn test_http_handler_creation() {
        let mock_chat = MockChatService {
            response: "mock response".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.known_paths.contains("/"));
    }

    #[test]
    fn test_http_parse_request_valid() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        let request = "GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = handler.parse_http_request(request);
        assert!(result.is_some());
        let (method, path) = result.unwrap();
        assert_eq!(method, "GET");
        assert_eq!(path, "/index.html");
    }

    #[test]
    fn test_http_parse_request_post() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        let request = "POST /login HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = handler.parse_http_request(request);
        assert!(result.is_some());
        let (method, path) = result.unwrap();
        assert_eq!(method, "POST");
        assert_eq!(path, "/login");
    }

    #[test]
    fn test_http_parse_request_invalid() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        let request = "INVALID";
        let result = handler.parse_http_request(request);
        assert!(result.is_none());
    }

    #[test]
    fn test_http_is_known_path_root() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.is_known_path("/"));
        assert!(handler.is_known_path("/index.html"));
        assert!(handler.is_known_path("/index.php"));
    }

    #[test]
    fn test_http_is_known_path_admin() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.is_known_path("/admin"));
        assert!(handler.is_known_path("/admin/"));
        assert!(handler.is_known_path("/login"));
    }

    #[test]
    fn test_http_is_known_path_wordpress() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(handler.is_known_path("/wp-admin"));
        assert!(handler.is_known_path("/wp-login.php"));
    }

    #[test]
    fn test_http_is_known_path_unknown() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        assert!(!handler.is_known_path("/random/path"));
        assert!(!handler.is_known_path("/exploit.php"));
    }

    #[test]
    fn test_http_get_native_response_root() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("It works!"));
    }

    #[test]
    fn test_http_get_native_response_admin() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/admin");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("Admin Panel"));
    }

    #[test]
    fn test_http_get_native_response_phpmyadmin() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/phpmyadmin");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("phpMyAdmin"));
    }

    #[test]
    fn test_http_get_native_response_robots() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/robots.txt");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("200 OK"));
        assert!(resp.contains("User-agent"));
    }

    #[test]
    fn test_http_get_native_response_forbidden() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/.env");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("403 Forbidden"));
    }

    #[test]
    fn test_http_get_native_response_404() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let mut handler = new_test_handler(mock_chat, config);

        let response = handler.get_native_response("GET", "/nonexistent");
        assert!(response.is_some());
        let resp = response.unwrap();
        assert!(resp.contains("404 Not Found"));
    }

    #[test]
    fn test_http_build_response() {
        let mock_chat = MockChatService {
            response: "".to_string(),
        };
        let config = LlmEscalationConfig::default();
        let handler = new_test_handler(mock_chat, config);

        let response = handler.build_http_response("200 OK", "text/html", "<html></html>");
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Type: text/html"));
        assert!(response.contains("Content-Length: 13"));
        assert!(response.contains("<html></html>"));
    }
}
