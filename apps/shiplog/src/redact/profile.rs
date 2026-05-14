//! Redaction profile parsing for shiplog projections.

use core::convert::Infallible;
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// Rendering profiles.
///
/// The tool produces multiple projections from the same ledger.
/// Think of them as lenses, not forks.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RedactionProfile {
    Internal,
    Manager,
    #[default]
    Public,
}

impl RedactionProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedactionProfile::Internal => "internal",
            RedactionProfile::Manager => "manager",
            RedactionProfile::Public => "public",
        }
    }

    /// Parse profile string, defaulting unknown values to `Public`.
    pub fn from_profile_str(profile: &str) -> Self {
        match profile {
            "internal" => RedactionProfile::Internal,
            "manager" => RedactionProfile::Manager,
            _ => RedactionProfile::Public,
        }
    }
}

impl fmt::Display for RedactionProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RedactionProfile {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_profile_str(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_returns_expected_values() {
        assert_eq!(RedactionProfile::Internal.as_str(), "internal");
        assert_eq!(RedactionProfile::Manager.as_str(), "manager");
        assert_eq!(RedactionProfile::Public.as_str(), "public");
    }

    #[test]
    fn parser_handles_known_values() {
        assert_eq!(
            RedactionProfile::from_profile_str("internal"),
            RedactionProfile::Internal
        );
        assert_eq!(
            RedactionProfile::from_profile_str("manager"),
            RedactionProfile::Manager
        );
        assert_eq!(
            RedactionProfile::from_profile_str("public"),
            RedactionProfile::Public
        );
    }

    #[test]
    fn parser_defaults_unknown_to_public() {
        assert_eq!(
            RedactionProfile::from_profile_str("unexpected"),
            RedactionProfile::Public
        );
        assert_eq!(
            RedactionProfile::from_profile_str(""),
            RedactionProfile::Public
        );
    }

    #[test]
    fn default_is_public() {
        assert_eq!(RedactionProfile::default(), RedactionProfile::Public);
    }

    #[test]
    fn from_str_and_display_are_canonical() {
        let parsed: RedactionProfile = "manager".parse().expect("infallible parser");
        assert_eq!(parsed, RedactionProfile::Manager);
        assert_eq!(parsed.to_string(), "manager");
    }
}
