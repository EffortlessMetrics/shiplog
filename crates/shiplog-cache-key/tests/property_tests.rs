//! Property tests for shiplog-cache-key.

use proptest::prelude::*;
use shiplog_cache_key::CacheKey;

proptest! {
    #[test]
    fn prop_search_key_is_stable(
        query in ".*",
        page in any::<u32>(),
        per_page in any::<u32>(),
    ) {
        let k1 = CacheKey::search(&query, page, per_page);
        let k2 = CacheKey::search(&query, page, per_page);
        prop_assert_eq!(k1, k2);
    }

    #[test]
    fn prop_search_key_changes_when_paging_changes(
        query in ".*",
        page_a in any::<u32>(),
        page_b in any::<u32>(),
        per_page in any::<u32>(),
    ) {
        prop_assume!(page_a != page_b);
        let k1 = CacheKey::search(&query, page_a, per_page);
        let k2 = CacheKey::search(&query, page_b, per_page);
        prop_assert_ne!(k1, k2);
    }

    #[test]
    fn prop_pr_review_key_contains_page(url in ".*", page in any::<u32>()) {
        let key = CacheKey::pr_reviews(&url, page);
        let page_suffix = format!(":page{}", page);
        prop_assert!(key.starts_with("pr:reviews:"));
        prop_assert!(key.ends_with(&page_suffix));
    }

    #[test]
    fn prop_gitlab_notes_key_contains_identifiers(
        project_id in any::<u64>(),
        mr_iid in any::<u64>(),
        page in any::<u32>(),
    ) {
        let key = CacheKey::mr_notes(project_id, mr_iid, page);
        let project_segment = format!("project{}", project_id);
        let mr_segment = format!(":mr{}:", mr_iid);
        let page_suffix = format!(":page{}", page);
        prop_assert!(key.starts_with("gitlab:mr:notes:"));
        prop_assert!(key.contains(&project_segment));
        prop_assert!(key.contains(&mr_segment));
        prop_assert!(key.ends_with(&page_suffix));
    }
}
