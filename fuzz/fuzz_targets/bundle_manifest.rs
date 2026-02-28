//! Fuzz harness for bundle manifest deserialization and BundleProfile parsing.
//!
//! Exercises serde round-tripping of BundleManifest from arbitrary JSON
//! and BundleProfile::from_str with arbitrary strings.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::bundle::{BundleManifest, BundleProfile};
use std::str::FromStr;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Fuzz BundleProfile::from_str — must not panic on any input
    let _ = BundleProfile::from_str(input);

    // Fuzz BundleManifest deserialization
    if let Ok(manifest) = serde_json::from_str::<BundleManifest>(input) {
        // If parsing succeeds, verify round-trip doesn't panic
        let _ = serde_json::to_string(&manifest);
    }
});
