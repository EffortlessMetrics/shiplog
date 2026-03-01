//! Integration tests for shiplog-semver.

use proptest::prelude::*;
use shiplog_semver::*;
use std::cmp::Ordering;

// ── SemVer 2.0.0 spec compliance ───────────────────────────────

#[test]
fn spec_parse_major_minor_patch() {
    let v = SemVer::parse("1.2.3").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));
    assert!(v.pre.is_empty());
    assert!(v.build.is_empty());
}

#[test]
fn spec_prerelease_identifiers() {
    let v = SemVer::parse("1.0.0-alpha.1").unwrap();
    assert_eq!(v.pre, vec!["alpha", "1"]);
}

#[test]
fn spec_build_metadata() {
    let v = SemVer::parse("1.0.0+build.123").unwrap();
    assert_eq!(v.build, vec!["build", "123"]);
}

#[test]
fn spec_full_version() {
    let v = SemVer::parse("1.0.0-alpha.1+build.123").unwrap();
    assert_eq!(v.pre, vec!["alpha", "1"]);
    assert_eq!(v.build, vec!["build", "123"]);
}

#[test]
fn spec_zero_version() {
    let v = SemVer::parse("0.0.0").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (0, 0, 0));
}

#[test]
fn spec_large_numbers() {
    let v = SemVer::parse("999.999.999").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (999, 999, 999));
}

// ── Precedence rules (SemVer 2.0.0 §11) ────────────────────────

#[test]
fn precedence_major_minor_patch() {
    assert!(SemVer::parse("1.0.0").unwrap() < SemVer::parse("2.0.0").unwrap());
    assert!(SemVer::parse("2.0.0").unwrap() < SemVer::parse("2.1.0").unwrap());
    assert!(SemVer::parse("2.1.0").unwrap() < SemVer::parse("2.1.1").unwrap());
}

#[test]
fn precedence_prerelease_lower_than_release() {
    let pre = SemVer::parse("1.0.0-alpha").unwrap();
    let release = SemVer::parse("1.0.0").unwrap();
    assert!(pre < release);
}

#[test]
fn precedence_prerelease_alpha_order() {
    assert!(SemVer::parse("1.0.0-alpha").unwrap() < SemVer::parse("1.0.0-beta").unwrap());
}

#[test]
fn precedence_prerelease_numeric_order() {
    assert!(SemVer::parse("1.0.0-alpha.1").unwrap() < SemVer::parse("1.0.0-alpha.2").unwrap());
    assert!(SemVer::parse("1.0.0-alpha.2").unwrap() < SemVer::parse("1.0.0-alpha.10").unwrap());
}

#[test]
fn precedence_numeric_before_alpha() {
    // Numeric identifiers have lower precedence than alphanumeric
    let numeric = SemVer::with_prerelease(1, 0, 0, &["1"]);
    let alpha = SemVer::with_prerelease(1, 0, 0, &["alpha"]);
    assert!(numeric < alpha);
}

#[test]
fn precedence_longer_set_greater() {
    let short = SemVer::parse("1.0.0-alpha").unwrap();
    let long = SemVer::parse("1.0.0-alpha.1").unwrap();
    assert!(short < long);
}

#[test]
fn precedence_spec_example_order() {
    // SemVer spec §11 example ordering
    let versions: Vec<SemVer> = vec![
        SemVer::parse("1.0.0-alpha").unwrap(),
        SemVer::parse("1.0.0-alpha.1").unwrap(),
        SemVer::parse("1.0.0-beta").unwrap(),
        SemVer::parse("1.0.0-beta.2").unwrap(),
        SemVer::parse("1.0.0-beta.11").unwrap(),
        SemVer::parse("1.0.0-rc.1").unwrap(),
        SemVer::parse("1.0.0").unwrap(),
    ];

    for i in 0..versions.len() - 1 {
        assert!(
            versions[i] < versions[i + 1],
            "{} should be < {}",
            versions[i],
            versions[i + 1]
        );
    }
}

// ── Display / parse round-trip ──────────────────────────────────

#[test]
fn display_roundtrip_simple() {
    let v = SemVer::new(1, 2, 3);
    assert_eq!(v.to_string(), "1.2.3");
    assert_eq!(SemVer::parse(&v.to_string()).unwrap(), v);
}

#[test]
fn display_roundtrip_prerelease() {
    let v = SemVer::parse("1.0.0-alpha.1").unwrap();
    assert_eq!(v.to_string(), "1.0.0-alpha.1");
    assert_eq!(SemVer::parse(&v.to_string()).unwrap(), v);
}

#[test]
fn display_roundtrip_full() {
    let v = SemVer::parse("1.0.0-alpha.1+build.123").unwrap();
    assert_eq!(v.to_string(), "1.0.0-alpha.1+build.123");
}

// ── Error cases ─────────────────────────────────────────────────

#[test]
fn parse_errors() {
    assert_eq!(SemVer::parse("").unwrap_err(), SemVerError::InvalidFormat);
    assert_eq!(SemVer::parse("1").unwrap_err(), SemVerError::InvalidFormat);
    assert_eq!(
        SemVer::parse("1.2").unwrap_err(),
        SemVerError::InvalidFormat
    );
    assert_eq!(
        SemVer::parse("a.b.c").unwrap_err(),
        SemVerError::InvalidNumber
    );
    assert_eq!(
        SemVer::parse("1.2.3-").unwrap_err(),
        SemVerError::InvalidFormat
    );
    assert_eq!(
        SemVer::parse("1.2.3+").unwrap_err(),
        SemVerError::InvalidFormat
    );
}

