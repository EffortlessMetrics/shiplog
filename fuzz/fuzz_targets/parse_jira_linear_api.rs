//! Fuzz harness for Jira/Linear API responses
//!
//! This harness tests robustness of Jira and Linear API JSON parsers against
//! malformed or unexpected responses.
//! Target: Jira and Linear API response types (issues, tickets)

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

// Jira API response structures
#[derive(Debug, Deserialize)]
struct JiraIssue {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    created: Option<String>,
    #[serde(default)]
    updated: Option<String>,
    #[serde(default)]
    status: Option<JiraStatus>,
    #[serde(default)]
    fields: Option<JiraFields>,
}

#[derive(Debug, Deserialize)]
struct JiraStatus {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JiraFields {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    priority: Option<JiraPriority>,
}

#[derive(Debug, Deserialize)]
struct JiraPriority {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

// Linear API response structures
#[derive(Debug, Deserialize)]
struct LinearIssue {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    status: Option<LinearStatus>,
    #[serde(default)]
    project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearStatus {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    color: Option<String>,
}

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignore invalid UTF-8 (handled error)
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse as Jira issue
        let _ = serde_json::from_str::<JiraIssue>(text);
        
        // Try to parse as Linear issue
        let _ = serde_json::from_str::<LinearIssue>(text);
    }
});
