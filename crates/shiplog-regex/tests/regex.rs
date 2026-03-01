//! Integration tests for shiplog-regex.

use proptest::prelude::*;
use shiplog_regex::*;

// ── Known-answer tests ──────────────────────────────────────────

#[test]
fn compile_valid_pattern() {
    assert!(compile(r"\d+").is_ok());
    assert!(compile(r"[a-z]+").is_ok());
    assert!(compile(r"^foo$").is_ok());
}

#[test]
fn compile_invalid_pattern() {
    assert!(compile(r"[").is_err());
    assert!(compile(r"(?P<>)").is_err());
    assert!(compile(r"*").is_err());
}

#[test]
fn is_match_basic() {
    assert!(is_match(r"\d+", "abc123def").unwrap());
    assert!(!is_match(r"\d+", "abcdef").unwrap());
    assert!(is_match(r"^hello$", "hello").unwrap());
    assert!(!is_match(r"^hello$", "hello world").unwrap());
}

#[test]
fn is_match_invalid_pattern() {
    assert!(is_match(r"[", "text").is_err());
}

#[test]
fn find_all_basic() {
    let matches = find_all(r"\d+", "a1b22c333").unwrap();
    assert_eq!(matches, vec!["1", "22", "333"]);
}

#[test]
fn find_all_no_matches() {
    let matches = find_all(r"\d+", "no digits here").unwrap();
    assert!(matches.is_empty());
}

#[test]
fn find_all_empty_text() {
    let matches = find_all(r"\d+", "").unwrap();
    assert!(matches.is_empty());
}

#[test]
fn replace_all_basic() {
    assert_eq!(replace_all(r"\d+", "a1b2c3", "X").unwrap(), "aXbXcX");
}

#[test]
fn replace_all_no_match() {
    assert_eq!(replace_all(r"\d+", "no digits", "X").unwrap(), "no digits");
}

#[test]
fn replace_all_empty_replacement() {
    assert_eq!(replace_all(r"\s+", "a b c", "").unwrap(), "abc");
}

#[test]
fn capture_groups_basic() {
    let groups = capture_groups(r"(\w+)@(\w+)", "user@host").unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0], vec!["user", "host"]);
}

#[test]
fn capture_groups_multiple_matches() {
    let groups = capture_groups(r"(\d+)-(\d+)", "1-2 3-4 5-6").unwrap();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0], vec!["1", "2"]);
    assert_eq!(groups[1], vec!["3", "4"]);
    assert_eq!(groups[2], vec!["5", "6"]);
}

#[test]
fn capture_groups_no_match() {
    let groups = capture_groups(r"(\d+)-(\d+)", "no match").unwrap();
    assert!(groups.is_empty());
}

#[test]
fn split_basic() {
    assert_eq!(split(r"[,;]", "a,b;c").unwrap(), vec!["a", "b", "c"]);
}

#[test]
fn split_no_delimiter() {
    assert_eq!(split(r",", "no delim").unwrap(), vec!["no delim"]);
}

#[test]
fn split_empty_text() {
    assert_eq!(split(r",", "").unwrap(), vec![""]);
}

#[test]
fn is_valid_pattern_tests() {
    assert!(is_valid_pattern(r"\d+"));
    assert!(is_valid_pattern(r"[a-zA-Z0-9]+"));
    assert!(is_valid_pattern(r"^$"));
    assert!(is_valid_pattern(r"(?:group)"));
    assert!(!is_valid_pattern(r"["));
    assert!(!is_valid_pattern(r"*"));
}

#[test]
fn count_matches_basic() {
    assert_eq!(count_matches(r"\d+", "1 22 333").unwrap(), 3);
    assert_eq!(count_matches(r"\d+", "no digits").unwrap(), 0);
    assert_eq!(count_matches(r".", "abc").unwrap(), 3);
}

// ── Edge cases ──────────────────────────────────────────────────

#[test]
fn empty_pattern() {
    // Empty pattern matches at every position
    let matches = find_all(r"", "abc").unwrap();
    assert!(!matches.is_empty());
}

#[test]
fn unicode_matching() {
    assert!(is_match(r"\p{L}+", "日本語").unwrap());
    let matches = find_all(r"\p{L}+", "hello 世界 foo").unwrap();
    assert_eq!(matches, vec!["hello", "世界", "foo"]);
}

#[test]
fn special_regex_chars_in_text() {
    assert!(is_match(r"\.", "a.b").unwrap());
    assert!(!is_match(r"\.", "abc").unwrap());
}

#[test]
fn greedy_vs_lazy() {
    let greedy = find_all(r"<.+>", "<a><b>").unwrap();
    assert_eq!(greedy, vec!["<a><b>"]);

    let lazy = find_all(r"<.+?>", "<a><b>").unwrap();
    assert_eq!(lazy, vec!["<a>", "<b>"]);
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_count_matches_equals_find_all_len(text in "[a-z0-9 ]{0,100}") {
        let pattern = r"\d+";
        let count = count_matches(pattern, &text).unwrap();
        let found = find_all(pattern, &text).unwrap();
        prop_assert_eq!(count, found.len());
    }

    #[test]
    fn prop_replace_all_removes_matches(text in "[a-z0-9 ]{0,50}") {
        let replaced = replace_all(r"\d+", &text, "").unwrap();
        let count_after = count_matches(r"\d+", &replaced).unwrap();
        prop_assert_eq!(count_after, 0);
    }

    #[test]
    fn prop_is_match_consistent_with_find_all(text in "[a-z0-9]{0,50}") {
        let pattern = r"\d+";
        let matched = is_match(pattern, &text).unwrap();
        let found = find_all(pattern, &text).unwrap();
        prop_assert_eq!(matched, !found.is_empty());
    }

    #[test]
    fn prop_split_rejoin(text in "[a-z]{1,5}(,[a-z]{1,5}){0,5}") {
        let parts = split(r",", &text).unwrap();
        let rejoined = parts.join(",");
        prop_assert_eq!(rejoined, text);
    }

    #[test]
    fn prop_is_valid_pattern_for_literal(s in "[a-zA-Z0-9]{1,20}") {
        // Alphanumeric strings are always valid regex patterns
        prop_assert!(is_valid_pattern(&s));
    }
}
