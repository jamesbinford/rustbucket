use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task;

async fn handle_client(mut stream: tokio::net::TcpStream) {
    let mut buffer = [0; 1024];
    
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                break;
            }
            Ok(n) => {
                // Echo the received data back to the client
                if let Err(e) = stream.write_all(&buffer[0..n]).await {
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
                        22 => {
                            // Handle connection for port 22
                            println!("Welcome to SSH!");
                            handle_client(stream).await;
                        }
                        80 => {
                            // Handle connection for port 80
                            println!("Welcome to Web!");
                            handle_client(stream).await;
                        }
                        25 => {
                            // Handle connection for port 25
                            println!("Welcome to SMTP!");
                            handle_client(stream).await;
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
    let ports = vec!["127.0.0.1:22", "127.0.0.1:80", "127.0.0.1:25"];
    
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
