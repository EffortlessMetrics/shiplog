//! Snapshot tests for shiplog-cache-key.

use shiplog_cache_key::CacheKey;

#[test]
fn snapshot_search_key() {
    let key = CacheKey::search("is:pr is:merged author:octocat", 2, 100);
    insta::assert_snapshot!("search_key_canonical", key);
}

#[test]
fn snapshot_pr_details_key() {
    let key = CacheKey::pr_details("https://api.github.com/repos/octocat/hello/pulls/42");
    insta::assert_snapshot!("pr_details_key_canonical", key);
}

#[test]
fn snapshot_pr_reviews_key() {
    let key = CacheKey::pr_reviews("https://api.github.com/repos/octocat/hello/pulls/42", 3);
    insta::assert_snapshot!("pr_reviews_key_canonical", key);
}

#[test]
fn snapshot_mr_notes_key() {
    let key = CacheKey::mr_notes(123, 456, 7);
    insta::assert_snapshot!("mr_notes_key_canonical", key);
}
