//! Fuzz harness for coverage manifest deserialization.
//!
//! Exercises serde round-tripping of CoverageManifest from arbitrary JSON,
//! ensuring no panics on malformed input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::coverage::CoverageManifest;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Attempt to deserialize — we don't care if it fails,
    // we just want to ensure it doesn't panic
    if let Ok(manifest) = serde_json::from_str::<CoverageManifest>(input) {
        // If parsing succeeds, verify round-trip doesn't panic
        let _ = serde_json::to_string(&manifest);
    }
});
