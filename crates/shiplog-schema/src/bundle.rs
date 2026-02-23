use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::RunId;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChecksum {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// Which redaction profile a bundle was built for.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BundleProfile {
    #[default]
    Internal,
    Manager,
    Public,
}

impl BundleProfile {
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleManifest {
    pub run_id: RunId,
    pub generated_at: DateTime<Utc>,
    #[serde(default)]
    pub profile: BundleProfile,
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
}
