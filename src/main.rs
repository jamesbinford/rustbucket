use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task;

async fn handle_client(mut stream: tokio::net::TcpStream, message: String) {
    let mut buffer = [0; 1024];
    
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                break;
            }
            Ok(n) => {
                // Pass user input to ChatGPT, parse the GPT response and send it back to the user
                // @TODO: Implement ChatGPT API
                let received_data = String::from_utf8_lossy(&buffer[0..n]);
                let response_message = format!("{}", message);
                
                if let Err(e) = stream.write_all(response_message.as_bytes()).await {
                    println!("Failed to send data: {}", e);
                    break;
                }
            }
            Err(e) => {
                println!("Failed to read from stream: {}", e);
                break;
            }
        }
    }
}

async fn start_listener(addr: &str) -> tokio::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;    
    // Retrieve the actual address and port the listener is bound to
    let listener_addr = listener.local_addr()?;
    println!("Listening on {}", listener_addr);
    
    loop {
        match listener.accept().await {
            Ok((stream, client_addr)) => {
                let port = client_addr.port();
                println!("New connection on {}: {}", client_addr, client_addr);
                // Spawn a new task to handle the connection asynchronously
                task::spawn(async move {
                    match listener_addr.port() {                        
                        25 => {
                            // Handle connection for port 25
                            println!("Welcome to SMTP!");
                            let message = "220 mail.example.com ESMTP Postfix (Ubuntu)".to_string();
                            handle_client(stream, message).await;
                        }
                        80 => {
                            // Handle connection for port 23
                            println!("Welcome to Telnet!");
                            let message = "GET / HTTP/1.1\nHost: example.com".to_string();
                            handle_client(stream, message).await;
                        }
                        21 => {
                            // Handle connection for port 21
                            println!("Welcome to FTP!");
                            let message = "220 (vsFTPd 3.0.3)".to_string();
                            handle_client(stream, message).await;
                        }
                        _ => {
                            println!("Unexpected port: {}", port);
                        }
                    }
                });
            }
            Err(e) => {
                println!("Failed to accept connection: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    // Create tasks for each listener on different ports
    let ports = vec!["127.0.0.1:25", "127.0.0.1:23", "127.0.0.1:21"];
    
    let mut handles = vec![];
    
    for port in ports {
        let handle = tokio::spawn(async move {
            start_listener(port).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all listeners to finish (this will run indefinitely)
    for handle in handles {
        handle.await.unwrap();
    }
    
    Ok(())
}
