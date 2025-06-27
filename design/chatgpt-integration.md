# ChatGPT Integration Design

## Overview
Rustbucket integrates with OpenAI's ChatGPT API to generate realistic server responses to attacker input. This AI-powered approach allows the honeypot to engage attackers dynamically rather than using static response tables.

## Integration Architecture

### ChatGPT Struct Design
```rust
#[derive(Debug, Clone)]
pub struct ChatGPT {
    api_key: String,
    static_messages: StaticMessages,
    client: Client,
}
```
- **API Key**: Stored from configuration file
- **Static Messages**: Pre-configured system prompts
- **HTTP Client**: Reusable reqwest client for API calls
- **Clone**: Allows sharing across async tasks

## Configuration System

### Config.toml Structure
```toml
[openai]
api_key = "your-openai-api-key"

[openai.static_messages]
message1 = "Hi ChatGPT! You are the backend for a honeypot..."
message2 = "Please maintain the history of each command..."
message3 = "The user has closed the session..."
```

### Configuration Loading
```rust
pub fn from_config(config_file: &str) -> Result<ChatGPT, Box<dyn Error>> {
    let settings = Config::builder()
        .add_source(File::with_name("Config.toml"))
        .build()?;
    
    let openai_config: OpenAIConfig = settings.get::<OpenAIConfig>("openai")?;
    // Initialize ChatGPT struct
}
```

## API Interaction Flow

### Request Structure
```rust
#[derive(Serialize, Debug)]
struct ChatGPTRequest<'a> {
    model: &'a str,           // "gpt-3.5-turbo"
    messages: Vec<Message<'a>>,
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
- **Storage**: Configuration file (excluded from git)
- **Access**: Environment variable alternative available
- **Rotation**: Manual key rotation required
- **Exposure Risk**: Config file access = API access

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