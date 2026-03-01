use shiplog_version::{Version, VersionError};
use std::cmp::Ordering;

#[test]
fn parse_valid_versions() {
    let v = Version::parse("1.2.3").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));

    let v = Version::parse("0.0.0").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (0, 0, 0));

    let v = Version::parse("999.888.777").unwrap();
    assert_eq!((v.major, v.minor, v.patch), (999, 888, 777));
}

#[test]
fn parse_invalid_format() {
    assert_eq!(
        Version::parse("1.2").unwrap_err(),
        VersionError::InvalidFormat
    );
    assert_eq!(
        Version::parse("1").unwrap_err(),
        VersionError::InvalidFormat
    );
    assert_eq!(
        Version::parse("1.2.3.4").unwrap_err(),
        VersionError::InvalidFormat
    );
    assert_eq!(Version::parse("").unwrap_err(), VersionError::InvalidFormat);
}

#[test]
fn parse_invalid_number() {
    assert_eq!(
        Version::parse("1.2.x").unwrap_err(),
        VersionError::InvalidNumber
    );
    assert_eq!(
        Version::parse("a.b.c").unwrap_err(),
        VersionError::InvalidNumber
    );
}

#[test]
fn compare_versions() {
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(1, 0, 1);
    let v3 = Version::new(1, 1, 0);
    let v4 = Version::new(2, 0, 0);

    assert_eq!(v1.compare(&v1), Ordering::Equal);
    assert_eq!(v1.compare(&v2), Ordering::Less);
    assert_eq!(v1.compare(&v3), Ordering::Less);
    assert_eq!(v4.compare(&v1), Ordering::Greater);
}

#[test]
fn ordering_sorts_correctly() {
    let mut versions = [
        Version::new(2, 0, 0),
        Version::new(1, 0, 0),
        Version::new(1, 2, 3),
        Version::new(1, 2, 0),
    ];
    versions.sort();
    assert_eq!(versions[0], Version::new(1, 0, 0));
    assert_eq!(versions[3], Version::new(2, 0, 0));
}

#[test]
fn is_compatible_same_major() {
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(1, 9, 9);
    let v3 = Version::new(2, 0, 0);

    assert!(v1.is_compatible(&v2));
    assert!(!v1.is_compatible(&v3));
}

#[test]
fn display_format() {
    assert_eq!(Version::new(1, 2, 3).to_string(), "1.2.3");
    assert_eq!(Version::new(0, 0, 0).to_string(), "0.0.0");
}

#[test]
fn version_error_display() {
    assert!(VersionError::InvalidFormat.to_string().contains("format"));
    assert!(VersionError::InvalidNumber.to_string().contains("number"));
}
