/// ChatService trait for sending messages to an LLM backend
#[async_trait::async_trait]
pub trait ChatService {
    async fn send_message(&self, message: &str) -> Result<String, String>;
}
