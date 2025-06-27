# Rustbucket System Overview

## Purpose
Rustbucket is a lightweight honeypot written in Rust designed to capture and analyze malicious network activity. It simulates vulnerable services on commonly attacked ports and uses ChatGPT integration to generate realistic responses to attackers.

## Core Features
- **Multi-Protocol Support**: Simulates SSH (22), HTTP (80), FTP (21), SMTP (25), DNS (53), and SMS (5000) services
- **AI-Powered Responses**: Integrates with OpenAI's ChatGPT to generate contextual server responses
- **Comprehensive Logging**: Captures all interactions with structured logging and log management
- **Configurable**: TOML-based configuration for ports, logging, and AI behavior
- **Scalable**: Async architecture using Tokio for handling concurrent connections

## High-Level Architecture

```
Internet → Multiple Port Listeners → Connection Handler → ChatGPT API → Response
                                         ↓
                                   Log Management System
                                         ↓
                                   Batch/Compress/Upload
```

## System Components

### 1. Network Layer
- Multiple TCP listeners bound to `0.0.0.0` on various ports
- Each port simulates a different service protocol
- Async connection handling with Tokio tasks

### 2. Protocol Simulation
- Port-specific initial responses (SMTP banners, HTTP headers, etc.)
- Service-appropriate error messages and behaviors
- Protocol emulation varies by service type

### 3. AI Response Engine
- ChatGPT integration for dynamic response generation
- Context-aware prompting to simulate Ubuntu server behavior
- Fallback responses when AI service is unavailable

### 4. Logging & Analytics
- Structured logging using the `tracing` crate
- Daily rolling log files in `logs/` directory
- Log batching, compression, and S3 upload capabilities

### 5. Configuration Management
- TOML-based configuration files
- Runtime configuration of ports, logging, and AI behavior
- Separate example configuration for deployment

## Deployment Scenarios
- **Standalone Server**: Direct deployment on exposed VMs
- **Container Deployment**: Docker containerization for easy scaling
- **Cloud Integration**: AWS S3 integration for log storage and analysis

## Security Considerations
- Honeypot nature requires internet exposure
- No authentication or authorization (intentional vulnerability)
- Logs may contain sensitive attack data requiring secure handling
- API keys stored in configuration files (excluded from version control)