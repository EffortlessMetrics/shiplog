use shiplog_shutdown::*;
use std::time::Duration;

#[test]
fn coordinator_creation() {
    let coord = ShutdownCoordinator::new();
    assert!(!coord.is_shutdown_initiated());
}

#[test]
fn coordinator_with_capacity() {
    let coord = ShutdownCoordinator::with_capacity(10);
    assert!(!coord.is_shutdown_initiated());
}

#[test]
fn coordinator_shutdown_sets_flag() {
    let coord = ShutdownCoordinator::new();
    coord.shutdown();
    assert!(coord.is_shutdown_initiated());
}

#[test]
fn coordinator_force_shutdown() {
    let coord = ShutdownCoordinator::new();
    coord.force_shutdown();
    assert!(coord.is_shutdown_initiated());
}

#[test]
fn create_channel() {
    let (coord, mut recv) = create_shutdown_channel();
    coord.shutdown();
    assert_eq!(recv.try_recv(), Some(ShutdownReason::Shutdown));
}

#[test]
fn shutdown_reason_equality() {
    assert_eq!(ShutdownReason::Shutdown, ShutdownReason::Shutdown);
    assert_ne!(ShutdownReason::Shutdown, ShutdownReason::Force);
    assert_ne!(ShutdownReason::Force, ShutdownReason::Timeout);
}

#[test]
fn shutdown_guard_triggers_on_drop() {
    let coord = ShutdownCoordinator::new();
    {
        let _guard = ShutdownGuard::new(coord.clone());
    }
    assert!(coord.is_shutdown_initiated());
}

#[test]
fn shutdown_guard_manual_trigger() {
    let coord = ShutdownCoordinator::new();
    let guard = ShutdownGuard::new(coord.clone());
    guard.trigger();
    assert!(coord.is_shutdown_initiated());
}

#[test]
fn coordinator_clone() {
    let c1 = ShutdownCoordinator::new();
    let c2 = c1.clone();
    c1.shutdown();
    assert!(c2.is_shutdown_initiated());
}

#[test]
fn coordinator_default() {
    let coord = ShutdownCoordinator::default();
    assert!(!coord.is_shutdown_initiated());
}

#[tokio::test]
async fn graceful_shutdown_success() {
    let result = graceful_shutdown(async { Ok::<_, &str>(42) }, Duration::from_secs(1)).await;
    assert_eq!(result, Ok(42));
}

#[tokio::test]
async fn graceful_shutdown_timeout() {
    let result = graceful_shutdown(
        async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok::<_, &str>(42)
        },
        Duration::from_millis(50),
    )
    .await;
    assert_eq!(result, Err(ShutdownReason::Timeout));
}

#[tokio::test]
async fn receiver_wait_for_shutdown() {
    let (coord, mut recv) = create_shutdown_channel();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        coord.shutdown();
    });
    let reason = recv.wait_for_shutdown().await;
    assert_eq!(reason, ShutdownReason::Shutdown);
}

#[tokio::test]
async fn receiver_wait_timeout() {
    let (_coord, mut recv) = create_shutdown_channel();
    let reason = recv
        .wait_for_shutdown_timeout(Duration::from_millis(50))
        .await;
    assert_eq!(reason, ShutdownReason::Timeout);
}
