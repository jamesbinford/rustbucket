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
**Configuration**: Enabled by default
- **Purpose**: Secure shell simulation for credential attacks
- **Implementation**: Currently handled generically (no specific banner)
- **Common Attacks**: Brute force, key scanning, privilege escalation

### 5. Telnet (Port 23)
**Implementation**: Generic handling without specific protocol response
- **Purpose**: Legacy remote access simulation
- **Common Attacks**: Credential brute force, IoT device targeting

### 6. DNS (Port 53)
**Configuration**: Disabled by default
- **Purpose**: Domain name service simulation
- **Implementation**: Configurable but not actively implemented
- **Potential Attacks**: DNS amplification, cache poisoning

### 7. SMS (Port 5000)
**Configuration**: Disabled by default
- **Purpose**: Custom service simulation
- **Implementation**: Configurable custom port
- **Use Case**: Application-specific attack simulation

## Protocol Implementation Strategy

### Current Approach
1. **Static Banners**: Each protocol sends a realistic initial response
2. **AI Handoff**: All subsequent interaction handled by ChatGPT
3. **Context Switching**: Port number determines initial protocol context
4. **Generic Processing**: Same handler function for all protocols

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