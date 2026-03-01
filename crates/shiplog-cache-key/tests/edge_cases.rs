//! Edge-case tests for shiplog-cache-key.

use shiplog_cache_key::CacheKey;

#[test]
fn search_key_with_empty_query() {
    let key = CacheKey::search("", 1, 100);
    assert!(key.starts_with("search:"));
    assert!(key.ends_with(":page1:per100"));
}

#[test]
fn search_key_with_unicode_query() {
    let key = CacheKey::search("日本語クエリ", 1, 50);
    assert!(key.starts_with("search:"));
    assert!(key.ends_with(":page1:per50"));
}

#[test]
fn search_key_with_very_long_query() {
    let long_query = "a".repeat(10_000);
    let key = CacheKey::search(&long_query, 1, 100);
    assert!(key.starts_with("search:"));
    // Hash ensures key doesn't grow unboundedly
    assert!(key.len() < 100, "key length should be bounded by hashing");
}

#[test]
fn search_key_different_queries_produce_different_hashes() {
    let k1 = CacheKey::search("query_a", 1, 100);
    let k2 = CacheKey::search("query_b", 1, 100);
    assert_ne!(k1, k2);
}

#[test]
fn search_key_zero_page_and_per_page() {
    let key = CacheKey::search("test", 0, 0);
    assert!(key.ends_with(":page0:per0"));
}

#[test]
fn search_key_max_u32_page() {
    let key = CacheKey::search("test", u32::MAX, u32::MAX);
    let expected_suffix = format!(":page{}:per{}", u32::MAX, u32::MAX);
    assert!(key.ends_with(&expected_suffix));
}

#[test]
fn pr_details_with_empty_url() {
    let key = CacheKey::pr_details("");
    assert_eq!(key, "pr:details:");
}

#[test]
fn pr_details_with_ghe_url() {
    let url = "https://github.example.com/api/v3/repos/org/repo/pulls/99";
    let key = CacheKey::pr_details(url);
    assert_eq!(key, format!("pr:details:{url}"));
}

#[test]
fn pr_reviews_page_zero() {
    let key = CacheKey::pr_reviews("https://api.github.com/repos/o/r/pulls/1", 0);
    assert!(key.ends_with(":page0"));
}

#[test]
fn mr_notes_with_zero_ids() {
    let key = CacheKey::mr_notes(0, 0, 0);
    assert_eq!(key, "gitlab:mr:notes:project0:mr0:page0");
}

#[test]
fn mr_notes_with_max_u64_ids() {
    let key = CacheKey::mr_notes(u64::MAX, u64::MAX, u32::MAX);
    assert!(key.starts_with("gitlab:mr:notes:project"));
    assert!(key.contains(&format!("mr{}", u64::MAX)));
}

#[test]
fn all_key_namespaces_have_distinct_prefixes() {
    let search = CacheKey::search("q", 1, 1);
    let details = CacheKey::pr_details("u");
    let reviews = CacheKey::pr_reviews("u", 1);
    let notes = CacheKey::mr_notes(1, 1, 1);

    let keys = [&search, &details, &reviews, &notes];
    for (i, a) in keys.iter().enumerate() {
        for (j, b) in keys.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "keys at indices {i} and {j} should differ");
            }
        }
    }
}
