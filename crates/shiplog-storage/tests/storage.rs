use shiplog_storage::{
    FileSystemConfig, InMemoryStorage, Storage, StorageBackend, StorageError, StorageKey,
};

// ── StorageKey ────────────────────────────────────────────────────

#[test]
fn storage_key_from_path() {
    let key = StorageKey::from_path("events/2024");
    assert_eq!(key.0, "events/2024");
}

#[test]
fn storage_key_display() {
    let key = StorageKey::from_path("my/key");
    assert_eq!(format!("{key}"), "my/key");
}

#[test]
fn storage_key_empty() {
    let key = StorageKey::from_path("");
    assert_eq!(key.0, "");
}

#[test]
fn storage_key_special_chars() {
    let key = StorageKey::from_path("path/with spaces/and日本語");
    assert_eq!(key.0, "path/with spaces/and日本語");
}

#[test]
fn storage_key_equality() {
    let a = StorageKey::from_path("same");
    let b = StorageKey::from_path("same");
    assert_eq!(a, b);
}

#[test]
fn storage_key_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(StorageKey::from_path("a"));
    set.insert(StorageKey::from_path("a"));
    assert_eq!(set.len(), 1);
}

#[test]
fn storage_key_clone() {
    let key = StorageKey::from_path("clone-me");
    let cloned = key.clone();
    assert_eq!(key, cloned);
}

// ── StorageBackend ────────────────────────────────────────────────

#[test]
fn storage_backend_display() {
    assert_eq!(format!("{}", StorageBackend::FileSystem), "filesystem");
    assert_eq!(format!("{}", StorageBackend::InMemory), "memory");
    assert_eq!(format!("{}", StorageBackend::Cloud), "cloud");
}

#[test]
fn storage_backend_clone_eq() {
    let a = StorageBackend::FileSystem;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn storage_backend_serde_roundtrip() {
    let backends = [
        StorageBackend::FileSystem,
        StorageBackend::InMemory,
        StorageBackend::Cloud,
    ];
    for b in &backends {
        let json = serde_json::to_string(b).unwrap();
        let deserialized: StorageBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(*b, deserialized);
    }
}

// ── StorageError ──────────────────────────────────────────────────

#[test]
fn storage_error_display() {
    let err = StorageError {
        message: "not found".into(),
        backend: StorageBackend::FileSystem,
    };
    assert_eq!(format!("{err}"), "storage error [filesystem]: not found");
}

#[test]
fn storage_error_is_std_error() {
    let err = StorageError {
        message: "test".into(),
        backend: StorageBackend::InMemory,
    };
    let _: &dyn std::error::Error = &err;
}

// ── InMemoryStorage ───────────────────────────────────────────────

#[test]
fn in_memory_default() {
    let s = InMemoryStorage::default();
    let key = StorageKey::from_path("x");
    assert!(!s.exists(&key).unwrap());
}

#[test]
fn in_memory_set_get() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("k");
    s.set(&key, b"value".to_vec()).unwrap();
    assert_eq!(s.get(&key).unwrap(), Some(b"value".to_vec()));
}

#[test]
fn in_memory_get_nonexistent() {
    let s = InMemoryStorage::new();
    assert_eq!(s.get(&StorageKey::from_path("nope")).unwrap(), None);
}

#[test]
fn in_memory_exists() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("exists");
    assert!(!s.exists(&key).unwrap());
    s.set(&key, vec![1]).unwrap();
    assert!(s.exists(&key).unwrap());
}

#[test]
fn in_memory_delete() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("del");
    s.set(&key, vec![1]).unwrap();
    s.delete(&key).unwrap();
    assert!(!s.exists(&key).unwrap());
    assert_eq!(s.get(&key).unwrap(), None);
}

#[test]
fn in_memory_delete_nonexistent() {
    let mut s = InMemoryStorage::new();
    // Should not error
    s.delete(&StorageKey::from_path("ghost")).unwrap();
}

#[test]
fn in_memory_overwrite() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("over");
    s.set(&key, b"v1".to_vec()).unwrap();
    s.set(&key, b"v2".to_vec()).unwrap();
    assert_eq!(s.get(&key).unwrap(), Some(b"v2".to_vec()));
}

