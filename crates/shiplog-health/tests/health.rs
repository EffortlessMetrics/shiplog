use proptest::prelude::*;
use shiplog_health::{
    HealthCheckResult, HealthMonitor, HealthRegistry, HealthStatus, ReadinessCheck,
};
use std::time::Duration;

// ── Property tests (sync) ───────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_health_status_display_round_trip(status in prop_oneof![
        Just(HealthStatus::Healthy),
        Just(HealthStatus::Degraded),
        Just(HealthStatus::Unhealthy),
    ]) {
        let displayed = format!("{}", status);
        match status {
            HealthStatus::Healthy => prop_assert_eq!(displayed, "healthy"),
            HealthStatus::Degraded => prop_assert_eq!(displayed, "degraded"),
            HealthStatus::Unhealthy => prop_assert_eq!(displayed, "unhealthy"),
        }
    }

    #[test]
    fn prop_health_check_result_preserves_name(name in "[a-z]{1,50}") {
        let result = HealthCheckResult::healthy(&name);
        prop_assert_eq!(result.name, name);
        prop_assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[test]
    fn prop_degraded_always_has_message(
        name in "[a-z]{1,50}",
        msg in "[a-z ]{1,100}",
    ) {
        let result = HealthCheckResult::degraded(&name, &msg);
        prop_assert_eq!(result.status, HealthStatus::Degraded);
        prop_assert!(result.message.is_some());
        prop_assert_eq!(result.message.unwrap(), msg);
    }

    #[test]
    fn prop_unhealthy_always_has_message(
        name in "[a-z]{1,50}",
        msg in "[a-z ]{1,100}",
    ) {
        let result = HealthCheckResult::unhealthy(&name, &msg);
        prop_assert_eq!(result.status, HealthStatus::Unhealthy);
        prop_assert!(result.message.is_some());
        prop_assert_eq!(result.message.unwrap(), msg);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_health_status_display() {
    assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
    assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
    assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
}

#[test]
fn known_answer_health_check_result() {
    let r = HealthCheckResult::new("db", HealthStatus::Healthy);
    assert_eq!(r.name, "db");
    assert_eq!(r.status, HealthStatus::Healthy);
    assert!(r.message.is_none());
    assert!(r.timestamp > 0);
}

#[test]
fn known_answer_degraded_result() {
    let r = HealthCheckResult::degraded("cache", "high latency");
    assert_eq!(r.name, "cache");
    assert_eq!(r.status, HealthStatus::Degraded);
    assert_eq!(r.message.as_deref(), Some("high latency"));
}

#[test]
fn known_answer_unhealthy_result() {
    let r = HealthCheckResult::unhealthy("db", "connection refused");
    assert_eq!(r.name, "db");
    assert_eq!(r.status, HealthStatus::Unhealthy);
    assert_eq!(r.message.as_deref(), Some("connection refused"));
}

// ── Edge cases (sync) ───────────────────────────────────────────────────────

#[test]
fn edge_health_status_default() {
    let status = HealthStatus::default();
    assert_eq!(status, HealthStatus::Healthy);
}

#[test]
fn edge_health_status_equality() {
    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
}

#[test]
fn edge_health_check_result_clone() {
    let r = HealthCheckResult::degraded("test", "msg");
    let r2 = r.clone();
    assert_eq!(r.name, r2.name);
    assert_eq!(r.status, r2.status);
    assert_eq!(r.message, r2.message);
}

// ── Async tests: HealthRegistry ─────────────────────────────────────────────

#[tokio::test]
async fn async_registry_empty() {
    let reg = HealthRegistry::new();
    assert!(reg.is_empty().await);
    assert_eq!(reg.len().await, 0);
    assert_eq!(reg.overall_status().await, HealthStatus::Healthy);
}

#[tokio::test]
async fn async_registry_register_and_check() {
    let reg = HealthRegistry::new();
    reg.register_simple("db", || HealthCheckResult::healthy("db"))
        .await;
    reg.register_simple("cache", || HealthCheckResult::healthy("cache"))
        .await;

    assert_eq!(reg.len().await, 2);
    let results = reg.check_all().await;
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.status == HealthStatus::Healthy));
}

#[tokio::test]
async fn async_registry_overall_all_healthy() {
    let reg = HealthRegistry::new();
    reg.register_simple("a", || HealthCheckResult::healthy("a"))
        .await;
    reg.register_simple("b", || HealthCheckResult::healthy("b"))
        .await;
    assert_eq!(reg.overall_status().await, HealthStatus::Healthy);
}

#[tokio::test]
async fn async_registry_overall_degraded_when_one_degraded() {
    let reg = HealthRegistry::new();
    reg.register_simple("a", || HealthCheckResult::healthy("a"))
        .await;
    reg.register_simple("b", || HealthCheckResult::degraded("b", "slow"))
        .await;
    assert_eq!(reg.overall_status().await, HealthStatus::Degraded);
}

