//! Semantic version utilities for shiplog.
//!
//! This crate provides semantic version utilities following the SemVer 2.0.0 specification
//! for the shiplog ecosystem.

use std::cmp::Ordering;

/// Represents a semantic version per SemVer 2.0.0 specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Vec<String>,
    pub build: Vec<String>,
}

impl SemVer {
    /// Creates a new SemVer from major, minor, and patch components.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: Vec::new(),
            build: Vec::new(),
        }
    }

    /// Creates a new SemVer with prerelease identifiers.
    pub fn with_prerelease(major: u32, minor: u32, patch: u32, pre: &[&str]) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: pre.iter().map(|s| s.to_string()).collect(),
            build: Vec::new(),
        }
    }

    /// Parses a semantic version string.
    pub fn parse(version: &str) -> Result<Self, SemVerError> {
        // Split on '+' first to separate version from build metadata
        let parts: Vec<&str> = version.split('+').collect();
        if parts.len() > 2 {
            return Err(SemVerError::InvalidFormat);
        }

        let (version_part, build) = if parts.len() == 2 {
            (parts[0], Some(parts[1]))
        } else {
            (version, None)
        };

        // Split on '-' to separate pre-release
        let version_parts: Vec<&str> = version_part.split('-').collect();
        if version_parts.len() > 2 {
            return Err(SemVerError::InvalidFormat);
        }

        let (core_part, pre) = if version_parts.len() == 2 {
            (version_parts[0], Some(version_parts[1]))
        } else {
            (version_parts[0], None)
        };

        // Parse core version (major.minor.patch)
        let core_parts: Vec<&str> = core_part.split('.').collect();
        if core_parts.len() != 3 {
            return Err(SemVerError::InvalidFormat);
        }

        let major = core_parts[0]
            .parse()
            .map_err(|_| SemVerError::InvalidNumber)?;
        let minor = core_parts[1]
            .parse()
            .map_err(|_| SemVerError::InvalidNumber)?;
        let patch = core_parts[2]
            .parse()
            .map_err(|_| SemVerError::InvalidNumber)?;

        // Parse prerelease identifiers
        let pre_ids = if let Some(pre) = pre {
            if pre.is_empty() {
                return Err(SemVerError::InvalidFormat);
            }
            pre.split('.').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        // Parse build metadata identifiers
        let build_ids = if let Some(build) = build {
            if build.is_empty() {
                return Err(SemVerError::InvalidFormat);
            }
            build.split('.').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            major,
            minor,
            patch,
            pre: pre_ids,
            build: build_ids,
        })
    }

    /// Compares this semver with another.
    /// Precedence is determined as per SemVer 2.0.0 specification.
    pub fn compare(&self, other: &Self) -> Ordering {
        // Compare major, minor, patch
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(self.compare_pre(&other.pre))
    }

    /// Compares prerelease versions.
    fn compare_pre(&self, other: &[String]) -> Ordering {
        // If both have no prerelease, they're equal
        if self.pre.is_empty() && other.is_empty() {
            return Ordering::Equal;
        }

        // If one has prerelease and other doesn't, the one without is greater
        if self.pre.is_empty() {
            return Ordering::Greater;
        }
        if other.is_empty() {
            return Ordering::Less;
        }

        // Compare prerelease identifiers
        let max_len = self.pre.len().max(other.len());
        for i in 0..max_len {
            let a = self.pre.get(i);
            let b = other.get(i);

            match (a, b) {
                (Some(a), Some(b)) => {
                    let result = compare_pre_id(a, b);
                    if result != Ordering::Equal {
                        return result;
                    }
                }
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => break,
            }
        }

        Ordering::Equal
    }

    /// Returns true if this is a prerelease version.
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Checks if this version satisfies a range.
    pub fn satisfies(&self, range: &VersionRange) -> bool {
        range.matches(self)
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        if !self.build.is_empty() {
            write!(f, "+{}", self.build.join("."))?;
        }
        Ok(())
    }
}

/// Compare two prerelease identifiers per SemVer spec.
fn compare_pre_id(a: &str, b: &str) -> Ordering {
    let a_is_num = a.chars().all(|c| c.is_ascii_digit());
    let b_is_num = b.chars().all(|c| c.is_ascii_digit());

    match (a_is_num, b_is_num) {
        (true, true) => {
            let a_num: u64 = a.parse().unwrap_or(0);
            let b_num: u64 = b.parse().unwrap_or(0);
            a_num.cmp(&b_num)
        }
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => a.cmp(b),
    }
}

/// Represents a version range for matching.
#[derive(Debug, Clone)]
pub struct VersionRange {
    pub comparator: Comparator,
}

impl VersionRange {
    /// Creates a new VersionRange from a comparator string like ">=1.0.0".
    pub fn parse(range: &str) -> Result<Self, SemVerError> {
        let comparator = Comparator::parse(range)?;
        Ok(Self { comparator })
    }

    /// Checks if a version matches this range.
    pub fn matches(&self, version: &SemVer) -> bool {
        self.comparator.matches(version)
    }
}

/// Comparator for version matching.
#[derive(Debug, Clone)]
pub enum Comparator {
    Exact(SemVer),
    Greater(SemVer),
    GreaterEq(SemVer),
    Less(SemVer),
    LessEq(SemVer),
    Range {
        min: Option<SemVer>,
        max: Option<SemVer>,
    },
}

