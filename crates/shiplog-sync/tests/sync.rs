//! Integration tests for shiplog-sync: state transitions, edge cases, proptest.

use chrono::{Duration, Utc};
use proptest::prelude::*;
use shiplog_sync::{SyncConfig, SyncResult, SyncState, SyncStatus};
use std::sync::{Arc, Mutex};
use std::thread;

// ── State machine transitions ─────────────────────────────────────────

#[test]
fn state_machine_full_lifecycle() {
    let mut state = SyncState::new(SyncConfig::default());
    assert_eq!(state.status, SyncStatus::Pending);
    assert!(state.needs_sync());

    state.start();
    assert_eq!(state.status, SyncStatus::InProgress);
    assert!(!state.needs_sync());

    state.complete(50);
    assert_eq!(state.status, SyncStatus::Completed);
    assert_eq!(state.items_synced, 50);
    assert!(state.config.last_sync.is_some());
}

#[test]
fn state_machine_fail_then_retry() {
    let mut state = SyncState::new(SyncConfig::default());
    state.start();
    state.fail("timeout".to_string());

    assert!(matches!(state.status, SyncStatus::Failed(ref msg) if msg == "timeout"));
    assert!(state.needs_sync(), "failed sync should allow retry");

    state.start();
    state.complete(10);
    assert_eq!(state.status, SyncStatus::Completed);
}

// ── needs_sync edge cases ─────────────────────────────────────────────

#[test]
fn needs_sync_completed_recently() {
    let config = SyncConfig {
        source: "test".into(),
        interval_seconds: 3600,
        retry_count: 3,
        last_sync: Some(Utc::now() - Duration::seconds(10)),
    };
    let mut state = SyncState::new(config);
    state.status = SyncStatus::Completed;
    assert!(!state.needs_sync(), "recent sync should not need re-sync");
}

#[test]
fn needs_sync_completed_no_last_sync() {
    let config = SyncConfig {
        source: "test".into(),
        interval_seconds: 3600,
        retry_count: 3,
        last_sync: None,
    };
    let mut state = SyncState::new(config);
    state.status = SyncStatus::Completed;
    assert!(
        state.needs_sync(),
        "completed with no last_sync should need sync"
    );
}

#[test]
fn needs_sync_zero_interval_always_syncs() {
    let config = SyncConfig {
        source: "test".into(),
        interval_seconds: 0,
        retry_count: 1,
        last_sync: Some(Utc::now()),
    };
    let mut state = SyncState::new(config);
    state.status = SyncStatus::Completed;
    assert!(state.needs_sync());
}

// ── SyncResult ────────────────────────────────────────────────────────

#[test]
fn sync_result_success_fields() {
    let r = SyncResult::success(42);
    assert!(r.success);
    assert_eq!(r.items_synced, 42);
    assert!(r.error.is_none());
    // timestamp should be approximately now
    let diff = Utc::now().signed_duration_since(r.timestamp);
    assert!(diff.num_seconds() < 2);
}

#[test]
fn sync_result_failure_fields() {
    let r = SyncResult::failure("boom".into());
    assert!(!r.success);
    assert_eq!(r.items_synced, 0);
    assert_eq!(r.error.as_deref(), Some("boom"));
}

// ── SyncConfig defaults ───────────────────────────────────────────────

#[test]
fn sync_config_defaults() {
    let cfg = SyncConfig::default();
    assert!(cfg.source.is_empty());
    assert_eq!(cfg.interval_seconds, 3600);
    assert_eq!(cfg.retry_count, 3);
    assert!(cfg.last_sync.is_none());
}

// ── Serialization round-trip ──────────────────────────────────────────

#[test]
fn sync_status_serde_roundtrip() {
    let statuses = vec![
        SyncStatus::Pending,
        SyncStatus::InProgress,
        SyncStatus::Completed,
        SyncStatus::Failed("err".into()),
    ];
    for s in statuses {
        let json = serde_json::to_string(&s).unwrap();
        let back: SyncStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}

#[test]
fn sync_config_serde_roundtrip() {
    let cfg = SyncConfig {
        source: "github".into(),
        interval_seconds: 120,
        retry_count: 5,
        last_sync: Some(Utc::now()),
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let back: SyncConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.source, "github");
    assert_eq!(back.interval_seconds, 120);
    assert_eq!(back.retry_count, 5);
}

// ── Thread-safe concurrent mutation via Arc<Mutex<SyncState>> ─────────

#[test]
fn concurrent_state_updates() {
    let state = Arc::new(Mutex::new(SyncState::new(SyncConfig::default())));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let s = Arc::clone(&state);
            thread::spawn(move || {
                let mut locked = s.lock().unwrap();
                locked.start();
                locked.complete(1);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let locked = state.lock().unwrap();
    assert_eq!(locked.status, SyncStatus::Completed);
    assert_eq!(locked.items_synced, 1);
}

// ── Property tests ────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_complete_sets_items(items in 0usize..100_000) {
        let mut state = SyncState::new(SyncConfig::default());
        state.start();
        state.complete(items);
        prop_assert_eq!(state.items_synced, items);
        prop_assert_eq!(state.status, SyncStatus::Completed);
    }

    #[test]
    fn prop_fail_preserves_message(msg in "[a-zA-Z0-9 ]{0,100}") {
        let mut state = SyncState::new(SyncConfig::default());
        state.fail(msg.clone());
        match &state.status {
            SyncStatus::Failed(m) => prop_assert_eq!(m, &msg),
            _ => prop_assert!(false, "should be Failed"),
        }
    }

    #[test]
    fn prop_sync_result_success_items(items in 0usize..100_000) {
        let r = SyncResult::success(items);
        prop_assert!(r.success);
        prop_assert_eq!(r.items_synced, items);
    }
}
