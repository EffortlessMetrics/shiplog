//! Fuzz harness for GitHub API responses
//!
//! This harness tests the robustness of the GitHub API JSON parser against
//! malformed or unexpected responses.
//! Target: GitHub API response types

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

// Keep this harness decoupled from shiplog-ingest-github internals.
// If needed later, expose those response structs behind a fuzz-only feature.
#[derive(Debug, Deserialize)]
struct Actor {
    #[serde(default)]
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default, rename = "private")]
    private_field: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PullBase {
    #[serde(default)]
    repo: Option<Repository>,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    created_at: Option<serde_json::Value>,
    #[serde(default)]
    merged_at: Option<serde_json::Value>,
    #[serde(default)]
    additions: Option<u64>,
    #[serde(default)]
    deletions: Option<u64>,
    #[serde(default)]
    changed_files: Option<u64>,
    #[serde(default)]
    base: Option<PullBase>,
}

#[derive(Debug, Deserialize)]
struct Review {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    submitted_at: Option<serde_json::Value>,
    #[serde(default)]
    user: Option<Actor>,
}

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