#[test]
fn error_display() {
    assert_eq!(
        SemVerError::InvalidFormat.to_string(),
        "Invalid semantic version format"
    );
    assert_eq!(
        SemVerError::InvalidNumber.to_string(),
        "Invalid version number"
    );
}

// ── is_prerelease ───────────────────────────────────────────────

#[test]
fn is_prerelease_tests() {
    assert!(SemVer::parse("1.0.0-alpha").unwrap().is_prerelease());
    assert!(SemVer::parse("1.0.0-0.1").unwrap().is_prerelease());
    assert!(!SemVer::parse("1.0.0").unwrap().is_prerelease());
    assert!(!SemVer::parse("1.0.0+build").unwrap().is_prerelease());
}

// ── VersionRange / Comparator tests ─────────────────────────────

#[test]
fn comparator_exact() {
    let comp = Comparator::parse("=1.2.3").unwrap();
    assert!(comp.matches(&SemVer::new(1, 2, 3)));
    assert!(!comp.matches(&SemVer::new(1, 2, 4)));
}

#[test]
fn comparator_implicit_exact() {
    let comp = Comparator::parse("1.2.3").unwrap();
    assert!(comp.matches(&SemVer::new(1, 2, 3)));
    assert!(!comp.matches(&SemVer::new(1, 2, 4)));
}

#[test]
fn comparator_greater() {
    let comp = Comparator::parse(">1.0.0").unwrap();
    assert!(comp.matches(&SemVer::new(1, 0, 1)));
    assert!(comp.matches(&SemVer::new(2, 0, 0)));
    assert!(!comp.matches(&SemVer::new(1, 0, 0)));
    assert!(!comp.matches(&SemVer::new(0, 9, 9)));
}

#[test]
fn comparator_greater_eq() {
    let comp = Comparator::parse(">=1.0.0").unwrap();
    assert!(comp.matches(&SemVer::new(1, 0, 0)));
    assert!(comp.matches(&SemVer::new(1, 0, 1)));
    assert!(!comp.matches(&SemVer::new(0, 9, 9)));
}

#[test]
fn comparator_less() {
    let comp = Comparator::parse("<2.0.0").unwrap();
    assert!(comp.matches(&SemVer::new(1, 9, 9)));
    assert!(!comp.matches(&SemVer::new(2, 0, 0)));
}

#[test]
fn comparator_less_eq() {
    let comp = Comparator::parse("<=2.0.0").unwrap();
    assert!(comp.matches(&SemVer::new(2, 0, 0)));
    assert!(comp.matches(&SemVer::new(1, 9, 9)));
    assert!(!comp.matches(&SemVer::new(2, 0, 1)));
}

#[test]
fn version_range_satisfies() {
    let v = SemVer::new(1, 5, 0);
    let range = VersionRange::parse(">=1.0.0").unwrap();
    assert!(v.satisfies(&range));
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_version_ordering() {
    let mut versions = [
        SemVer::parse("2.0.0").unwrap(),
        SemVer::parse("1.0.0-alpha").unwrap(),
        SemVer::parse("0.1.0").unwrap(),
        SemVer::parse("1.0.0").unwrap(),
        SemVer::parse("1.0.0-beta").unwrap(),
        SemVer::parse("0.0.1").unwrap(),
    ];
    versions.sort();
    let formatted: Vec<String> = versions.iter().map(|v| v.to_string()).collect();
    insta::assert_snapshot!(formatted.join("\n"), @r"
    0.0.1
    0.1.0
    1.0.0-alpha
    1.0.0-beta
    1.0.0
    2.0.0
    ");
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_parse_display_roundtrip(
        major in 0u32..1000,
        minor in 0u32..1000,
        patch in 0u32..1000,
    ) {
        let v = SemVer::new(major, minor, patch);
        let s = v.to_string();
        let parsed = SemVer::parse(&s).unwrap();
        prop_assert_eq!(v, parsed);
    }

    #[test]
    fn prop_ordering_reflexive(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100,
    ) {
        let v = SemVer::new(major, minor, patch);
        prop_assert_eq!(v.compare(&v), Ordering::Equal);
    }

    #[test]
    fn prop_ordering_antisymmetric(
        a_major in 0u32..10, a_minor in 0u32..10, a_patch in 0u32..10,
        b_major in 0u32..10, b_minor in 0u32..10, b_patch in 0u32..10,
    ) {
        let a = SemVer::new(a_major, a_minor, a_patch);
        let b = SemVer::new(b_major, b_minor, b_patch);
        let ab = a.compare(&b);
        let ba = b.compare(&a);
        match ab {
            Ordering::Less => prop_assert_eq!(ba, Ordering::Greater),
            Ordering::Greater => prop_assert_eq!(ba, Ordering::Less),
            Ordering::Equal => prop_assert_eq!(ba, Ordering::Equal),
        }
    }

    #[test]
    fn prop_display_contains_dots(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100,
    ) {
        let v = SemVer::new(major, minor, patch);
        let s = v.to_string();
        prop_assert_eq!(s.matches('.').count(), 2);
    }

    #[test]
    fn prop_new_not_prerelease(
        major in 0u32..100,
        minor in 0u32..100,
        patch in 0u32..100,
    ) {
        let v = SemVer::new(major, minor, patch);
        prop_assert!(!v.is_prerelease());
    }
}
