use proptest::prelude::*;
use shiplog_trie::Trie;

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn inserted_words_are_found(words in proptest::collection::vec("[a-z]{1,12}", 1..50)) {
        let mut trie = Trie::new();
        for w in &words {
            trie.insert(w, None);
        }
        for w in &words {
            prop_assert!(trie.search(w), "word '{w}' not found after insert");
        }
    }

    #[test]
    fn size_equals_unique_word_count(words in proptest::collection::vec("[a-z]{1,8}", 1..60)) {
        let mut trie = Trie::new();
        for w in &words {
            trie.insert(w, None);
        }
        let unique: std::collections::HashSet<&String> = words.iter().collect();
        prop_assert_eq!(trie.len(), unique.len());
    }

    #[test]
    fn autocomplete_returns_all_matching_words(
        prefix in "[a-z]{1,4}",
        suffixes in proptest::collection::vec("[a-z]{0,6}", 1..20),
    ) {
        let mut trie = Trie::new();
        let mut expected = std::collections::HashSet::new();
        for s in &suffixes {
            let word = format!("{prefix}{s}");
            trie.insert(&word, None);
            expected.insert(word);
        }
        let results: std::collections::HashSet<String> =
            trie.autocomplete(&prefix).into_iter().collect();
        prop_assert_eq!(results, expected);
    }

    #[test]
    fn starts_with_is_true_for_any_prefix_of_inserted_word(word in "[a-z]{2,15}") {
        let mut trie = Trie::new();
        trie.insert(&word, None);
        for end in 1..=word.len() {
            let prefix: String = word.chars().take(end).collect();
            prop_assert!(trie.starts_with(&prefix), "starts_with failed for prefix '{prefix}'");
        }
    }

    #[test]
    fn get_returns_stored_value(word in "[a-z]{1,10}", value in "[a-z]{1,10}") {
        let mut trie = Trie::new();
        trie.insert(&word, Some(value.clone()));
        prop_assert_eq!(trie.get(&word), Some(&value));
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_trie() {
    let trie = Trie::new();
    assert!(trie.is_empty());
    assert_eq!(trie.len(), 0);
    assert!(!trie.search("anything"));
    assert!(!trie.starts_with("a"));
    assert!(trie.autocomplete("").is_empty());
}

#[test]
fn single_character_word() {
    let mut trie = Trie::new();
    trie.insert("a", Some("alpha".to_string()));
    assert!(trie.search("a"));
    assert_eq!(trie.get("a"), Some(&"alpha".to_string()));
    assert!(!trie.search("b"));
}

#[test]
fn empty_string_word() {
    let mut trie = Trie::new();
    trie.insert("", Some("empty".to_string()));
    assert!(trie.search(""));
    assert_eq!(trie.len(), 1);
    assert_eq!(trie.get(""), Some(&"empty".to_string()));
}

#[test]
fn duplicate_insert_does_not_increase_size() {
    let mut trie = Trie::new();
    trie.insert("hello", None);
    trie.insert("hello", None);
    trie.insert("hello", None);
    assert_eq!(trie.len(), 1);
}

#[test]
fn duplicate_insert_updates_value() {
    let mut trie = Trie::new();
    trie.insert("key", Some("v1".to_string()));
    trie.insert("key", Some("v2".to_string()));
    assert_eq!(trie.get("key"), Some(&"v2".to_string()));
    assert_eq!(trie.len(), 1);
}

#[test]
fn prefix_and_full_word_coexist() {
    let mut trie = Trie::new();
    trie.insert("he", None);
    trie.insert("hello", None);
    assert!(trie.search("he"));
    assert!(trie.search("hello"));
    assert!(!trie.search("hel"));
    assert_eq!(trie.len(), 2);
}

#[test]
fn autocomplete_empty_prefix_returns_all() {
    let mut trie = Trie::new();
    trie.insert("apple", None);
    trie.insert("banana", None);
    trie.insert("cherry", None);
    let mut results = trie.autocomplete("");
    results.sort();
    assert_eq!(results, vec!["apple", "banana", "cherry"]);
}

#[test]
fn autocomplete_nonexistent_prefix_returns_empty() {
    let mut trie = Trie::new();
    trie.insert("apple", None);
    assert!(trie.autocomplete("xyz").is_empty());
}

#[test]
fn get_nonexistent_returns_none() {
    let trie = Trie::new();
    assert_eq!(trie.get("missing"), None);
}

#[test]
fn default_creates_empty_trie() {
    let trie = Trie::default();
    assert!(trie.is_empty());
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_many_words() {
    let mut trie = Trie::new();
    let words: Vec<String> = (0..1000).map(|i| format!("word{i:04}")).collect();
    for w in &words {
        trie.insert(w, Some(w.clone()));
    }
    assert_eq!(trie.len(), 1000);
    for w in &words {
        assert!(trie.search(w));
        assert_eq!(trie.get(w), Some(w));
    }
    let completions = trie.autocomplete("word00");
    assert_eq!(completions.len(), 100); // word0000..word0099
}
