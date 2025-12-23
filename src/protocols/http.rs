use super::{ProtocolHandler, SessionState, LlmEscalationConfig};
use crate::handler::ChatService;
use crate::prelude::*;
use crate::rate_limiter::RateLimiterRef;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::collections::HashSet;

#[cfg(test)]
#[path = "http_tests.rs"]
mod http_tests;

/// HTTP Protocol Handler
pub struct HttpHandler<C: ChatService> {
    chat_service: C,
    session_state: SessionState,
    llm_config: LlmEscalationConfig,
    known_paths: HashSet<String>,
    rate_limiter: RateLimiterRef,
}

impl<C: ChatService> HttpHandler<C> {
    pub fn new(chat_service: C, llm_config: LlmEscalationConfig, rate_limiter: RateLimiterRef) -> Self {
        let mut known_paths = HashSet::new();
        known_paths.insert("/".to_string());
        known_paths.insert("/index.html".to_string());
        known_paths.insert("/index.php".to_string());
        known_paths.insert("/admin".to_string());
        known_paths.insert("/admin/".to_string());
        known_paths.insert("/login".to_string());
        known_paths.insert("/login.php".to_string());
        known_paths.insert("/wp-admin".to_string());
        known_paths.insert("/wp-admin/".to_string());
        known_paths.insert("/wp-login.php".to_string());
        known_paths.insert("/phpmyadmin".to_string());
        known_paths.insert("/phpmyadmin/".to_string());
        known_paths.insert("/administrator".to_string());
        known_paths.insert("/robots.txt".to_string());
        known_paths.insert("/.env".to_string());
        known_paths.insert("/.git/config".to_string());

        Self {
            chat_service,
            session_state: SessionState::new(),
            llm_config,
            known_paths,
            rate_limiter,
        }
    }

    pub(crate) fn parse_http_request(&self, data: &str) -> Option<(String, String)> {
        let lines: Vec<&str> = data.lines().collect();
        if lines.is_empty() {
            return None;
        }

        let request_line = lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() < 2 {
            return None;
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();

        Some((method, path))
    }

    pub(crate) fn is_known_path(&self, path: &str) -> bool {
        self.known_paths.contains(path)
    }

    pub(crate) fn get_native_response(&mut self, method: &str, path: &str) -> Option<String> {
        match path {
            "/" | "/index.html" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>Welcome</title></head><body><h1>It works!</h1></body></html>",
                ))
            }
            "/index.php" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>Apache2 Ubuntu Default Page</title></head><body><h1>Apache2 Ubuntu Default Page</h1><p>It works!</p></body></html>",
                ))
            }
            "/admin" | "/admin/" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>Admin Login</title></head><body><h1>Admin Panel</h1><form><input type=\"text\" name=\"username\" placeholder=\"Username\"><input type=\"password\" name=\"password\" placeholder=\"Password\"><button type=\"submit\">Login</button></form></body></html>",
                ))
            }
            "/login" | "/login.php" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>Login</title></head><body><h1>Login</h1><form method=\"post\"><input type=\"text\" name=\"username\"><input type=\"password\" name=\"password\"><button type=\"submit\">Login</button></form></body></html>",
                ))
            }
            "/wp-admin" | "/wp-admin/" | "/wp-login.php" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>WordPress Login</title></head><body><h1>WordPress</h1><form name=\"loginform\" id=\"loginform\" action=\"/wp-login.php\" method=\"post\"><p><label for=\"user_login\">Username</label><input type=\"text\" name=\"log\" id=\"user_login\"></p><p><label for=\"user_pass\">Password</label><input type=\"password\" name=\"pwd\" id=\"user_pass\"></p><p class=\"submit\"><input type=\"submit\" name=\"wp-submit\" id=\"wp-submit\" value=\"Log In\"></p></form></body></html>",
                ))
            }
            "/phpmyadmin" | "/phpmyadmin/" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/html",
                    "<html><head><title>phpMyAdmin</title></head><body><h1>phpMyAdmin</h1><form method=\"post\" action=\"index.php\"><fieldset><legend>Log in</legend><label for=\"input_username\">Username:</label><input type=\"text\" name=\"pma_username\" id=\"input_username\"><label for=\"input_password\">Password:</label><input type=\"password\" name=\"pma_password\" id=\"input_password\"><input type=\"submit\" value=\"Go\"></fieldset></form></body></html>",
                ))
            }
            "/administrator" => {
                Some(self.build_http_response(
                    "301 Moved Permanently",
                    "text/html",
                    "<html><body>Redirecting to admin panel...</body></html>",
                ))
            }
            "/robots.txt" => {
                Some(self.build_http_response(
                    "200 OK",
                    "text/plain",
                    "User-agent: *\nDisallow: /admin/\nDisallow: /private/\n",
                ))
            }
            "/.env" => {
                Some(self.build_http_response(
                    "403 Forbidden",
                    "text/html",
                    "<html><body><h1>403 Forbidden</h1></body></html>",
                ))
            }
            "/.git/config" => {
                Some(self.build_http_response(
                    "403 Forbidden",
                    "text/html",
                    "<html><body><h1>403 Forbidden</h1></body></html>",
                ))
            }
            _ => {
                // Default 404 response
                if method == "GET" || method == "POST" || method == "HEAD" {
                    Some(self.build_http_response(
                        "404 Not Found",
                        "text/html",
                        "<html><body><h1>404 Not Found</h1></body></html>",
                    ))
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn build_http_response(&self, status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {}\r\nServer: Apache/2.4.52 (Ubuntu)\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            content_type,
            body.len(),
            body
        )
    }
}

