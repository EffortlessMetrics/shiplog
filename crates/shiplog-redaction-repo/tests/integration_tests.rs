//! Integration tests for shiplog-redaction-repo.

use shiplog_alias::DeterministicAliasStore;
use shiplog_redaction_repo::{AliasResolver, redact_repo_public};
use shiplog_schema::event::{RepoRef, RepoVisibility};

struct PrefixAlias;

impl AliasResolver for PrefixAlias {
    fn alias(&self, kind: &str, value: &str) -> String {
        format!("{kind}:{}", value.to_uppercase())
    }
}

#[test]
fn integrates_with_deterministic_alias_store() {
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let resolver = |kind: &str, value: &str| alias_store.alias(kind, value);

    let repo = RepoRef {
        full_name: "acme/private-repo".to_string(),
        html_url: Some("https://github.com/acme/private-repo".to_string()),
        visibility: RepoVisibility::Private,
    };

    let out = redact_repo_public(&repo, &resolver);
    assert_eq!(
        out.full_name,
        alias_store.alias("repo", "acme/private-repo")
    );
    assert_eq!(out.html_url, None);
    assert_eq!(out.visibility, RepoVisibility::Unknown);
}

#[test]
fn supports_custom_alias_resolver_implementations() {
    let repo = RepoRef {
        full_name: "org/service".to_string(),
        html_url: Some("https://example.com/org/service".to_string()),
        visibility: RepoVisibility::Public,
    };

    let out = redact_repo_public(&repo, &PrefixAlias);
    assert_eq!(out.full_name, "repo:ORG/SERVICE");
    assert!(out.html_url.is_none());
    assert_eq!(out.visibility, RepoVisibility::Unknown);
}

#[test]
fn deterministic_alias_store_produces_stable_results() {
    let store = DeterministicAliasStore::new(b"stable-key");
    let resolver = |kind: &str, value: &str| store.alias(kind, value);
    let repo = RepoRef {
        full_name: "acme/core".to_string(),
        html_url: Some("https://github.com/acme/core".to_string()),
        visibility: RepoVisibility::Private,
    };

    let first = redact_repo_public(&repo, &resolver);
    let second = redact_repo_public(&repo, &resolver);
    assert_eq!(first.full_name, second.full_name);
}

#[test]
fn redaction_strips_all_visibilities() {
    for vis in [
        RepoVisibility::Public,
        RepoVisibility::Private,
        RepoVisibility::Unknown,
    ] {
        let repo = RepoRef {
            full_name: "org/any".to_string(),
            html_url: Some("https://github.com/org/any".to_string()),
            visibility: vis,
        };
        let alias = |kind: &str, value: &str| format!("{kind}:{value}");
        let out = redact_repo_public(&repo, &alias);
        assert_eq!(out.visibility, RepoVisibility::Unknown);
        assert!(out.html_url.is_none());
    }
}

#[test]
fn redaction_works_when_html_url_is_already_none() {
    let repo = RepoRef {
        full_name: "org/repo".to_string(),
        html_url: None,
        visibility: RepoVisibility::Private,
    };
    let alias = |kind: &str, value: &str| format!("{kind}:{value}");
    let out = redact_repo_public(&repo, &alias);
    assert_eq!(out.html_url, None);
    assert_eq!(out.full_name, "repo:org/repo");
}

#[test]
fn different_repos_get_different_aliases() {
    let store = DeterministicAliasStore::new(b"diff-key");
    let resolver = |kind: &str, value: &str| store.alias(kind, value);

    let repo_a = RepoRef {
        full_name: "acme/alpha".to_string(),
        html_url: None,
        visibility: RepoVisibility::Private,
    };
    let repo_b = RepoRef {
        full_name: "acme/beta".to_string(),
        html_url: None,
        visibility: RepoVisibility::Private,
    };

    let out_a = redact_repo_public(&repo_a, &resolver);
    let out_b = redact_repo_public(&repo_b, &resolver);
    assert_ne!(out_a.full_name, out_b.full_name);
}

#[test]
fn closure_resolver_captures_external_state() {
    let prefix = "custom";
    let resolver = |kind: &str, value: &str| format!("{prefix}-{kind}-{value}");
    let repo = RepoRef {
        full_name: "org/repo".to_string(),
        html_url: None,
        visibility: RepoVisibility::Private,
    };
    let out = redact_repo_public(&repo, &resolver);
    assert_eq!(out.full_name, "custom-repo-org/repo");
}
