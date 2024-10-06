mod handlers;
mod emulators;
mod logger;
mod config;

use std::net::TcpStream;
use std::net::TcpListener;
use config::Config;

#[tokio::main]
async fn main() {
    let config = Config::new();
    start_honeypot(config).await;
}

async fn start_honeypot(config: Config) {
    // Set up listeners for specified ports
    for port in config.ports {
        tokio::spawn(async move {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                tokio::spawn(handle_connection(socket, port));
            }
        });
    }
}

async fn handle_connection(socket: TcpStream, port: u16) {
    match port {
        22 => emulate_ssh(socket).await,
        // Add other ports and their respective emulation functions
        _ => {}
    }
}

async fn emulate_ssh(mut socket: TcpStream) {
    // Send SSH banner
    socket.write_all(b"SSH-2.0-Rustbucket\r\n").await.unwrap();
    // Log interactions, etc.
}
