# shiplog-health

Health check utilities for shiplog.

## Description

This crate provides health check utilities for the shiplog ecosystem, allowing applications to expose health status and readiness probes for Kubernetes and other orchestration systems.

## Usage

```rust
use shiplog_health::{HealthRegistry, HealthCheckResult, ReadinessCheck};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = HealthRegistry::new();
    
    // Register health checks
    registry.register_simple("database", || {
        HealthCheckResult::healthy("database")
    }).await;
    
    // Check all health status
    let results = registry.check_all().await;
    let overall = registry.overall_status().await;
    
    println!("Overall status: {}", overall);
    
    // Use readiness check for Kubernetes
    let readiness = ReadinessCheck::new();
    readiness.set_ready().await;
    
    if readiness.is_ready().await {
        println!("Service is ready!");
    }
    
    Ok(())
}
```

## Features

- Health status tracking (Healthy, Degraded, Unhealthy)
- Health check registry with multiple checks
- Readiness probes for Kubernetes
- Health monitor with time tracking
- Serde serialization for API responses
- Simple closure-based health checks
