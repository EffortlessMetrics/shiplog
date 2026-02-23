//! Canonical repository redaction contract for shiplog public projections.
//!
//! This crate has a narrow responsibility:
//! - define the alias resolver abstraction used by public repo redaction
//! - apply deterministic structural redaction for `RepoRef`

use shiplog_schema::event::{RepoRef, RepoVisibility};

/// Alias resolver used by public repository redaction.
pub trait AliasResolver {
    fn alias(&self, kind: &str, value: &str) -> String;
}

impl<F> AliasResolver for F
where
    F: Fn(&str, &str) -> String,
{
    fn alias(&self, kind: &str, value: &str) -> String {
        (self)(kind, value)
    }
}

/// Redact a repository reference for `public` profile projection.
#[must_use]
pub fn redact_repo_public<A: AliasResolver + ?Sized>(repo: &RepoRef, aliases: &A) -> RepoRef {
    RepoRef {
        full_name: aliases.alias("repo", &repo.full_name),
        html_url: None,
        visibility: RepoVisibility::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_alias_resolver_is_supported() {
        let resolver = |kind: &str, value: &str| format!("{kind}:{value}");
        assert_eq!(resolver.alias("repo", "acme/core"), "repo:acme/core");
    }

    #[test]
    fn redact_repo_public_applies_public_contract() {
        let repo = RepoRef {
            full_name: "acme/private-repo".to_string(),
            html_url: Some("https://github.com/acme/private-repo".to_string()),
            visibility: RepoVisibility::Private,
        };
        let resolver = |kind: &str, value: &str| format!("{kind}-alias:{value}");

        let out = redact_repo_public(&repo, &resolver);
        assert_eq!(out.full_name, "repo-alias:acme/private-repo");
        assert!(out.html_url.is_none());
        assert_eq!(out.visibility, RepoVisibility::Unknown);
    }
}
