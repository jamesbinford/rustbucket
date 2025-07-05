# Rustbucket

![Rustbucket Logo](https://drive.google.com/uc?id=1RHe19I8YuFVjgXVx5BkwnxkhgLr9dzz6)

Rustbucket is a lightweight honeypot written in Rust that runs on virtual machines and containers exposed to the internet. By simulating common services on commonly attacked ports, Rustbucket captures and logs malicious activity for analysis.

The fun sauce is that Rustbucket also has a built-in ChatGPT integration, allowing it to generate responses to attackers in real-time. This can be used to confuse attackers, gather more information, or simply have fun with them.

Admittedly, ChatGPT can only pretend to be an Ubuntu server about 60% of the time. Nonetheless, it's a fun addition to the honeypot that can lead to some interesting interactions.
You can also modify the prompts yourself in Config.toml to make ChatGPT behavior however you'd like!

## Features

- **Protocol Emulation**: Mimics popular services such as SMTP, HTTP, and FTP.
- **Configurable Ports**: Easily configure which ports to monitor and the services to emulate through a TOML configuration file.
- **Logging**: Captures all interactions, providing valuable insights into potential attacks.
- **Concurrency**: Utilizes Rust’s async capabilities for handling multiple simultaneous connections efficiently.

### Prerequisites

- Rust (1.50 or later)
- Cargo (Rust’s package manager and build system)
- ChatGPT API Key with usage quota (it doesn't use much!)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/rustbucket.git
   cd rustbucket
    ```
2. Update Config.toml as needed. Feel free to customize the Prompt messages to make Rustbucket behave the way you want.
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Optionally, build it in a container:
   ```bash
   docker build -t rustbucket .
   ```
5. Make sure your OpenAI API key is set in a CHATGPT_API_KEY environment variable.