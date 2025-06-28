# Rustbucket System Overview

## Purpose
Rustbucket is a lightweight honeypot written in Rust designed to capture and analyze malicious network activity. It simulates vulnerable services on commonly attacked ports and uses ChatGPT integration to generate realistic responses to attackers.

## Core Features
- **Multi-Protocol Support (Active)**: Simulates HTTP (80), FTP (21), SMTP (25), and Telnet (23) services.
- **Multi-Protocol Support (Designed)**: SSH (22), DNS (53), and custom ports (e.g., SMS 5000) are part of the design and configuration structure but not actively listening in the default `main.rs`.
- **AI-Powered Responses**: Utilizes a `ChatService` trait (implemented by `chatgpt.rs`) to integrate with OpenAI's ChatGPT for dynamic, contextual server responses.
- **Daily Rolling Logs**: Captures interactions using the `tracing` crate, with logs saved to daily rotating files (e.g., `logs/rustbucket.log.YYYY-MM-DD`).
- **Instance Registration (Optional)**: Includes a module (`registration.rs`) to register the honeypot instance with a central registry (currently not called from `main.rs`).
- **Configurable**: TOML-based configuration (`Config.toml`) for AI behavior (OpenAI API key, prompts) and instance registration URL. Port listening is currently hardcoded in `main.rs`.
- **Scalable**: Async architecture using Tokio for handling concurrent connections.

## High-Level Architecture

```
Internet → Multiple Port Listeners (Hardcoded: 21, 23, 25, 80)
             │
             └─→ Connection Handler (`handler.rs` with `ChatService` trait)
                   │
                   ├─→ ChatGPT (`chatgpt.rs` via OpenAI API) → Response to client
                   │
                   └─→ Logging (`tracing` to daily files)

(Optional, not in main flow currently)
Rustbucket Instance → Registration (`registration.rs`) → Central Registry
```

## System Components

### 1. Network Layer (`main.rs`)
- Multiple TCP listeners bound to `0.0.0.0` on hardcoded ports: 21 (FTP), 23 (Telnet), 25 (SMTP), 80 (HTTP).
- Each active port simulates a different service protocol, typically with a static banner followed by AI interaction.
- Async connection handling using Tokio tasks.

### 2. Protocol Simulation (`main.rs` for banners, `handler.rs` for interaction logic)
- Port-specific initial static banner responses for FTP, SMTP, HTTP sent from `main.rs`.
- Telnet connections, and all subsequent interactions for banner-based ports, are handled by `handler.rs`.
- `handler.rs` uses the `ChatService` trait for generic AI-driven interaction.

### 3. AI Response Engine (`chatgpt.rs`, `handler.rs`)
- `ChatService` trait in `handler.rs` defines the abstraction for AI responses.
- `chatgpt.rs` provides the concrete implementation using OpenAI's API.
- Context-aware prompting (via `Config.toml` static messages) aims to simulate Ubuntu server behavior.
- The `ChatService` trait returns a `Result<String, String>`, allowing handlers to process errors from the AI service.

### 4. Logging & Analytics (`main.rs` for setup, `tracing` ecosystem)
- Structured logging implemented using the `tracing` crate.
- `tracing_appender::rolling::daily` ensures logs are rotated into new files daily (e.g., `logs/rustbucket.log.YYYY-MM-DD`).
- Log levels and output are configured via `tracing_subscriber` in `main.rs`.
- (The previously designed S3 batch upload for logs is not currently implemented.)

### 5. Configuration Management (`Config.toml`, loaded by `chatgpt.rs`, `registration.rs`)
- TOML-based configuration file (`Config.toml`).
- Used for:
    - OpenAI API key and system prompt messages (`[openai]` section).
    - Instance registration URL (`[registration]` section).
- Port configuration (enabled services, port numbers) is defined in structs within `handler.rs` and can be described in `Config.toml` (`[ports]` section), but `main.rs` currently uses hardcoded ports for listeners and does not dynamically load this port configuration.

### 6. Instance Registration (`registration.rs`)
- An optional module that allows the Rustbucket instance to register itself with a central server.
- Generates a unique name and token for the instance.
- Sends this information via HTTP POST to a configurable URL.
- This functionality is present but not currently integrated into the main application flow in `main.rs`.

## Deployment Scenarios
- **Standalone Server**: Direct deployment on exposed VMs or physical servers.
- **Container Deployment**: Docker containerization is suitable for easy scaling and consistent environments (Dockerfile provided in repository).
- **Cloud Integration**: While direct S3 log upload is not current, logs are stored locally and can be collected by standard cloud agent-based log shippers if needed. Instance registration can point to a cloud-hosted registry.

## Security Considerations
- Honeypot nature requires internet exposure
- No authentication or authorization (intentional vulnerability)
- Logs may contain sensitive attack data requiring secure handling
- API keys stored in configuration files (excluded from version control)