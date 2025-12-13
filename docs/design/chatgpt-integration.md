# ChatGPT Integration Design

## Overview
Rustbucket integrates with OpenAI's ChatGPT API to generate realistic server responses to attacker input. This AI-powered approach allows the honeypot to engage attackers dynamically rather than using static response tables.

## Integration Architecture

The primary integration point for AI-driven responses is the `ChatService` trait, defined in `src/handler.rs`. This trait decouples the connection handler from a specific ChatGPT implementation.

### `ChatService` Trait (`src/handler.rs`)
```rust
#[async_trait::async_trait]
pub trait ChatService {
    async fn send_message(&self, message: &str) -> Result<String, String>;
}
```
- This trait defines a contract for any service that can take a user message (string) and asynchronously return a response (string) or an error (string).
- The `handle_client` function in `src/handler.rs` is generic over this trait, allowing different chat services to be plugged in.

### `ChatGPT` Struct Design (`src/chatgpt.rs`)
The `ChatGPT` struct is the concrete implementation of the `ChatService` trait using the OpenAI API.
```rust
#[derive(Debug, Clone)]
pub struct ChatGPT {
    api_key: String,
    static_messages: StaticMessages, // Holds system prompts from config
    client: Client, // reqwest HTTP client
}
```
- **API Key**: Stored from the configuration file (`Config.toml`).
- **Static Messages**: Pre-configured system prompts loaded from `Config.toml`.
- **HTTP Client**: Reusable `reqwest::Client` for making API calls to OpenAI.
- **Clone**: Allows instances of `ChatGPT` to be shared across async tasks (e.g., per connection).

It implements the `ChatService` trait:
```rust
#[async_trait::async_trait]
impl ChatService for ChatGPT {
    async fn send_message(&self, message: &str) -> Result<String, String> {
        // Calls its own inherent method `ChatGPT::send_message(self, message)`
        // and maps the Result<String, Box<dyn Error>> to Result<String, String>.
        match self::send_message(self, message).await { // Note: `self::send_message` refers to the struct's own method
            Ok(response) => Ok(response),
            Err(e) => Err(e.to_string()),
        }
    }
}
```

## Configuration System

### Configuration System
The `ChatGPT` struct requires configuration from two sources:

#### 1. Environment Variable
```bash
export CHATGPT_API_KEY="your-openai-api-key"
```
The API key is loaded from the `CHATGPT_API_KEY` environment variable for security.

#### 2. Config.toml Structure
```toml
[llm.static_messages]
# These messages are used to set the context for ChatGPT.
message1 = "Hi ChatGPT! You are the backend for a honeypot. An unknown user has connected to the honeypot and is executing actions on it. The user is not aware that they are interacting with a honeypot. The goal is to gather information about the user's intentions and actions. I need you to act like an Ubuntu server and respond to the user's commands like a server would."
message2 = "Please maintain the history of each command and always respond as if you were an actual Ubuntu server. Don't respond using full sentences, or the user will know it's you! If the user inputs an invalid command or text, please respond with 'Invalid Command'."
```

**Security Note**: The API key is NOT stored in the config file for security reasons.

### Configuration Loading (`src/chatgpt.rs`)
The `ChatGPT::from_config` method loads this configuration:
```rust
// Simplified from src/chatgpt.rs
pub fn from_config(_config_file: &str) -> Result<ChatGPT, Box<dyn Error>> {
    let settings = Config::builder()
        .add_source(File::with_name("Config.toml")) // Always uses Config.toml
        .build()?;
    
    // Load API key from environment variable for security
    let api_key = std::env::var("CHATGPT_API_KEY")
        .map_err(|_| Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ChatGPT API key not found in environment variable CHATGPT_API_KEY",
        )))?;
    
    // Load static messages from config file under [llm] section
    let llm_config_from_file: Option<OpenAIConfig> = settings.get("llm").ok();
    let static_messages = llm_config_from_file
        .map(|conf| conf.static_messages)
        .ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Static messages not found in config file",
            ))
        })?;

    Ok(ChatGPT {
        api_key,
        static_messages,
        client: Client::new(),
    })
}
```

## API Interaction Flow

### Request Structure (`src/chatgpt.rs`)
The request to OpenAI API is structured as follows:
```rust
#[derive(Serialize, Debug)]
struct ChatGPTRequest<'a> {
    model: &'a str,           // Currently hardcoded to "gpt-3.5-turbo" in src/chatgpt.rs
    messages: Vec<Message<'a>>, // A vector of system and user messages
}

#[derive(Serialize, Debug)]
struct Message<'a> {
    role: &'a str,            // "system" or "user"
    content: &'a str,         // Message content
}
```

### Message Flow
```rust
let messages = vec![
    Message {
        role: "system",
        content: &self.static_messages.message1,  // Honeypot context
    },
    Message {
        role: "system", 
        content: &self.static_messages.message2,  // Response guidelines
    },
    Message {
        role: "user",
        content: user_message,                    // Attacker input
    },
];
```

