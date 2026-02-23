//! Property tests for shiplog-redaction-repo.

use proptest::prelude::*;
use shiplog_redaction_repo::redact_repo_public;
use shiplog_schema::event::{RepoRef, RepoVisibility};

fn alias(kind: &str, value: &str) -> String {
    let mut acc = 0xcbf29ce484222325u64;
    for byte in kind.bytes().chain(value.bytes()) {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(0x100000001b3);
    }
    format!("{kind}-{acc:016x}")
}

proptest! {
    #[test]
    fn prop_public_repo_redaction_always_removes_url_and_sets_unknown_visibility(
        owner in "[a-z0-9_-]{1,32}",
        repo in "[a-z0-9_-]{1,32}",
        has_url in any::<bool>(),
    ) {
        let full_name = format!("{owner}/{repo}");
        let input = RepoRef {
            full_name: full_name.clone(),
            html_url: has_url.then(|| format!("https://github.com/{full_name}")),
            visibility: RepoVisibility::Private,
        };

        let out = redact_repo_public(&input, &alias);
        prop_assert_eq!(out.html_url, None);
        prop_assert_eq!(out.visibility, RepoVisibility::Unknown);
        prop_assert_eq!(out.full_name, alias("repo", &full_name));
    }

    #[test]
    fn prop_same_repo_and_alias_resolver_produces_same_alias(
        owner in "[a-z0-9_-]{1,32}",
        repo in "[a-z0-9_-]{1,32}",
    ) {
        let full_name = format!("{owner}/{repo}");
        let input = RepoRef {
            full_name: full_name.clone(),
            html_url: Some(format!("https://github.com/{full_name}")),
            visibility: RepoVisibility::Public,
        };

        let a = redact_repo_public(&input, &alias);
        let b = redact_repo_public(&input, &alias);
        prop_assert_eq!(a.full_name, b.full_name);
        prop_assert_eq!(a.html_url, b.html_url);
        prop_assert_eq!(a.visibility, b.visibility);
    }
}
