# Rustbucket

Rustbucket is a lightweight honeypot written in Rust that runs on virtual machines and containers exposed to the internet. By simulating common services on commonly attacked ports, Rustbucket captures and logs malicious activity for analysis.

## Features

- **Protocol Emulation**: Mimics popular services such as SSH and HTTP.
- **Configurable Ports**: Easily configure which ports to monitor and the services to emulate through a TOML configuration file.
- **Logging**: Captures all interactions, providing valuable insights into potential attacks.
- **Concurrency**: Utilizes Rust’s async capabilities for handling multiple simultaneous connections efficiently.

## Getting Started

### Prerequisites

- Rust (1.50 or later)
- Cargo (Rust’s package manager and build system)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/rustbucket.git
   cd rustbucket