### Response Processing
```rust
#[derive(Deserialize, Debug)]
struct ChatGPTResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: MessageResponse,
}

#[derive(Deserialize, Debug)]
struct MessageResponse {
    content: String,  // AI-generated response
}
```

## Prompt Engineering Strategy

### System Message 1: Context Setting
```
"Hi ChatGPT! You are the backend for a honeypot. An unknown user has 
connected to the honeypot and is executing actions on it. The user is 
not aware that they are interacting with a honeypot. The goal is to 
gather information about the user's intentions and actions. I need you 
to act like an Ubuntu server and respond to the user's commands like 
a server would."
```

### System Message 2: Response Guidelines
```
"Please maintain the history of each command and always respond as if 
you were an actual Ubuntu server. Don't respond using full sentences, 
or the user will know it's you! If the user inputs an invalid command 
or text, please respond with 'Invalid Command'."
```

### Effectiveness Analysis
- **Success Rate**: ~60% realistic responses
- **Common Failures**: 
  - Too verbose (full sentences)
  - Breaking character (revealing AI nature)
  - Generic responses not matching protocol
  - Inappropriate command interpretation

## Error Handling Strategy

### API Failures
```rust
if !response.status().is_success() {
    let error_text = response.text().await?;
    error!("Error response from ChatGPT: {}", error_text);
    return Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to get a successful response from ChatGPT",
    )));
}
```

### Fallback Mechanisms
```rust
let response = chatgpt.send_message(&received_data).await
    .unwrap_or_else(|_| "Error processing request".to_string());
```

### Common Error Scenarios
- **Authentication**: Invalid API key
- **Rate Limiting**: Exceeding API quotas
- **Network Issues**: Connectivity problems
- **Quota Exhaustion**: Account limits reached
- **Service Downtime**: OpenAI API unavailable

## Performance Considerations

### API Call Overhead
- **Latency**: 200-2000ms per request (typical)
- **Throughput**: Limited by OpenAI rate limits
- **Concurrency**: Multiple simultaneous connections supported
- **Caching**: No response caching implemented

### Cost Management
- **Model Selection**: `gpt-3.5-turbo` for cost efficiency
- **Token Usage**: Minimal context to reduce costs
- **Rate Limiting**: Natural throttling via API limits
- **Monitoring**: Log all requests for cost tracking

## Security and Privacy

### API Key Management
- **Storage**: Environment variable only (`CHATGPT_API_KEY`)
- **Access**: Loaded from environment at runtime
- **Rotation**: Update environment variable and restart
- **Exposure Risk**: Environment access required

### Data Privacy
- **Attacker Data**: Sent to OpenAI servers
- **Retention**: Subject to OpenAI data policies
- **Compliance**: Consider GDPR/privacy implications
- **Anonymization**: No built-in data scrubbing

## Response Quality Optimization

### Current Limitations
1. **Context Loss**: No conversation history maintained
2. **Protocol Ignorance**: AI doesn't understand specific protocols
3. **Verbosity**: Often too chatty for server responses
4. **Consistency**: Responses may contradict previous interactions

### Potential Improvements

#### 1. Protocol-Specific Prompting
```rust
let protocol_prompt = match port {
    25 => "Respond only with SMTP status codes and minimal text",
    21 => "Use standard FTP response codes (220, 331, 530, etc.)",
    80 => "Return valid HTTP responses with appropriate headers",
    _ => "Respond like a terse Unix command line"
};
```

#### 2. Response Filtering
```rust
fn filter_response(response: &str, protocol: Protocol) -> String {
    match protocol {
        Protocol::SMTP if response.len() > 50 => "500 Error",
        Protocol::FTP if !response.starts_with(char::is_numeric) => "502 Command not implemented",
        _ => response.to_string()
    }
}
```

#### 3. Conversation History
```rust
struct Session {
    history: Vec<Message>,
    protocol: Protocol,
    start_time: Instant,
}
```

## Integration Points

### Connection Handler Integration
```rust
// In handle_client function
let response = chatgpt.send_message(&received_data).await
    .unwrap_or_else(|_| "Error processing request".to_string());
```

### Logging Integration
```rust
info!("We sent this to ChatGPT: {:?}", request_body);
info!("ChatGPT responded: {}", reply);
```

### Configuration Integration
- Shared config loading with main application
- Runtime reconfiguration not supported
- Static message customization via config file

## Future Enhancement Opportunities

### Advanced AI Features
- **Fine-tuning**: Custom model training on honeypot data
- **Multi-turn Conversations**: Maintain session context
- **Protocol Awareness**: Specialized models per protocol
- **Behavioral Learning**: Adapt responses based on attack patterns

### Alternative AI Providers
- **Local Models**: Ollama, GPT4All for privacy
- **Specialized Models**: Security-focused language models
- **Hybrid Approach**: Multiple models for different protocols
- **Custom Training**: Honeypot-specific model fine-tuning