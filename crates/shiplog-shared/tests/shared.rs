//! Integration tests for shiplog-shared: concurrent readers/writers, SharedStateMap.

use shiplog_shared::{SharedState, SharedStateConfig, SharedStateMap};
use std::sync::Arc;
use tokio::sync::Barrier;

// ── SharedState: concurrent reads ─────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_state_concurrent_reads() {
    let state = SharedState::new(42i32);
    let barrier = Arc::new(Barrier::new(10));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let s = state.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            let guard = s.read().await;
            assert_eq!(*guard, 42);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

// ── SharedState: writer excludes readers ──────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_state_write_then_read() {
    let state = SharedState::new(0u64);

    state
        .update(|v| {
            *v = 999;
        })
        .await;

    let val = state.get().await;
    assert_eq!(val, 999);
}

// ── SharedState: concurrent increments ────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_state_concurrent_increments() {
    let state = SharedState::new(0u64);
    let barrier = Arc::new(Barrier::new(10));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let s = state.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for _ in 0..100 {
                s.update(|v| *v += 1).await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(state.get().await, 1000);
}

// ── SharedState: clone shares same data ───────────────────────────────

#[tokio::test]
async fn shared_state_clone_shares_data() {
    let state = SharedState::new(1i32);
    let clone = state.clone();

    assert!(!state.is_unique());

    clone.update(|v| *v = 42).await;
    assert_eq!(state.get().await, 42);
}

// ── SharedState: is_unique ────────────────────────────────────────────

#[test]
fn shared_state_unique_when_single() {
    let state = SharedState::new("hello");
    assert!(state.is_unique());
}

#[test]
fn shared_state_not_unique_when_cloned() {
    let state = SharedState::new("hello");
    let _clone = state.clone();
    assert!(!state.is_unique());
}

// ── SharedStateConfig ─────────────────────────────────────────────────

#[test]
fn shared_state_config_defaults() {
    let cfg = SharedStateConfig::default();
    assert_eq!(cfg.initial_value, None);
    assert!(!cfg.persistent);
    assert_eq!(cfg.persist_path, None);
}

// ── SharedStateMap: basic operations ──────────────────────────────────

#[tokio::test]
async fn state_map_insert_get_remove() {
    let map = SharedStateMap::new();
    assert!(map.is_empty().await);

    map.insert("k1".into(), "v1".into()).await;
    assert_eq!(map.len().await, 1);
    assert!(map.contains_key("k1").await);
    assert_eq!(map.get("k1").await, Some("v1".into()));

    let removed = map.remove("k1").await;
    assert_eq!(removed, Some("v1".into()));
    assert!(map.is_empty().await);
}

