//! Canonical key builders for shiplog API caching.
//!
//! This crate intentionally has one responsibility:
//! constructing stable cache-key strings for API request shapes.

/// Cache key builder for API requests.
pub struct CacheKey;

impl CacheKey {
    /// Create a key for a search query.
    #[must_use]
    pub fn search(query: &str, page: u32, per_page: u32) -> String {
        format!(
            "search:{}:page{}:per{}",
            Self::hash_query(query),
            page,
            per_page
        )
    }

    /// Create a key for pull request details.
    #[must_use]
    pub fn pr_details(pr_api_url: &str) -> String {
        format!("pr:details:{pr_api_url}")
    }

    /// Create a key for pull request reviews.
    #[must_use]
    pub fn pr_reviews(pr_api_url: &str, page: u32) -> String {
        format!("pr:reviews:{pr_api_url}:page{page}")
    }

    /// Create a key for GitLab merge-request notes.
    #[must_use]
    pub fn mr_notes(project_id: u64, mr_iid: u64, page: u32) -> String {
        format!("gitlab:mr:notes:project{project_id}:mr{mr_iid}:page{page}")
    }

    fn hash_query(query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::CacheKey;

    #[test]
    fn search_key_has_expected_shape() {
        let key = CacheKey::search("is:pr author:octocat", 3, 100);
        assert!(key.starts_with("search:"));
        assert!(key.ends_with(":page3:per100"));
    }

    #[test]
    fn search_key_is_stable_for_same_input() {
        let k1 = CacheKey::search("is:pr author:octocat", 1, 100);
        let k2 = CacheKey::search("is:pr author:octocat", 1, 100);
        assert_eq!(k1, k2);
    }

    #[test]
    fn pr_details_key_has_expected_prefix() {
        let key = CacheKey::pr_details("https://api.github.com/repos/o/r/pulls/1");
        assert_eq!(key, "pr:details:https://api.github.com/repos/o/r/pulls/1");
    }

    #[test]
    fn pr_reviews_key_has_expected_prefix_and_page() {
        let key = CacheKey::pr_reviews("https://api.github.com/repos/o/r/pulls/1", 4);
        assert_eq!(
            key,
            "pr:reviews:https://api.github.com/repos/o/r/pulls/1:page4"
        );
    }

    #[test]
    fn mr_notes_key_has_expected_segments() {
        let key = CacheKey::mr_notes(42, 7, 2);
        assert_eq!(key, "gitlab:mr:notes:project42:mr7:page2");
    }
}
