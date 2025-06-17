use crate::prelude::*;
use serde::Deserialize;
// Removed: use crate::chatgpt::ChatGPT;
// Tokio I/O traits
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// Define the ChatService trait
#[async_trait::async_trait]
pub trait ChatService {
    async fn send_message(&self, message: &str) -> Result<String, String>;
}

#[derive(Debug, Deserialize)]
struct PortConfig {
	enabled: bool,
	port: u16,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
	ports: Ports,
}

#[derive(Debug, Deserialize)]
struct Ports {
	ssh: PortConfig,
	http: PortConfig,
	ftp: PortConfig,
	sftp: PortConfig,
	smtp: PortConfig,
	dns: PortConfig,
	sms: PortConfig,
}

// Updated handle_client function
pub async fn handle_client<S, C>(
    mut stream: S,
    _initial_message: String, // Renamed, as it's not used in the loop based on current logic
    chat_service: &C,
) where
    S: AsyncRead + AsyncWrite + Unpin,
    C: ChatService + Sync, // Added Sync bound as chat_service is shared across await points
{
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("Connection closed");
                break;
            }
            Ok(n) => {
                let received_data = String::from_utf8_lossy(&buffer[0..n]);
                // Use the chat_service trait method
                let response = chat_service
                    .send_message(&received_data)
                    .await
                    // Adjust error handling to match trait's Result<String, String>
                    .unwrap_or_else(|err_string| format!("Error processing request: {}", err_string));

                let response_message = format!("{}", response);
                info!("Received data: {}", received_data);
                info!("Response message: {}", response_message);

                if let Err(e) = stream.write_all(response_message.as_bytes()).await {
                    error!("Failed to send data: {}", e); // Changed to error!
                    info!("Failed to write data."); // This info! might be redundant if error! is used
                    break;
                }
            }
            Err(e) => {
                error!("Failed to read from stream: {}", e); // Changed to error!
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // use tokio::net::{TcpListener, TcpStream}; // No longer needed for mock stream test
    // use tokio::io::{AsyncReadExt, AsyncWriteExt}; // These are now part of the S trait bound
    // use std::error::Error; // Not needed as MockChatGPT error type is String
    // Note: crate::handler::handle_client is now generic.
    // crate::chatgpt::ChatGPT will be imported in main.rs and used there.
    use super::ChatService; // Import the new trait
    use super::handle_client; // Import the refactored handle_client
    use tokio_test::io::Builder as MockStreamBuilder; // For mocking the stream

    // Minimal mock for ChatGPT, now implementing ChatService
    #[derive(Clone, Default)] // Added Default
    struct MockChatGPT {
        success_response: String,
        error_response: Option<String>, // Some(error_message) to return Err, None to return Ok
    }

    #[async_trait::async_trait]
    impl ChatService for MockChatGPT {
        async fn send_message(&self, _message: &str) -> Result<String, String> {
            if let Some(err_msg) = &self.error_response {
                Err(err_msg.clone())
            } else {
                Ok(self.success_response.clone())
            }
        }
    }

    #[tokio::test]
    async fn test_handle_client_success() {
        // 1. Create a mock stream using tokio-test
        let mock_stream = MockStreamBuilder::new()
            // Simulate client sending "hello"
            .read(b"hello")
            // Expect server to write "Test response" (handle_client's format adds no newline for simple strings)
            .write(b"Test response")
            // Simulate client closing connection after response, or handle_client expecting further reads
            .read_error(std::io::ErrorKind::BrokenPipe.into()) // Or .read(b"") if a clean close is expected
            .build();

        // 2. Mock ChatGPT
        let mock_chat_service = MockChatGPT {
            success_response: "Test response".to_string(),
            error_response: None,
        };

        // 3. Call handle_client with the mock stream and mock ChatGPT
        // The `_initial_message` is not used by the loop, so an empty string is fine.
        handle_client(mock_stream, String::new(), &mock_chat_service).await;

        // Assertions are implicitly handled by the mock_stream's builder:
        // - It asserts that all expected reads happen.
        // - It asserts that all expected writes happen with the correct data.
        // - If the actual interaction deviates, the mock_stream will panic.
    }

    #[tokio::test]
    async fn test_handle_client_connection_closed() {
        let mock_stream = MockStreamBuilder::new()
            // Simulate client closes connection immediately (read returns Ok(0))
            .read(&[]) // read Ok(0)
            .build();

        let mock_chat_service = MockChatGPT {
            success_response: "Should not be called".to_string(),
            error_response: None,
        };

        // Expect handle_client to complete without panic and no writes to stream
        handle_client(mock_stream, String::new(), &mock_chat_service).await;
    }

    #[tokio::test]
    async fn test_handle_client_stream_read_error() {
        let mock_stream = MockStreamBuilder::new()
            // First read is successful
            .read(b"hello")
            // Server writes the response
            .write(b"Test response after hello")
            // Subsequent read attempt results in an error
            .read_error(std::io::Error::new(std::io::ErrorKind::Other, "test read error"))
            .build();

        let mock_chat_service = MockChatGPT {
            success_response: "Test response after hello".to_string(),
            error_response: None,
        };

        // Expect handle_client to complete without panic.
        // No further writes should occur after the read error.
        handle_client(mock_stream, String::new(), &mock_chat_service).await;
    }

    #[tokio::test]
    async fn test_handle_client_stream_write_error() {
        let mock_stream = MockStreamBuilder::new()
            .read(b"some input")
            // Expect server to attempt to write the response, but this write will fail
            .write_error(std::io::Error::new(std::io::ErrorKind::Other, "test write error"))
            .build();

        let mock_chat_service = MockChatGPT {
            success_response: "This will not be fully written".to_string(),
            error_response: None,
        };

        // Expect handle_client to complete without panic, even if writing response fails.
        handle_client(mock_stream, String::new(), &mock_chat_service).await;
    }

    #[tokio::test]
    async fn test_handle_client_chat_service_error() {
        let mock_stream = MockStreamBuilder::new()
            .read(b"trigger error")
            // Expect server to write the error message from ChatService, formatted by handle_client
            .write(b"Error processing request: Test chat service error")
            // Then, the loop should break or handle the next read.
            // Let's assume it breaks after an error, so simulate a closed read.
            .read_error(std::io::ErrorKind::BrokenPipe.into())
            .build();

        let mock_chat_service = MockChatGPT {
            success_response: String::new(), // Not used
            error_response: Some("Test chat service error".to_string()),
        };

        handle_client(mock_stream, String::new(), &mock_chat_service).await;
    }
}