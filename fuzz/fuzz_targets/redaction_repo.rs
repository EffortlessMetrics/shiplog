//! Fuzz harness for public repository redaction contracts.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_redaction_repo::redact_repo_public;
use shiplog_schema::event::{RepoRef, RepoVisibility};

fn alias(kind: &str, value: &str) -> String {
    let mut acc = 14695981039346656037u64;
    for byte in kind.bytes().chain(value.bytes()) {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(1099511628211);
    }
    format!("{kind}-{acc:016x}")
}

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or(0) % 3;
    let split = data.len() / 2;
    let full_name = String::from_utf8_lossy(&data[..split]).to_string();
    let url_raw = String::from_utf8_lossy(&data[split..]).to_string();

    let input = RepoRef {
        full_name: full_name.clone(),
        html_url: if url_raw.is_empty() { None } else { Some(url_raw) },
        visibility: match selector {
            0 => RepoVisibility::Private,
            1 => RepoVisibility::Public,
            _ => RepoVisibility::Unknown,
        },
    };

    let out = redact_repo_public(&input, &alias);
    assert_eq!(out.full_name, alias("repo", &full_name));
    assert!(out.html_url.is_none());
    assert_eq!(out.visibility, RepoVisibility::Unknown);
});
