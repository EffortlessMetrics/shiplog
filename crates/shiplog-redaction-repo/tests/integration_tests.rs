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