#[async_trait::async_trait]
impl<C: ChatService + Send + Sync> ProtocolHandler for HttpHandler<C> {
    async fn handle_connection<S>(&mut self, mut stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        info!("HTTP handler started");

        let mut buffer = [0u8; 4096];
        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("HTTP connection closed immediately");
                return;
            }
            Ok(n) => {
                let request_data = String::from_utf8_lossy(&buffer[0..n]);
                info!("HTTP request received:\n{}", request_data);

                self.session_state.commands_processed += 1;
                self.session_state.last_command_time = Some(std::time::Instant::now());

                // Parse HTTP request
                let (method, path) = match self.parse_http_request(&request_data) {
                    Some((m, p)) => (m, p),
                    None => {
                        error!("Failed to parse HTTP request");
                        let _ = stream.write_all(
                            self.build_http_response(
                                "400 Bad Request",
                                "text/html",
                                "<html><body><h1>400 Bad Request</h1></body></html>",
                            ).as_bytes()
                        ).await;
                        return;
                    }
                };

                info!("HTTP {} {}", method, path);

                // Determine if we should use LLM or native response
                let is_known = self.is_known_path(&path);
                let use_llm = self.llm_config.should_use_llm(
                    &format!("{} {}", method, path),
                    is_known,
                    &self.session_state,
                );

                let response = if use_llm {
                    info!("HTTP: Escalating to LLM for {} {}", method, path);
                    self.session_state.llm_calls_made += 1;
                    match self.chat_service.send_message(&request_data).await {
                        Ok(resp) => {
                            // LLM response might not be valid HTTP, so wrap it
                            self.build_http_response("200 OK", "text/plain", &resp)
                        }
                        Err(e) => {
                            error!("LLM error: {}", e);
                            self.build_http_response(
                                "500 Internal Server Error",
                                "text/html",
                                "<html><body><h1>500 Internal Server Error</h1></body></html>",
                            )
                        }
                    }
                } else if let Some(native_resp) = self.get_native_response(&method, &path) {
                    info!("HTTP: Using native response for {} {}", method, path);
                    native_resp
                } else {
                    info!("HTTP: Unknown path, incrementing counter: {}", path);
                    self.session_state.unknown_commands_count += 1;
                    self.build_http_response(
                        "404 Not Found",
                        "text/html",
                        "<html><body><h1>404 Not Found</h1></body></html>",
                    )
                };

                // Apply response delay for realism
                self.rate_limiter.apply_response_delay().await;

                // Send response
                if let Err(e) = stream.write_all(response.as_bytes()).await {
                    error!("Failed to send HTTP response: {}", e);
                }
            }
            Err(e) => {
                error!("HTTP read error: {}", e);
            }
        }

        info!(
            "HTTP session ended. Requests: {}, LLM calls: {}, Duration: {:?}",
            self.session_state.commands_processed,
            self.session_state.llm_calls_made,
            self.session_state.session_duration()
        );
    }
}
