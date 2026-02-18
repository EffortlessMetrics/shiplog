//! Fuzz harness for GitHub API responses
//!
//! This harness tests the robustness of the GitHub API JSON parser against
//! malformed or unexpected responses.
//! Target: GitHub API response types

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_ingest_github::types::*;
use serde::Deserialize;

fuzz_target!(|data: &[u8]| {
    // Ensure the input is valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip non-UTF-8 input
    };

    // Try to parse as various GitHub API types
    let _: Result<PullRequest, _> = serde_json::from_str(input);
    let _: Result<Review, _> = serde_json::from_str(input);
    let _: Result<Actor, _> = serde_json::from_str(input);
    let _: Result<Repository, _> = serde_json::from_str(input);

    // We don't care if it fails - we just want to ensure it doesn't panic
});