#[tokio::test]
async fn async_registry_overall_unhealthy_overrides_degraded() {
    let reg = HealthRegistry::new();
    reg.register_simple("a", || HealthCheckResult::healthy("a"))
        .await;
    reg.register_simple("b", || HealthCheckResult::degraded("b", "slow"))
        .await;
    reg.register_simple("c", || HealthCheckResult::unhealthy("c", "down"))
        .await;
    assert_eq!(reg.overall_status().await, HealthStatus::Unhealthy);
}

#[tokio::test]
async fn async_registry_register_boxed() {
    let reg = HealthRegistry::new();
    let check: Box<dyn Fn() -> HealthCheckResult + Send + Sync> =
        Box::new(|| HealthCheckResult::healthy("boxed"));
    reg.register("boxed", check).await;
    assert_eq!(reg.len().await, 1);
    let results = reg.check_all().await;
    assert_eq!(results[0].name, "boxed");
}

// ── Async tests: HealthMonitor ──────────────────────────────────────────────

#[tokio::test]
async fn async_monitor_initial_status() {
    let monitor = HealthMonitor::new(Duration::from_secs(30));
    assert_eq!(monitor.get_status().await, HealthStatus::Healthy);
    assert!(monitor.time_since_last_check().await.is_none());
}

#[tokio::test]
async fn async_monitor_set_and_get() {
    let monitor = HealthMonitor::new(Duration::from_secs(10));
    monitor.set_status(HealthStatus::Degraded).await;
    assert_eq!(monitor.get_status().await, HealthStatus::Degraded);
    assert!(monitor.time_since_last_check().await.is_some());
}

#[tokio::test]
async fn async_monitor_transitions() {
    let monitor = HealthMonitor::new(Duration::from_secs(5));
    monitor.set_status(HealthStatus::Healthy).await;
    assert_eq!(monitor.get_status().await, HealthStatus::Healthy);

    monitor.set_status(HealthStatus::Degraded).await;
    assert_eq!(monitor.get_status().await, HealthStatus::Degraded);

    monitor.set_status(HealthStatus::Unhealthy).await;
    assert_eq!(monitor.get_status().await, HealthStatus::Unhealthy);

    monitor.set_status(HealthStatus::Healthy).await;
    assert_eq!(monitor.get_status().await, HealthStatus::Healthy);
}

#[tokio::test]
async fn async_monitor_check_interval() {
    let monitor = HealthMonitor::new(Duration::from_secs(60));
    assert_eq!(monitor.check_interval(), Duration::from_secs(60));
}

#[tokio::test]
async fn async_monitor_default() {
    let monitor = HealthMonitor::default();
    assert_eq!(monitor.check_interval(), Duration::from_secs(30));
    assert_eq!(monitor.get_status().await, HealthStatus::Healthy);
}

// ── Async tests: ReadinessCheck ─────────────────────────────────────────────

#[tokio::test]
async fn async_readiness_initially_not_ready() {
    let check = ReadinessCheck::new();
    assert!(!check.is_ready().await);
}

#[tokio::test]
async fn async_readiness_set_ready() {
    let check = ReadinessCheck::new();
    check.set_ready().await;
    assert!(check.is_ready().await);
}

#[tokio::test]
async fn async_readiness_toggle() {
    let check = ReadinessCheck::new();
    check.set_ready().await;
    assert!(check.is_ready().await);
    check.set_not_ready().await;
    assert!(!check.is_ready().await);
    check.set_ready().await;
    assert!(check.is_ready().await);
}

#[tokio::test]
async fn async_readiness_default() {
    let check = ReadinessCheck::default();
    assert!(!check.is_ready().await);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn health_status_serde_round_trip() {
    let json = serde_json::to_string(&HealthStatus::Healthy).unwrap();
    assert_eq!(json, "\"healthy\"");
    let s: HealthStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(s, HealthStatus::Healthy);

    let json_d = serde_json::to_string(&HealthStatus::Degraded).unwrap();
    assert_eq!(json_d, "\"degraded\"");

    let json_u = serde_json::to_string(&HealthStatus::Unhealthy).unwrap();
    assert_eq!(json_u, "\"unhealthy\"");
}

#[test]
fn health_check_result_serde_round_trip() {
    let result = HealthCheckResult::degraded("db", "slow queries");
    let json = serde_json::to_string(&result).unwrap();
    let r2: HealthCheckResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r2.name, "db");
    assert_eq!(r2.status, HealthStatus::Degraded);
    assert_eq!(r2.message.as_deref(), Some("slow queries"));
}
