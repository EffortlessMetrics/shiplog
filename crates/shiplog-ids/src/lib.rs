#![warn(missing_docs)]
//! Stable identifier types used across the shiplog pipeline.
//!
//! Includes deterministic SHA-256 constructors for event/workstream IDs and a
//! timestamp-based run ID helper.

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
///
/// # Examples
///
/// ```
/// use shiplog_ids::EventId;
///
/// let id = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
/// assert_eq!(id.0.len(), 64); // SHA-256 hex string
///
/// // Same inputs always produce the same ID:
/// let id2 = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
/// assert_eq!(id, id2);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A deterministic workstream identifier.
///
/// # Examples
///
/// ```
/// use shiplog_ids::WorkstreamId;
///
/// let id = WorkstreamId::from_parts(["repo", "acme/widgets"]);
/// assert_eq!(id.0.len(), 64);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkstreamId(pub String);

impl fmt::Display for WorkstreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A timestamp-based run identifier.
///
/// # Examples
///
/// ```
/// use shiplog_ids::RunId;
///
/// let id = RunId::now("shiplog");
/// assert!(id.0.starts_with("shiplog_"));
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_ids::EventId;
    ///
    /// let id = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
    /// // Different parts produce different IDs:
    /// let other = EventId::from_parts(["github", "pr", "owner/repo", "99"]);
    /// assert_ne!(id, other);
    /// ```
    pub fn from_parts(parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self(hash_hex(parts))
    }
}

impl WorkstreamId {
    /// Deterministic workstream id from stable parts.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_ids::WorkstreamId;
    ///
    /// let a = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    /// let b = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    /// assert_eq!(a, b);
    /// ```
    pub fn from_parts(parts: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self(hash_hex(parts))
    }
}

impl RunId {
    /// Non-deterministic enough to avoid collisions without dragging in UUID/rand.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_ids::RunId;
    ///
    /// let run = RunId::now("shiplog");
    /// // Each call generates a unique ID:
    /// let run2 = RunId::now("shiplog");
    /// assert_ne!(run, run2);
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_deterministic() {
        let a = EventId::from_parts(["github", "pr", "o/r", "42"]);
        let b = EventId::from_parts(["github", "pr", "o/r", "42"]);
        assert_eq!(a, b);
    }

    #[test]
    fn event_id_varies_with_parts() {
        let a = EventId::from_parts(["github", "pr", "o/r", "1"]);
        let b = EventId::from_parts(["github", "pr", "o/r", "2"]);
        assert_ne!(a, b);
    }

    #[test]
    fn event_id_is_valid_sha256_hex() {
        let id = EventId::from_parts(["x"]);
        assert_eq!(id.0.len(), 64);
        assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn workstream_id_deterministic() {
        let a = WorkstreamId::from_parts(["repo", "acme/foo"]);
        let b = WorkstreamId::from_parts(["repo", "acme/foo"]);
        assert_eq!(a, b);
    }

    #[test]
    fn part_boundary_matters() {
        let a = EventId::from_parts(["a", "bc"]);
        let b = EventId::from_parts(["ab", "c"]);
        assert_ne!(
            a, b,
            "newline separator should prevent part-boundary collisions"
        );
    }

    #[test]
    fn run_id_starts_with_prefix() {
        let id = RunId::now("shiplog");
        assert!(id.0.starts_with("shiplog_"));
    }

    #[test]
    fn display_matches_inner() {
        let id = EventId::from_parts(["display", "test"]);
        assert_eq!(format!("{id}"), id.0);
    }

    #[test]
    fn single_part_matches_known_sha256() {
        // SHA-256("abc") — no leading newline
        let id = EventId::from_parts(["abc"]);
        assert_eq!(
            id.0,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn multi_part_uses_newline_separator_not_prefix() {
        // SHA-256("a\nb") — newline between parts, not before first
        let id = EventId::from_parts(["a", "b"]);
        let expected = {
            let mut h = sha2::Sha256::new();
            h.update(b"a\nb");
            hex::encode(h.finalize())
        };
        assert_eq!(id.0, expected);

        // Verify it does NOT equal SHA-256("\na\nb") which the mutant would produce
        let wrong = {
            let mut h = sha2::Sha256::new();
            h.update(b"\na\nb");
            hex::encode(h.finalize())
        };
        assert_ne!(
            id.0, wrong,
            "hash_hex must not prepend a newline before the first part"
        );
    }
}
