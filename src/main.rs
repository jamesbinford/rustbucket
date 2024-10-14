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
    println!("Listening on {}", addr);
    
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
                        8080 => {
                            // Handle connection for port 8080
                            println!("Welcome to 8080!");
                            handle_client(stream).await;
                        }
                        8181 => {
                            // Handle connection for port 8181
                            println!("Welcome to 8181!");
                            handle_client(stream).await;
                        }
                        2250 => {
                            // Handle connection for port 2250
                            println!("Welcome to 2250!");
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
    let ports = vec!["127.0.0.1:8080", "127.0.0.1:8181", "127.0.0.1:2250"];
    
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
