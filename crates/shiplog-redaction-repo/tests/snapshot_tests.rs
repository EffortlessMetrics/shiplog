//! Snapshot tests for shiplog-redaction-repo redaction outputs.

use shiplog_redaction_repo::redact_repo_public;
use shiplog_schema::event::{RepoRef, RepoVisibility};

fn stable_alias(kind: &str, value: &str) -> String {
    format!("{kind}-REDACTED({value})")
}

#[test]
fn snapshot_redacted_private_repo() {
    let repo = RepoRef {
        full_name: "acme/private-service".to_string(),
        html_url: Some("https://github.com/acme/private-service".to_string()),
        visibility: RepoVisibility::Private,
    };
    let out = redact_repo_public(&repo, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("redacted_private_repo", json);
}

#[test]
fn snapshot_redacted_public_repo() {
    let repo = RepoRef {
        full_name: "oss/public-lib".to_string(),
        html_url: Some("https://github.com/oss/public-lib".to_string()),
        visibility: RepoVisibility::Public,
    };
    let out = redact_repo_public(&repo, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("redacted_public_repo", json);
}

#[test]
fn snapshot_redacted_repo_no_url() {
    let repo = RepoRef {
        full_name: "internal/no-url-repo".to_string(),
        html_url: None,
        visibility: RepoVisibility::Unknown,
    };
    let out = redact_repo_public(&repo, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("redacted_repo_no_url", json);
}
