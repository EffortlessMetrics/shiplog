use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Stable identifiers used across the shiplog pipeline.
///
/// The rule is simple:
/// - IDs are deterministic when derived from source data.
/// - IDs are printable and safe to paste into docs.
///
/// This makes downstream redaction and diffing tractable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkstreamId(pub String);

impl fmt::Display for WorkstreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub String);

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl EventId {
    /// Deterministic event id from a small set of stable parts.
    ///
    /// You want this to survive:
    /// - re-runs
    /// - different machines
    /// - different render profiles
    pub fn from_parts(parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self(hash_hex(parts))
    }
}

impl WorkstreamId {
    pub fn from_parts(parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self(hash_hex(parts))
    }
}

impl RunId {
    /// Non-deterministic enough to avoid collisions without dragging in UUID/rand.
    pub fn now(prefix: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        RunId(format!("{prefix}_{nanos}"))
    }
}

fn hash_hex(parts: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut hasher = Sha256::new();
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            hasher.update(b"\n");
        }
        hasher.update(p.as_ref().as_bytes());
    }
    let out = hasher.finalize();
    hex::encode(out)
}
