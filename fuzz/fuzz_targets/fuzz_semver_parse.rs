//! Fuzz harness for semantic version parsing (shiplog-semver)
//!
//! Exercises `SemVer::parse` and `VersionRange::parse` with arbitrary strings.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_semver::{SemVer, VersionRange};

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Fuzz version parsing
    if let Ok(ver) = SemVer::parse(input) {
        let _ = ver.is_prerelease();
        let _ = ver.compare(&SemVer::new(0, 0, 0));
    }

    // Fuzz range parsing and matching
    if let Ok(range) = VersionRange::parse(input) {
        let _ = range.matches(&SemVer::new(1, 0, 0));
    }
});
