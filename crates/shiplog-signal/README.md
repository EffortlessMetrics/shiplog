# shiplog-signal

Signal handling utilities for shiplog.

## Description

This crate provides signal handling utilities for the shiplog ecosystem, allowing graceful handling of OS signals like SIGINT, SIGTERM, etc.

## Usage

```rust
use shiplog_signal::{SignalHandler, create_shutdown_channel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = create_shutdown_channel();
    
    // Spawn signal handler
    tokio::spawn(async move {
        match SignalHandler::wait_for_signal().await {
            Ok(signal) => {
                println!("Received signal: {:?}", signal);
            }
            Err(e) => {
                eprintln!("Error waiting for signal: {}", e);
            }
        }
    });
    
    // Wait for shutdown signal
    rx.recv().await;
    
    println!("Shutting down...");
    Ok(())
}
```

## Features

- Signal handling for SIGINT, SIGTERM, and SIGHUP
- Async signal listening
- Channel-based signal notification
- Graceful shutdown support
