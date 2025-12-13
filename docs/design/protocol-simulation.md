# Protocol Simulation Design

## Overview
Rustbucket simulates multiple network protocols to attract and capture malicious activity. Each protocol implementation provides just enough realism to engage attackers while maintaining simplicity.

## Supported Protocols

### 1. SMTP (Port 25)
**Initial Response**: `"220 mail.example.com ESMTP Postfix (Ubuntu)"`
- **Purpose**: Simulates mail server for spam/relay attempts
- **Implementation**: Static banner followed by ChatGPT responses
- **Common Attacks**: Open relay testing, credential harvesting

### 2. HTTP (Port 80)
**Initial Response**: `"GET / HTTP/1.1\nHost: example.com"`
- **Purpose**: Web server simulation for various web attacks
- **Implementation**: Fake HTTP request followed by AI responses
- **Common Attacks**: Directory traversal, injection attempts, bot scanning

### 3. FTP (Port 21)
**Initial Response**: `"220 (vsFTPd 3.0.3)"`
- **Purpose**: File transfer protocol for credential attacks
- **Implementation**: vsftpd banner simulation with AI interaction
- **Common Attacks**: Anonymous login attempts, brute force, directory listing

### 4. SSH (Port 22)
**Configuration**: Designed and documented in `Config.toml` examples, but **not currently implemented** in `main.rs` active listeners.
- **Purpose**: Secure shell simulation for credential attacks.
- **Implementation if Active**: Would likely be generic AI handling unless a specific banner is added.
- **Common Attacks**: Brute force, key scanning, privilege escalation.

### 5. Telnet (Port 23)
**Initial Response**: No specific banner. Connection directly handled by `handle_client` for generic AI interaction.
- **Implementation**: Generic AI handling via `handle_client` and `ChatService`.
- **Purpose**: Legacy remote access simulation.
- **Common Attacks**: Credential brute force, IoT device targeting.

### 6. DNS (Port 53)
**Configuration**: Designed and documented in `Config.toml` examples, but **not currently implemented** in `main.rs` active listeners.
- **Purpose**: Domain name service simulation.
- **Implementation if Active**: Would require specific DNS protocol handling; generic AI is not suitable.
- **Potential Attacks**: DNS amplification, cache poisoning.

### 7. SMS (Port 5000 - Example Custom Port)
**Configuration**: Designed as an example custom port in `Config.toml`, but **not currently implemented** in `main.rs` active listeners.
- **Purpose**: Custom service simulation.
- **Implementation if Active**: Would be generic AI handling unless specific logic is added.
- **Use Case**: Application-specific attack simulation.

## Protocol Implementation Strategy

### Current Approach (as seen in `src/main.rs` and `src/handler.rs`)
1.  **Initial Interaction (for specific ports)**:
    *   For ports 21 (FTP), 25 (SMTP), and 80 (HTTP), `main.rs` sends a predefined static banner message to the client. This initial banner is also passed (somewhat redundantly) as `_initial_message` to `handle_client`.
    *   For port 23 (Telnet) and any other configured-but-generic ports, no initial banner is sent by `main.rs` before handing off to `handle_client`.
2.  **AI Handoff**: All subsequent interaction for all active ports is managed by `handle_client`, which uses the `ChatService` trait (implemented by `src/chatgpt.rs`) to get responses. The `_initial_message` argument in `handle_client` is currently not used within its loop for subsequent interactions.
3.  **Generic Processing**: The same `handle_client` function, utilizing the `ChatService` abstraction, is used for all protocols after their optional initial static banner. The AI's behavior is primarily guided by the static system prompts configured in `Config.toml` for the `ChatGPT` service.

### Protocol Realism Levels

#### High Realism (Recommended)
```rust
// Example: Full SMTP command handling
match command {
    "HELO" | "EHLO" => "250 Hello",
    "MAIL FROM:" => "250 OK",
    "RCPT TO:" => "250 OK",
    "DATA" => "354 Start mail input",
    _ => chatgpt_response
}
```

#### Current Implementation (Basic)
```rust
// Current: Static banner + AI
let banner = match port {
    25 => "220 mail.example.com ESMTP Postfix (Ubuntu)",
    21 => "220 (vsFTPd 3.0.3)",
    80 => "GET / HTTP/1.1\nHost: example.com",
    _ => ""
};
// All subsequent interaction via ChatGPT
```

## ChatGPT Integration Strategy

### System Prompts
The AI is primed with two key messages:
1. **Context Setting**: "Act like an Ubuntu server in a honeypot"
2. **Response Style**: "Don't use full sentences, respond like a real server"

### Response Quality
- **Success Rate**: ~60% realistic responses
- **Failure Modes**: Too verbose, breaks character, generic responses
- **Mitigation**: Fallback to "Invalid Command" for poor responses

### Protocol-Specific Prompting
Future enhancement could include protocol-specific system prompts:
```rust
let system_prompt = match port {
    25 => "You are an SMTP server. Respond with proper SMTP codes.",
    21 => "You are an FTP server. Use standard FTP response codes.",
    80 => "You are a web server. Return HTTP responses.",
    _ => "You are a generic Ubuntu server."
};
```

## Attack Detection Patterns

### Common Attack Signatures
- **Credential Stuffing**: Repeated login attempts
- **Directory Traversal**: `../` patterns in requests
- **SQL Injection**: SQL keywords and special characters
- **Command Injection**: Shell metacharacters
- **Port Scanning**: Rapid connection/disconnection patterns

### Logging Strategy
Each protocol interaction logs:
- Source IP and connection metadata
- Raw input received from attacker
- Protocol-specific context (port, service type)
- AI-generated response sent back
- Connection duration and termination reason

## Future Protocol Enhancements

### Proposed Improvements
1. **Stateful Protocols**: Maintain session state for complex protocols
2. **File System Simulation**: Mock file/directory structures for FTP/HTTP
3. **Authentication Simulation**: Fake credential validation with delays
4. **Protocol Compliance**: More accurate protocol implementation
5. **Vulnerability Simulation**: Intentional weaknesses to attract specific attacks

### Implementation Considerations
- **Performance**: Balance realism with resource usage
- **Maintenance**: More complex protocols require more upkeep
- **Detection Avoidance**: Too perfect simulation may reveal honeypot nature
- **Legal Compliance**: Ensure simulated vulnerabilities don't create real risks