#[tokio::test]
async fn state_map_keys() {
    let map = SharedStateMap::new();
    map.insert("a".into(), "1".into()).await;
    map.insert("b".into(), "2".into()).await;
    map.insert("c".into(), "3".into()).await;

    let mut keys = map.keys().await;
    keys.sort();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn state_map_get_nonexistent_returns_none() {
    let map = SharedStateMap::new();
    assert_eq!(map.get("missing").await, None);
}

#[tokio::test]
async fn state_map_remove_nonexistent_returns_none() {
    let map = SharedStateMap::new();
    assert_eq!(map.remove("missing").await, None);
}

// ── SharedStateMap: concurrent inserts ────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn state_map_concurrent_inserts() {
    let map = SharedStateMap::new();
    let barrier = Arc::new(Barrier::new(10));

    let mut handles = Vec::new();
    for i in 0..10u32 {
        let m = map.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for j in 0..10u32 {
                let key = format!("{i}-{j}");
                m.insert(key.clone(), format!("val-{key}")).await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(map.len().await, 100);
}

// ── SharedStateMap: concurrent reads and writes ───────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn state_map_concurrent_read_write() {
    let map = SharedStateMap::new();
    map.insert("shared".into(), "0".into()).await;

    let barrier = Arc::new(Barrier::new(6));

    // 3 writers
    let mut handles = Vec::new();
    for i in 0..3 {
        let m = map.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for j in 0..20 {
                m.insert("shared".into(), format!("{i}-{j}")).await;
            }
        }));
    }

    // 3 readers
    for _ in 0..3 {
        let m = map.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for _ in 0..20 {
                let _ = m.get("shared").await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // "shared" key should still exist with some value
    assert!(map.contains_key("shared").await);
}

// ── SharedState: update with complex closure ──────────────────────────

#[tokio::test]
async fn shared_state_update_complex() {
    let state = SharedState::new(Vec::<i32>::new());

    for i in 0..5 {
        state.update(|v| v.push(i)).await;
    }

    let vec = state.get().await;
    assert_eq!(vec, vec![0, 1, 2, 3, 4]);
}

// ── SharedStateMap: clone shares underlying data ──────────────────────

#[tokio::test]
async fn state_map_clone_shares_data() {
    let map = SharedStateMap::new();
    let clone = map.clone();

    map.insert("key".into(), "value".into()).await;
    assert_eq!(clone.get("key").await, Some("value".into()));
}

// ── SharedState: write guard dereference ──────────────────────────────

#[tokio::test]
async fn shared_state_write_guard_deref_mut() {
    let state = SharedState::new(vec![1, 2, 3]);
    {
        let mut guard = state.write().await;
        guard.push(4);
    }
    let val = state.get().await;
    assert_eq!(val, vec![1, 2, 3, 4]);
}

// ── SharedState: read guard dereference ───────────────────────────────

#[tokio::test]
async fn shared_state_read_guard_deref() {
    let state = SharedState::new(String::from("hello"));
    let guard = state.read().await;
    assert_eq!(guard.len(), 5);
    assert_eq!(&*guard, "hello");
}

// ── SharedStateMap: overwrite existing key ────────────────────────────

#[tokio::test]
async fn state_map_overwrite_existing_key() {
    let map = SharedStateMap::new();
    map.insert("key".into(), "v1".into()).await;
    map.insert("key".into(), "v2".into()).await;
    assert_eq!(map.get("key").await, Some("v2".into()));
    assert_eq!(map.len().await, 1);
}

// ── SharedStateMap: len after removes ─────────────────────────────────

#[tokio::test]
async fn state_map_len_decreases_after_remove() {
    let map = SharedStateMap::new();
    map.insert("a".into(), "1".into()).await;
    map.insert("b".into(), "2".into()).await;
    map.insert("c".into(), "3".into()).await;
    assert_eq!(map.len().await, 3);
    map.remove("b").await;
    assert_eq!(map.len().await, 2);
    assert!(!map.contains_key("b").await);
}

// ── SharedStateConfig: custom values ──────────────────────────────────

#[test]
fn shared_state_config_custom_values() {
    let cfg = SharedStateConfig {
        initial_value: Some("init".to_string()),
        persistent: true,
        persist_path: Some("/tmp/state.json".to_string()),
    };
    assert_eq!(cfg.initial_value, Some("init".to_string()));
    assert!(cfg.persistent);
    assert_eq!(cfg.persist_path, Some("/tmp/state.json".to_string()));
}

// ── SharedStateConfig: serde roundtrip ────────────────────────────────

#[test]
fn shared_state_config_serde_roundtrip() {
    let cfg = SharedStateConfig {
        initial_value: Some("test".to_string()),
        persistent: true,
        persist_path: Some("/path".to_string()),
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let deser: SharedStateConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.initial_value, cfg.initial_value);
    assert_eq!(deser.persistent, cfg.persistent);
    assert_eq!(deser.persist_path, cfg.persist_path);
}

// ── SharedState: concurrent writers with update ───────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_state_concurrent_vec_pushes() {
    let state = SharedState::new(Vec::<u32>::new());
    let barrier = Arc::new(Barrier::new(5));

    let mut handles = Vec::new();
    for i in 0..5u32 {
        let s = state.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            for j in 0..10u32 {
                s.update(|v| v.push(i * 10 + j)).await;
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let vec = state.get().await;
    assert_eq!(vec.len(), 50);
}

// ── SharedStateMap: default is empty ──────────────────────────────────

#[tokio::test]
async fn state_map_default_is_empty() {
    let map = SharedStateMap::default();
    assert!(map.is_empty().await);
    assert_eq!(map.len().await, 0);
}