#[test]
fn in_memory_list_prefix() {
    let mut s = InMemoryStorage::new();
    s.set(&StorageKey::from_path("events/1"), vec![]).unwrap();
    s.set(&StorageKey::from_path("events/2"), vec![]).unwrap();
    s.set(&StorageKey::from_path("other/1"), vec![]).unwrap();

    let keys = s.list(&StorageKey::from_path("events/")).unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|k| k.0.starts_with("events/")));
}

#[test]
fn in_memory_list_empty_prefix() {
    let mut s = InMemoryStorage::new();
    s.set(&StorageKey::from_path("a"), vec![]).unwrap();
    s.set(&StorageKey::from_path("b"), vec![]).unwrap();

    let keys = s.list(&StorageKey::from_path("")).unwrap();
    assert_eq!(keys.len(), 2);
}

#[test]
fn in_memory_list_no_matches() {
    let mut s = InMemoryStorage::new();
    s.set(&StorageKey::from_path("a"), vec![]).unwrap();
    let keys = s.list(&StorageKey::from_path("zzz")).unwrap();
    assert!(keys.is_empty());
}

#[test]
fn in_memory_empty_value() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("empty");
    s.set(&key, vec![]).unwrap();
    assert_eq!(s.get(&key).unwrap(), Some(vec![]));
    assert!(s.exists(&key).unwrap());
}

#[test]
fn in_memory_large_value() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("big");
    let data = vec![0xFFu8; 100_000];
    s.set(&key, data.clone()).unwrap();
    assert_eq!(s.get(&key).unwrap(), Some(data));
}

#[test]
fn in_memory_binary_value() {
    let mut s = InMemoryStorage::new();
    let key = StorageKey::from_path("bin");
    let data: Vec<u8> = (0..=255).collect();
    s.set(&key, data.clone()).unwrap();
    assert_eq!(s.get(&key).unwrap(), Some(data));
}

// ── FileSystemConfig ──────────────────────────────────────────────

#[test]
fn fs_config_from_string() {
    let cfg = FileSystemConfig::new("/data".to_string());
    assert_eq!(cfg.root_path, "/data");
}

#[test]
fn fs_config_from_str() {
    let cfg = FileSystemConfig::new("/data");
    assert_eq!(cfg.root_path, "/data");
}

#[test]
fn fs_config_serde_roundtrip() {
    let cfg = FileSystemConfig::new("/some/path");
    let json = serde_json::to_string(&cfg).unwrap();
    let de: FileSystemConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(de.root_path, "/some/path");
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn set_get_roundtrip(key in "[a-z/]{1,50}", value in prop::collection::vec(any::<u8>(), 0..500)) {
            let mut s = InMemoryStorage::new();
            let k = StorageKey::from_path(&key);
            s.set(&k, value.clone()).unwrap();
            prop_assert_eq!(s.get(&k).unwrap(), Some(value));
        }

        #[test]
        fn delete_removes(key in "[a-z/]{1,50}", value in prop::collection::vec(any::<u8>(), 0..100)) {
            let mut s = InMemoryStorage::new();
            let k = StorageKey::from_path(&key);
            s.set(&k, value).unwrap();
            s.delete(&k).unwrap();
            prop_assert_eq!(s.get(&k).unwrap(), None);
            prop_assert!(!s.exists(&k).unwrap());
        }

        #[test]
        fn overwrite_last_wins(key in "[a-z]{1,20}", v1 in prop::collection::vec(any::<u8>(), 0..100), v2 in prop::collection::vec(any::<u8>(), 0..100)) {
            let mut s = InMemoryStorage::new();
            let k = StorageKey::from_path(&key);
            s.set(&k, v1).unwrap();
            s.set(&k, v2.clone()).unwrap();
            prop_assert_eq!(s.get(&k).unwrap(), Some(v2));
        }

        #[test]
        fn storage_key_display_matches_inner(path in "\\PC{0,100}") {
            let key = StorageKey::from_path(&path);
            prop_assert_eq!(format!("{key}"), path);
        }
    }
}
