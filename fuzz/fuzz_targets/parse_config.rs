//! Fuzz harness for Configuration files (shiplog.yaml)
//!
//! This harness tests the robustness of the configuration parser against
//! malformed input.
//! Target: CLI configuration YAML

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

// Define a simplified config structure for fuzzing
#[derive(Debug, Deserialize)]
struct ShiplogConfig {
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    output_dir: Option<String>,
    #[serde(default)]
    cache_dir: Option<String>,
}

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    // Try to parse as config
    let _: Result<ShiplogConfig, _> = serde_yaml::from_str(input);
    // We don't care if it fails - we just want to ensure it doesn't panic
});
