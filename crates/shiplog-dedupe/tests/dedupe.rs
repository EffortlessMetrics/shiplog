//! Integration tests for shiplog-dedupe.

use shiplog_dedupe::*;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_dedup_key_display() {
    let key = DedupKey::new("my-unique-key");
    insta::assert_snapshot!("dedup_key_display", format!("{key}"));
}

#[test]
fn snapshot_dedup_key_from_content() {
    let key = DedupKey::from_content("hello world");
    insta::assert_snapshot!("dedup_key_from_content", key.as_str());
}

#[test]
fn snapshot_dedupe_strings_result() {
    let input = vec!["apple", "banana", "apple", "cherry", "banana", "date"];
    let result = dedupe_strings(&input);
    insta::assert_yaml_snapshot!("dedupe_strings_result", result);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn dedup_idempotent(items in prop::collection::vec("[a-z]{1,5}", 0..20)) {
            let dedup = Deduplicator::default_config();
            let first = dedup.deduplicate_simple(&items);
            let second = dedup.deduplicate_simple(&first);
            prop_assert_eq!(&first, &second, "dedup should be idempotent");
        }

        #[test]
        fn dedup_preserves_unique_items(items in prop::collection::vec("[a-z]{1,5}", 0..20)) {
            let dedup = Deduplicator::default_config();
            let result = dedup.deduplicate_simple(&items);
            // Every unique item should appear exactly once
            let mut unique: Vec<String> = items.iter().map(|s| s.to_string()).collect();
            unique.sort();
            unique.dedup();
            prop_assert_eq!(result.len(), unique.len());
        }

        #[test]
        fn dedup_result_subset_of_input(items in prop::collection::vec("[a-z]{1,5}", 0..20)) {
            let dedup = Deduplicator::default_config();
            let result = dedup.deduplicate_simple(&items);
            for item in &result {
                prop_assert!(items.contains(item));
            }
        }

        #[test]
        fn dedup_never_larger_than_input(items in prop::collection::vec("[a-z]{1,3}", 0..30)) {
            let dedup = Deduplicator::default_config();
            let result = dedup.deduplicate_simple(&items);
            prop_assert!(result.len() <= items.len());
        }

        #[test]
        fn dedup_key_deterministic(content in "[a-zA-Z0-9]{1,100}") {
            let k1 = DedupKey::from_content(&content);
            let k2 = DedupKey::from_content(&content);
            prop_assert_eq!(k1, k2);
        }

        #[test]
        fn dedup_key_different_content(a in "[a-z]{1,20}", b in "[a-z]{1,20}") {
            prop_assume!(a != b);
            let k1 = DedupKey::from_content(&a);
            let k2 = DedupKey::from_content(&b);
            prop_assert_ne!(k1, k2);
        }

        #[test]
        fn dedupe_strings_idempotent(items in prop::collection::vec("[a-z]{1,5}", 0..15)) {
            let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
            let first = dedupe_strings(&refs);
            let first_refs: Vec<&str> = first.iter().map(|s| s.as_str()).collect();
            let second = dedupe_strings(&first_refs);
            prop_assert_eq!(first, second);
        }

        #[test]
        fn case_insensitive_dedup_reduces_more(items in prop::collection::vec("[a-zA-Z]{1,5}", 0..20)) {
            let sensitive = Deduplicator::new(DedupConfig::new().case_sensitive());
            let insensitive = Deduplicator::new(DedupConfig::new().case_insensitive());
            let sens_result = sensitive.deduplicate_simple(&items);
            let insens_result = insensitive.deduplicate_simple(&items);
            prop_assert!(insens_result.len() <= sens_result.len());
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn dedup_empty_input() {
    let dedup = Deduplicator::<String>::default_config();
    let items: Vec<String> = vec![];
    let result = dedup.deduplicate_simple(&items);
    assert!(result.is_empty());
}

#[test]
fn dedup_all_identical() {
    let dedup = Deduplicator::default_config();
    let items = vec!["same", "same", "same", "same"];
    let result = dedup.deduplicate_simple(&items);
    assert_eq!(result, vec!["same"]);
}

#[test]
fn dedup_all_unique() {
    let dedup = Deduplicator::default_config();
    let items = vec!["a", "b", "c", "d"];
    let result = dedup.deduplicate_simple(&items);
    assert_eq!(result, vec!["a", "b", "c", "d"]);
}

#[test]
fn dedup_single_item() {
    let dedup = Deduplicator::default_config();
    let items = vec!["only"];
    let result = dedup.deduplicate_simple(&items);
    assert_eq!(result, vec!["only"]);
}

#[test]
fn dedup_case_sensitive_keeps_variants() {
    let config = DedupConfig::new().case_sensitive();
    let dedup = Deduplicator::new(config);
    let items = vec!["Hello", "hello", "HELLO"];
    let result = dedup.deduplicate_simple(&items);
    assert_eq!(result.len(), 3);
}

#[test]
fn dedup_case_insensitive_collapses_variants() {
    let config = DedupConfig::new().case_insensitive();
    let dedup = Deduplicator::new(config);
    let items = vec!["Hello", "hello", "HELLO"];
    let result = dedup.deduplicate_simple(&items);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], "Hello"); // keeps first
}

