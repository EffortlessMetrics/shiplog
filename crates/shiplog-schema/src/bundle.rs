use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::RunId;
use std::fmt;
use std::str::FromStr;

/// SHA-256 checksum and size for a single file in the bundle.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChecksum {
    /// Relative path within the run directory.
    pub path: String,
    /// Hex-encoded SHA-256 digest.
    pub sha256: String,
    /// File size in bytes.
    pub bytes: u64,
}

/// Which redaction profile a bundle was built for.
///
/// # Examples
///
/// ```
/// use shiplog_schema::bundle::BundleProfile;
///
/// let p: BundleProfile = "manager".parse().unwrap();
/// assert_eq!(p, BundleProfile::Manager);
/// assert_eq!(p.as_str(), "manager");
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BundleProfile {
    /// Full-fidelity internal packet (default).
    #[default]
    Internal,
    /// Manager-facing packet with selective redaction.
    Manager,
    /// Fully redacted public packet.
    Public,
}

impl BundleProfile {
    /// Returns the canonical lowercase name of this profile.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::bundle::BundleProfile;
    ///
    /// assert_eq!(BundleProfile::Internal.as_str(), "internal");
    /// assert_eq!(BundleProfile::Manager.as_str(), "manager");
    /// assert_eq!(BundleProfile::Public.as_str(), "public");
    /// ```
    pub fn as_str(&self) -> &str {
        match self {
            Self::Internal => "internal",
            Self::Manager => "manager",
            Self::Public => "public",
        }
    }
}

impl fmt::Display for BundleProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BundleProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "internal" => Ok(Self::Internal),
            "manager" => Ok(Self::Manager),
            "public" => Ok(Self::Public),
            other => Err(format!(
                "unknown bundle profile: {other:?} (expected internal|manager|public)"
            )),
        }
    }
}

/// Manifest listing every file in a bundle together with checksums.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleManifest {
    /// Run that produced this bundle.
    pub run_id: RunId,
    /// When the bundle was created.
    pub generated_at: DateTime<Utc>,
    /// Redaction profile used for this bundle.
    #[serde(default)]
    pub profile: BundleProfile,
    /// Checksums for each file in the bundle.
    pub files: Vec<FileChecksum>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_profile_from_str_round_trip() {
        for (s, expected) in [
            ("internal", BundleProfile::Internal),
            ("manager", BundleProfile::Manager),
            ("public", BundleProfile::Public),
            ("Internal", BundleProfile::Internal),
            ("MANAGER", BundleProfile::Manager),
        ] {
            let parsed: BundleProfile = s.parse().unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn bundle_profile_from_str_unknown() {
        let res: Result<BundleProfile, _> = "bogus".parse();
        assert!(res.is_err());
    }

    #[test]
    fn bundle_profile_default_is_internal() {
        assert_eq!(BundleProfile::default(), BundleProfile::Internal);
    }

    #[test]
    fn bundle_manifest_missing_profile_defaults_to_internal() {
        let json = r#"{
            "run_id": "test-run",
            "generated_at": "2025-01-01T00:00:00Z",
            "files": []
        }"#;
        let manifest: BundleManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.profile, BundleProfile::Internal);
    }

    #[test]
    fn bundle_profile_as_str_returns_expected_values() {
        assert_eq!(BundleProfile::Internal.as_str(), "internal");
        assert_eq!(BundleProfile::Manager.as_str(), "manager");
        assert_eq!(BundleProfile::Public.as_str(), "public");
    }

    #[test]
    fn bundle_profile_display_matches_as_str() {
        for profile in [
            BundleProfile::Internal,
            BundleProfile::Manager,
            BundleProfile::Public,
        ] {
            assert_eq!(profile.to_string(), profile.as_str());
        }
    }

    #[test]
    fn file_checksum_serde_roundtrip() {
        let fc = FileChecksum {
            path: "packet.md".into(),
            sha256: "abcdef1234567890".into(),
            bytes: 4096,
        };
        let json = serde_json::to_string(&fc).unwrap();
        let back: FileChecksum = serde_json::from_str(&json).unwrap();
        assert_eq!(fc, back);
    }

    #[test]
    fn bundle_manifest_serde_roundtrip() {
        let manifest = BundleManifest {
            run_id: RunId("run-123".into()),
            generated_at: chrono::Utc::now(),
            profile: BundleProfile::Manager,
            files: vec![
                FileChecksum {
                    path: "packet.md".into(),
                    sha256: "abc".into(),
                    bytes: 100,
                },
                FileChecksum {
                    path: "ledger.events.jsonl".into(),
                    sha256: "def".into(),
                    bytes: 200,
                },
            ],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: BundleManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }
}
