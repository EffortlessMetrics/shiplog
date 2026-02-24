//! Integration tests for shiplog-cache-key.

use shiplog_cache_key::CacheKey;

#[test]
fn key_namespaces_do_not_collide_for_same_identifier() {
    let url = "https://api.github.com/repos/acme/service/pulls/42";

    let details = CacheKey::pr_details(url);
    let reviews = CacheKey::pr_reviews(url, 1);
    let search = CacheKey::search("is:pr author:alice", 1, 100);
    let notes = CacheKey::mr_notes(42, 7, 1);

    assert_ne!(details, reviews);
    assert_ne!(details, search);
    assert_ne!(details, notes);
    assert_ne!(reviews, search);
    assert_ne!(reviews, notes);
    assert_ne!(search, notes);
}

#[test]
fn search_key_keeps_page_and_per_page_contract() {
    let query = "is:pr is:merged author:alice";
    let key = CacheKey::search(query, 3, 50);

    assert!(key.starts_with("search:"));
    assert!(key.ends_with(":page3:per50"));
    assert!(!key.contains(' '));
}
