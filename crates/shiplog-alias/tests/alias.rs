//! Integration tests for shiplog-alias.

use shiplog_alias::*;
use std::path::Path;
use tempfile::TempDir;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_alias_format() {
    let store = DeterministicAliasStore::new(b"stable-test-key");
    let alias = store.alias("repo", "acme/service");
    insta::assert_snapshot!("alias_repo_format", alias);
}

#[test]
fn snapshot_alias_different_kinds() {
    let store = DeterministicAliasStore::new(b"stable-test-key");
    let results = vec![
        ("repo", store.alias("repo", "acme/api")),
        ("user", store.alias("user", "octocat")),
        ("ws", store.alias("ws", "infrastructure")),
        ("pr", store.alias("pr", "42")),
    ];
    insta::assert_yaml_snapshot!("alias_different_kinds", results);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn alias_is_deterministic(
            kind in "[a-z]{1,10}",
            value in "[a-zA-Z0-9/.-]{1,50}"
        ) {
            let store = DeterministicAliasStore::new(b"test-key");
            let a1 = store.alias(&kind, &value);
            let a2 = store.alias(&kind, &value);
            prop_assert_eq!(a1, a2);
        }

        #[test]
        fn alias_starts_with_kind(
            kind in "[a-z]{1,10}",
            value in "[a-z]{1,20}"
        ) {
            let store = DeterministicAliasStore::new(b"key");
            let alias = store.alias(&kind, &value);
            let prefix = format!("{}-", kind);
            prop_assert!(alias.starts_with(&prefix));
        }

        #[test]
        fn different_values_produce_different_aliases(
            kind in "[a-z]{1,5}",
            v1 in "[a-z]{1,20}",
            v2 in "[a-z]{1,20}"
        ) {
            prop_assume!(v1 != v2);
            let store = DeterministicAliasStore::new(b"key");
            let a1 = store.alias(&kind, &v1);
            let a2 = store.alias(&kind, &v2);
            prop_assert_ne!(a1, a2);
        }

        #[test]
        fn different_keys_produce_different_aliases(value in "[a-z]{1,20}") {
            let s1 = DeterministicAliasStore::new(b"key-1");
            let s2 = DeterministicAliasStore::new(b"key-2");
            prop_assert_ne!(
                s1.alias("repo", &value),
                s2.alias("repo", &value)
            );
        }

        #[test]
        fn alias_has_fixed_length_hash_suffix(
            kind in "[a-z]{1,10}",
            value in "[a-z]{1,50}"
        ) {
            let store = DeterministicAliasStore::new(b"key");
            let alias = store.alias(&kind, &value);
            // Format: kind-<12 hex chars>
            let suffix = alias.strip_prefix(&format!("{kind}-")).unwrap();
            prop_assert_eq!(suffix.len(), 12);
            prop_assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_key_produces_alias() {
    let store = DeterministicAliasStore::new(b"");
    let alias = store.alias("repo", "test");
    assert!(alias.starts_with("repo-"));
}

#[test]
fn empty_value_produces_alias() {
    let store = DeterministicAliasStore::new(b"key");
    let alias = store.alias("repo", "");
    assert!(alias.starts_with("repo-"));
}

#[test]
fn empty_kind_produces_alias() {
    let store = DeterministicAliasStore::new(b"key");
    let alias = store.alias("", "value");
    assert!(alias.starts_with("-"));
}

#[test]
fn unicode_value_produces_alias() {
    let store = DeterministicAliasStore::new(b"key");
    let alias = store.alias("user", "用户名");
    assert!(alias.starts_with("user-"));
}

#[test]
fn cache_path_constant() {
    assert_eq!(CACHE_FILENAME, "redaction.aliases.json");
}

#[test]
fn cache_path_joins_correctly() {
    let path = DeterministicAliasStore::cache_path(Path::new("/run/out"));
    assert!(path.ends_with("redaction.aliases.json"));
}

#[test]
fn load_missing_cache_is_noop() {
    let store = DeterministicAliasStore::new(b"key");
    assert!(
        store
            .load_cache(Path::new("/nonexistent/path.json"))
            .is_ok()
    );
}

#[test]
fn load_corrupt_cache_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);
    std::fs::write(&path, "not json").unwrap();

    let store = DeterministicAliasStore::new(b"key");
    assert!(store.load_cache(&path).is_err());
}

#[test]
fn load_wrong_version_errors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);
    let bad = serde_json::json!({"version": 99, "entries": {}});
    std::fs::write(&path, serde_json::to_string(&bad).unwrap()).unwrap();

    let store = DeterministicAliasStore::new(b"key");
    let err = store.load_cache(&path).unwrap_err();
    assert!(err.to_string().contains("unsupported alias cache version"));
}

#[test]
fn save_and_load_preserves_aliases() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);

    let store1 = DeterministicAliasStore::new(b"key");
    let a1 = store1.alias("repo", "acme/api");
    let a2 = store1.alias("user", "octocat");
    store1.save_cache(&path).unwrap();

    let store2 = DeterministicAliasStore::new(b"key");
    store2.load_cache(&path).unwrap();
    assert_eq!(store2.alias("repo", "acme/api"), a1);
    assert_eq!(store2.alias("user", "octocat"), a2);
}

#[test]
fn cached_alias_survives_key_change() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);

    let store1 = DeterministicAliasStore::new(b"key-A");
    let original = store1.alias("repo", "acme/api");
    store1.save_cache(&path).unwrap();

    // Load with a different key
    let store2 = DeterministicAliasStore::new(b"key-B");
    store2.load_cache(&path).unwrap();
    // Cached entry is preserved
    assert_eq!(store2.alias("repo", "acme/api"), original);
}

#[test]
fn uncached_alias_uses_current_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);

    let store1 = DeterministicAliasStore::new(b"key-A");
    store1.alias("repo", "acme/api");
    store1.save_cache(&path).unwrap();

    let store2 = DeterministicAliasStore::new(b"key-B");
    store2.load_cache(&path).unwrap();
    // New alias not in cache uses new key
    let new_alias = store2.alias("repo", "acme/new-repo");
    let store3 = DeterministicAliasStore::new(b"key-A");
    let other_alias = store3.alias("repo", "acme/new-repo");
    assert_ne!(new_alias, other_alias);
}

#[test]
fn many_aliases_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(CACHE_FILENAME);

    let store = DeterministicAliasStore::new(b"key");
    let mut expected = Vec::new();
    for i in 0..100 {
        let alias = store.alias("item", &format!("value-{i}"));
        expected.push((format!("value-{i}"), alias));
    }
    store.save_cache(&path).unwrap();

    let store2 = DeterministicAliasStore::new(b"key");
    store2.load_cache(&path).unwrap();
    for (value, alias) in &expected {
        assert_eq!(&store2.alias("item", value), alias);
    }
}
