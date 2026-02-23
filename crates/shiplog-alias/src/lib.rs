//! Deterministic alias generation and persistence helpers.
//!
//! This crate provides a narrow contract:
//! - keyed deterministic alias generation
//! - optional cache load/save for stable aliases across runs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct AliasCache {
    version: u32,
    entries: BTreeMap<String, String>,
}

/// Canonical alias cache filename emitted in run directories.
pub const CACHE_FILENAME: &str = "redaction.aliases.json";

/// Thread-safe deterministic alias store backed by keyed SHA-256.
#[derive(Debug)]
pub struct DeterministicAliasStore {
    key: Vec<u8>,
    cache: Mutex<BTreeMap<String, String>>,
}

impl DeterministicAliasStore {
    /// Create an empty alias store with a stable key.
    pub fn new(key: impl AsRef<[u8]>) -> Self {
        Self {
            key: key.as_ref().to_vec(),
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// Path to the alias cache file in a run output directory.
    pub fn cache_path(out_dir: &Path) -> PathBuf {
        out_dir.join(CACHE_FILENAME)
    }

    /// Load aliases from disk. Missing files are treated as a no-op.
    pub fn load_cache(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read alias cache from {path:?}"))?;
        let cache: AliasCache =
            serde_json::from_str(&text).with_context(|| format!("parse alias cache {path:?}"))?;

        if cache.version != 1 {
            anyhow::bail!("unsupported alias cache version: {}", cache.version);
        }

        if let Ok(mut current) = self.cache.lock() {
            for (k, v) in cache.entries {
                current.entry(k).or_insert(v);
            }
        }

        Ok(())
    }

    /// Save aliases to disk.
    pub fn save_cache(&self, path: &Path) -> Result<()> {
        let entries = self
            .cache
            .lock()
            .map_err(|e| anyhow::anyhow!("lock alias cache: {e}"))?
            .clone();

        let cache = AliasCache {
            version: 1,
            entries,
        };
        let json = serde_json::to_string_pretty(&cache)?;
        std::fs::write(path, json).with_context(|| format!("write alias cache to {path:?}"))?;
        Ok(())
    }

    /// Resolve a stable alias for a (`kind`, `value`) pair.
    pub fn alias(&self, kind: &str, value: &str) -> String {
        let cache_key = format!("{kind}:{value}");
        #[allow(clippy::collapsible_if)]
        if let Ok(cache) = self.cache.lock() {
            if let Some(v) = cache.get(&cache_key) {
                return v.clone();
            }
        }

        let mut hasher = Sha256::new();
        hasher.update(&self.key);
        hasher.update(b"\n");
        hasher.update(kind.as_bytes());
        hasher.update(b"\n");
        hasher.update(value.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let alias = format!("{kind}-{}", &hash[..12]);

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(cache_key, alias.clone());
        }

        alias
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_are_stable_for_same_key_and_value() {
        let aliases = DeterministicAliasStore::new(b"test-key");
        let a1 = aliases.alias("repo", "acme/service");
        let a2 = aliases.alias("repo", "acme/service");
        assert_eq!(a1, a2);
    }

    #[test]
    fn aliases_differ_for_different_keys_when_uncached() {
        let a = DeterministicAliasStore::new(b"key-a");
        let b = DeterministicAliasStore::new(b"key-b");
        assert_ne!(
            a.alias("repo", "acme/service"),
            b.alias("repo", "acme/service")
        );
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join(CACHE_FILENAME);

        let first = DeterministicAliasStore::new(b"key-a");
        let repo_alias = first.alias("repo", "acme/foo");
        let ws_alias = first.alias("ws", "my-workstream");
        first.save_cache(&cache_path).expect("save cache");

        let second = DeterministicAliasStore::new(b"key-a");
        second.load_cache(&cache_path).expect("load cache");
        assert_eq!(second.alias("repo", "acme/foo"), repo_alias);
        assert_eq!(second.alias("ws", "my-workstream"), ws_alias);
    }

    #[test]
    fn missing_file_is_noop() {
        let aliases = DeterministicAliasStore::new(b"key");
        let result = aliases.load_cache(Path::new("/nonexistent/path/cache.json"));
        assert!(result.is_ok());
    }

    #[test]
    fn corrupt_file_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join(CACHE_FILENAME);
        std::fs::write(&cache_path, "this is not json!!!").expect("write");

        let aliases = DeterministicAliasStore::new(b"key");
        let result = aliases.load_cache(&cache_path);
        assert!(result.is_err());
    }

    #[test]
    fn version_mismatch_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join(CACHE_FILENAME);
        let bad = serde_json::json!({ "version": 99, "entries": {} });
        std::fs::write(
            &cache_path,
            serde_json::to_string(&bad).expect("serialize bad"),
        )
        .expect("write");

        let aliases = DeterministicAliasStore::new(b"key");
        let result = aliases.load_cache(&cache_path);
        assert!(result.is_err());
        let msg = format!("{}", result.expect_err("expected version mismatch"));
        assert!(msg.contains("unsupported alias cache version"));
    }

    #[test]
    fn cache_path_joins_out_dir_with_filename() {
        let path = DeterministicAliasStore::cache_path(Path::new("/some/out"));
        assert!(
            path.ends_with(CACHE_FILENAME),
            "expected path to end with cache filename, got: {path:?}"
        );
    }

    #[test]
    fn cache_preserves_across_key_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join(CACHE_FILENAME);

        let first = DeterministicAliasStore::new(b"key-A");
        let alias_a = first.alias("repo", "acme/foo");
        first.save_cache(&cache_path).expect("save cache");

        let second = DeterministicAliasStore::new(b"key-B");
        second.load_cache(&cache_path).expect("load cache");
        let alias_b = second.alias("repo", "acme/foo");
        assert_eq!(
            alias_a, alias_b,
            "cached alias should be preserved, not regenerated with new key"
        );

        let fresh_b = second.alias("repo", "acme/bar");
        let third = DeterministicAliasStore::new(b"key-A");
        let fresh_a = third.alias("repo", "acme/bar");
        assert_ne!(
            fresh_b, fresh_a,
            "uncached alias should use current key, not old key"
        );
    }
}
