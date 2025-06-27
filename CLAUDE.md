# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

- Build: `cargo build --release`
- Run: `cargo run`
- Test: `cargo test`
- Check: `cargo check`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Docker build: `docker build -t rustbucket .`

## Project Architecture

Rustbucket is a Rust-based honeypot that simulates vulnerable services to capture and log malicious activity. The application uses an async architecture with Tokio for handling multiple concurrent connections.

### Core Components

- **main.rs**: Entry point that spawns multiple TCP listeners on different ports (21, 23, 25, 80)
- **handler.rs**: Contains `handle_client()` function that processes incoming connections and forwards user input to ChatGPT
- **chatgpt.rs**: ChatGPT integration module with configuration loading and API communication
- **prelude.rs**: Common imports and utilities used across modules
- **log_*.rs modules**: Log management system for batching, compressing, and uploading logs

### Configuration

The application uses `Config.toml` for configuration with sections:
- `[general]`: Log level, directory, and verbosity settings
- `[ports]`: Enable/disable specific service ports (SSH, HTTP, FTP, SMTP, DNS, SMS)
- `[openai]`: API key and static prompt messages for ChatGPT integration

### Key Architectural Patterns

1. **Async Connection Handling**: Each incoming connection spawns a new Tokio task
2. **Protocol Simulation**: Different ports trigger different service responses (SMTP on 25, HTTP on 80, FTP on 21)
3. **ChatGPT Integration**: User input is forwarded to OpenAI API to generate realistic server responses
4. **Structured Logging**: Uses tracing crate with daily rolling logs in `logs/` directory

### Important Notes

- Configuration file `Config.toml` is excluded from package builds (see Cargo.toml exclude)
- Static messages in config are used to prime ChatGPT to act like an Ubuntu server
- The application listens on 0.0.0.0 (all interfaces) for maximum exposure as a honeypot
- Error handling includes fallback responses when ChatGPT API fails