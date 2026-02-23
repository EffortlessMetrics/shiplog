//! Fuzz harness for GitLab API responses
//!
//! This harness tests robustness of GitLab API JSON parser against
//! malformed or unexpected responses.
//! Target: GitLab API response types (merge requests, reviews)

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

// GitLab API response structures
#[derive(Debug, Deserialize)]
struct GitLabAuthor {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabProject {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    web_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitLabMergeRequest {
    #[serde(default)]
    iid: Option<u64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    number: Option<u64>,
    #[serde(default)]
    merged: Option<bool>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    merged_at: Option<String>,
    #[serde(default)]
    author: Option<GitLabAuthor>,
    #[serde(default)]
    project: Option<GitLabProject>,
}

#[derive(Debug, Deserialize)]
struct GitLabReview {
    #[serde(default)]
    mr_iid: Option<u64>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    approved: Option<bool>,
    #[serde(default)]
    author: Option<GitLabAuthor>,
}

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignore invalid UTF-8 (handled error)
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse as GitLab merge request
        let _ = serde_json::from_str::<GitLabMergeRequest>(text);
        
        // Try to parse as GitLab review
        let _ = serde_json::from_str::<GitLabReview>(text);
    }
});
