//! Fuzz harness for Manual Events YAML (manual_events.yaml)
//!
//! This harness tests the robustness of the manual events parser against
//! malformed input.
//! Target: ManualEventsFile YAML

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::event::ManualEventsFile;

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    // Try to parse as ManualEventsFile
    let _: Result<ManualEventsFile, _> = serde_yaml::from_str(input);
    // We don't care if it fails - we just want to ensure it doesn't panic
});
