# shiplog-shutdown

Graceful shutdown utilities for shiplog.

## Description

This crate provides graceful shutdown utilities for the shiplog ecosystem, allowing applications to gracefully shut down components in order with proper coordination.

## Usage

```rust
use shiplog_shutdown::{ShutdownCoordinator, create_shutdown_channel};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (coordinator, mut receiver) = create_shutdown_channel();
    
    // Spawn components that respect shutdown
    tokio::spawn(async move {
        // Do some work
        loop {
            tokio::select! {
                _ = receiver.wait_for_shutdown() => {
                    println!("Shutting down component...");
                    break;
                }
            }
        }
    });
    
    // Trigger graceful shutdown
    coordinator.shutdown();
    
    println!("Shutdown complete");
    Ok(())
}
```

## Features

- Graceful shutdown coordination
- Broadcast-based shutdown signaling
- Shutdown guards for automatic cleanup
- Timeout support for graceful shutdown
- Multiple shutdown reasons (Shutdown, Force, Timeout)