#[test]
fn dedup_with_key_function() {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct Event {
        id: u32,
        name: String,
    }

    let dedup = Deduplicator::default_config();
    let items = vec![
        Event {
            id: 1,
            name: "first".into(),
        },
        Event {
            id: 2,
            name: "second".into(),
        },
        Event {
            id: 1,
            name: "duplicate".into(),
        },
        Event {
            id: 3,
            name: "third".into(),
        },
    ];

    let result = dedup.deduplicate(&items, |e| e.id.to_string());
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].name, "first");
}

#[test]
fn dedupe_strings_empty() {
    let result = dedupe_strings(&[]);
    assert!(result.is_empty());
}

#[test]
fn dedupe_strings_preserves_order() {
    let input = vec!["c", "a", "b", "a", "c"];
    let result = dedupe_strings(&input);
    assert_eq!(result, vec!["c", "a", "b"]);
}

#[test]
fn dedupe_by_key_empty() {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct Item(u32);
    let items: Vec<Item> = vec![];
    let result = dedupe_by_key(&items, |i| i.0.to_string());
    assert!(result.is_empty());
}

#[test]
fn dedup_key_from_string() {
    let key: DedupKey = "test".into();
    assert_eq!(key.as_str(), "test");
}

#[test]
fn dedup_key_from_owned_string() {
    let key: DedupKey = String::from("owned").into();
    assert_eq!(key.as_str(), "owned");
}

#[test]
fn dedup_key_display() {
    let key = DedupKey::new("display-me");
    assert_eq!(format!("{key}"), "display-me");
}

#[test]
fn dedup_config_defaults() {
    let config = DedupConfig::new();
    assert!(config.keep_first);
    assert!(config.case_sensitive);
}

#[test]
fn dedup_config_keep_last() {
    let config = DedupConfig::new().keep_last();
    assert!(!config.keep_first);
}

#[test]
fn dedup_config_chaining() {
    let config = DedupConfig::new()
        .keep_last()
        .case_insensitive()
        .keep_first()
        .case_sensitive();
    assert!(config.keep_first);
    assert!(config.case_sensitive);
}

#[test]
fn dedup_key_equality() {
    let k1 = DedupKey::new("same");
    let k2 = DedupKey::new("same");
    let k3 = DedupKey::new("different");
    assert_eq!(k1, k2);
    assert_ne!(k1, k3);
}

#[test]
fn dedup_key_hash_consistency() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(DedupKey::new("a"));
    set.insert(DedupKey::new("a"));
    set.insert(DedupKey::new("b"));
    assert_eq!(set.len(), 2);
}
