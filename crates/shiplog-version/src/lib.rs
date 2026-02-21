//! Version parsing and comparison utilities for shiplog.
//!
//! This crate provides version parsing and comparison utilities for the shiplog ecosystem.

use std::cmp::Ordering;

/// Represents a version with major, minor, and patch components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Creates a new Version from major, minor, and patch components.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parses a version string in the format "major.minor.patch".
    pub fn parse(version: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = version.split('.').collect();
        
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat);
        }

        let major = parts[0]
            .parse()
            .map_err(|_| VersionError::InvalidNumber)?;
        let minor = parts[1]
            .parse()
            .map_err(|_| VersionError::InvalidNumber)?;
        let patch = parts[2]
            .parse()
            .map_err(|_| VersionError::InvalidNumber)?;

        Ok(Self { major, minor, patch })
    }

    /// Compares this version with another.
    /// Returns Ordering::Less, Ordering::Equal, or Ordering::Greater.
    pub fn compare(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }

    /// Checks if this version is compatible with another (same major version).
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }

    /// Returns the version as a string.
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.compare(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Error type for version parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum VersionError {
    InvalidFormat,
    InvalidNumber,
}

impl std::fmt::Display for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionError::InvalidFormat => write!(f, "Invalid version format: expected major.minor.patch"),
            VersionError::InvalidNumber => write!(f, "Invalid version number"),
        }
    }
}

impl std::error::Error for VersionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_version() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = Version::parse("1.2");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VersionError::InvalidFormat);
    }

    #[test]
    fn test_parse_invalid_number() {
        let result = Version::parse("1.2.x");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VersionError::InvalidNumber);
    }

    #[test]
    fn test_compare_versions() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.4").unwrap();
        let v3 = Version::parse("2.0.0").unwrap();

        assert_eq!(v1.compare(&v2), Ordering::Less);
        assert_eq!(v1.compare(&v1), Ordering::Equal);
        assert_eq!(v1.compare(&v3), Ordering::Less);
        assert_eq!(v3.compare(&v1), Ordering::Greater);
    }

    #[test]
    fn test_is_compatible() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.5.0").unwrap();
        let v3 = Version::parse("2.0.0").unwrap();

        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));
    }

    #[test]
    fn test_to_string() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_display() {
        let version = Version::new(1, 2, 3);
        assert_eq!(format!("{}", version), "1.2.3");
    }

    #[test]
    fn test_ord_trait() {
        let mut versions = vec![
            Version::new(2, 0, 0),
            Version::new(1, 0, 0),
            Version::new(1, 2, 3),
            Version::new(1, 2, 0),
        ];
        versions.sort();
        // Versions should be sorted: 1.0.0 < 1.2.0 < 1.2.3 < 2.0.0 (lexicographic ordering)
        assert_eq!(
            versions,
            vec![
                Version::new(1, 0, 0),
                Version::new(1, 2, 0),
                Version::new(1, 2, 3),
                Version::new(2, 0, 0),
            ]
        );
    }
}