impl Comparator {
    pub fn parse(comp: &str) -> Result<Self, SemVerError> {
        if let Some(rest) = comp.strip_prefix(">=") {
            let version = SemVer::parse(rest)?;
            Ok(Comparator::GreaterEq(version))
        } else if let Some(rest) = comp.strip_prefix('>') {
            let version = SemVer::parse(rest)?;
            Ok(Comparator::Greater(version))
        } else if let Some(rest) = comp.strip_prefix("<=") {
            let version = SemVer::parse(rest)?;
            Ok(Comparator::LessEq(version))
        } else if let Some(rest) = comp.strip_prefix('<') {
            let version = SemVer::parse(rest)?;
            Ok(Comparator::Less(version))
        } else if let Some(rest) = comp.strip_prefix('=') {
            let version = SemVer::parse(rest)?;
            Ok(Comparator::Exact(version))
        } else {
            let version = SemVer::parse(comp)?;
            Ok(Comparator::Exact(version))
        }
    }

    pub fn matches(&self, version: &SemVer) -> bool {
        match self {
            Comparator::Exact(v) => version == v,
            Comparator::Greater(v) => version > v,
            Comparator::GreaterEq(v) => version >= v,
            Comparator::Less(v) => version < v,
            Comparator::LessEq(v) => version <= v,
            Comparator::Range { min, max } => {
                let above_min = min.as_ref().is_none_or(|m| version >= m);
                let below_max = max.as_ref().is_none_or(|m| version < m);
                above_min && below_max
            }
        }
    }
}

/// Error type for semantic version parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum SemVerError {
    InvalidFormat,
    InvalidNumber,
}

impl std::fmt::Display for SemVerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemVerError::InvalidFormat => write!(f, "Invalid semantic version format"),
            SemVerError::InvalidNumber => write!(f, "Invalid version number"),
        }
    }
}

impl std::error::Error for SemVerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_semver() {
        let version = SemVer::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.pre.is_empty());
    }

    #[test]
    fn test_parse_with_prerelease() {
        let version = SemVer::parse("1.0.0-alpha.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.pre, vec!["alpha", "1"]);
    }

    #[test]
    fn test_parse_with_build() {
        let version = SemVer::parse("1.0.0+build.123").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.build, vec!["build", "123"]);
    }

    #[test]
    fn test_parse_full_semver() {
        let version = SemVer::parse("1.0.0-alpha.1+build.123").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.pre, vec!["alpha", "1"]);
        assert_eq!(version.build, vec!["build", "123"]);
    }

    #[test]
    fn test_compare_prerelease() {
        let v1 = SemVer::parse("1.0.0-alpha").unwrap();
        let v2 = SemVer::parse("1.0.0-beta").unwrap();
        let v3 = SemVer::parse("1.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_compare_numeric_prerelease() {
        let v1 = SemVer::parse("1.0.0-alpha.1").unwrap();
        let v2 = SemVer::parse("1.0.0-alpha.2").unwrap();
        let v3 = SemVer::parse("1.0.0-alpha.10").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_is_prerelease() {
        let v1 = SemVer::parse("1.0.0-alpha").unwrap();
        let v2 = SemVer::parse("1.0.0").unwrap();

        assert!(v1.is_prerelease());
        assert!(!v2.is_prerelease());
    }

    #[test]
    fn test_to_string() {
        let version = SemVer::parse("1.2.3-alpha.1+build.123").unwrap();
        assert_eq!(version.to_string(), "1.2.3-alpha.1+build.123");
    }

    #[test]
    fn test_comparator_greater() {
        let comp = Comparator::parse(">1.0.0").unwrap();
        let v1 = SemVer::parse("1.0.1").unwrap();
        let v2 = SemVer::parse("1.0.0").unwrap();

        assert!(comp.matches(&v1));
        assert!(!comp.matches(&v2));
    }

    #[test]
    fn test_comparator_greater_eq() {
        let comp = Comparator::parse(">=1.0.0").unwrap();
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("1.0.1").unwrap();

        assert!(comp.matches(&v1));
        assert!(comp.matches(&v2));
    }

    #[test]
    fn test_comparator_less() {
        let comp = Comparator::parse("<2.0.0").unwrap();
        let v1 = SemVer::parse("1.9.9").unwrap();
        let v2 = SemVer::parse("2.0.0").unwrap();

        assert!(comp.matches(&v1));
        assert!(!comp.matches(&v2));
    }

    #[test]
    fn test_comparator_exact() {
        let comp = Comparator::parse("=1.2.3").unwrap();
        let v1 = SemVer::parse("1.2.3").unwrap();
        let v2 = SemVer::parse("1.2.4").unwrap();

        assert!(comp.matches(&v1));
        assert!(!comp.matches(&v2));
    }

    #[test]
    fn test_version_range() {
        // Use a single comparator for simplicity
        let range = VersionRange::parse(">=1.0.0").unwrap();
        let v1 = SemVer::parse("1.5.0").unwrap();
        let v2 = SemVer::parse("1.0.0").unwrap();
        let v3 = SemVer::parse("0.9.0").unwrap();

        assert!(range.matches(&v1));
        assert!(range.matches(&v2));
        assert!(!range.matches(&v3));
    }

    #[test]
    fn test_satisfies() {
        let version = SemVer::parse("1.2.3").unwrap();
        let range = VersionRange::parse(">=1.0.0").unwrap();

        assert!(version.satisfies(&range));
    }
}
