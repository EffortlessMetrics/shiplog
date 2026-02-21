//! Fuzz harness for YAML workstream edits (workstreams.yaml)
//!
//! This harness tests the robustness of the YAML parser against malformed input.
//! Target: `serde_yaml` workstream loader

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::workstream::WorkstreamsFile;

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    // Try to parse as WorkstreamsFile
    let _result: Result<WorkstreamsFile, _> = serde_yaml::from_str(input);
    // We don't care if it fails - we just want to ensure it doesn't panic
});
