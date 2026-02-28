//! Integration tests for shiplog-path.

use proptest::prelude::*;
use shiplog_path::*;
use std::path::PathBuf;

// ── Known-answer tests ──────────────────────────────────────────

#[test]
fn normalize_removes_dotdot() {
    let p = normalize_path("foo/bar/../baz");
    assert_eq!(p, PathBuf::from("foo/baz"));
}

#[test]
fn normalize_removes_dot() {
    let p = normalize_path("./foo/./bar");
    assert_eq!(p, PathBuf::from("foo/bar"));
}

#[test]
fn normalize_complex() {
    let p = normalize_path("a/b/c/../../d");
    assert_eq!(p, PathBuf::from("a/d"));
}

#[test]
fn normalize_already_normal() {
    let p = normalize_path("foo/bar/baz");
    assert_eq!(p, PathBuf::from("foo/bar/baz"));
}

#[test]
fn normalize_empty_path() {
    let p = normalize_path("");
    assert_eq!(p, PathBuf::from(""));
}

// ── join_paths tests ────────────────────────────────────────────

#[test]
fn join_paths_basic() {
    let p = join_paths("base", &["foo", "bar"]);
    let s = to_forward_slashes(&p);
    assert_eq!(s, "base/foo/bar");
}

#[test]
fn join_paths_empty_segments() {
    let p = join_paths("base", &[]);
    assert_eq!(p, PathBuf::from("base"));
}

#[test]
fn join_paths_single_segment() {
    let p = join_paths("base", &["child"]);
    let s = to_forward_slashes(&p);
    assert_eq!(s, "base/child");
}

// ── relative_path tests ─────────────────────────────────────────

#[test]
fn relative_path_basic() {
    let rel = relative_path("a/b", "a/b/c/d");
    assert!(rel.is_some());
    assert_eq!(rel.unwrap(), PathBuf::from("c/d"));
}

#[test]
fn relative_path_same() {
    let rel = relative_path("a/b", "a/b");
    assert!(rel.is_some());
    assert_eq!(rel.unwrap(), PathBuf::from(""));
}

#[test]
fn relative_path_divergent() {
    let rel = relative_path("a/b", "a/c");
    assert!(rel.is_none());
}

// ── is_absolute / is_relative ───────────────────────────────────

#[test]
fn absolute_relative_consistency() {
    let paths = ["foo/bar", "relative"];
    for p in &paths {
        assert_eq!(is_absolute_path(p), !is_relative_path(p));
    }
}

#[cfg(windows)]
#[test]
fn windows_absolute_paths() {
    assert!(is_absolute_path("C:\\foo\\bar"));
    assert!(is_absolute_path("D:\\"));
    assert!(!is_absolute_path("foo\\bar"));
}

// ── get_extension tests ─────────────────────────────────────────

#[test]
fn get_extension_basic() {
    assert_eq!(get_extension("file.txt"), Some("txt".to_string()));
    assert_eq!(get_extension("file.tar.gz"), Some("gz".to_string()));
    assert_eq!(get_extension("noext"), None);
    assert_eq!(get_extension(".hidden"), None);
}

#[test]
fn get_extension_multiple_dots() {
    assert_eq!(get_extension("my.file.name.rs"), Some("rs".to_string()));
}

// ── with_extension tests ────────────────────────────────────────

#[test]
fn with_extension_basic() {
    let p = with_extension("file.txt", "rs");
    assert_eq!(p, PathBuf::from("file.rs"));
}

#[test]
fn with_extension_add_new() {
    let p = with_extension("file", "txt");
    assert_eq!(p, PathBuf::from("file.txt"));
}

#[test]
fn with_extension_remove() {
    let p = with_extension("file.txt", "");
    assert_eq!(p, PathBuf::from("file"));
}

// ── to_forward_slashes tests ────────────────────────────────────

#[test]
fn forward_slashes_basic() {
    assert_eq!(to_forward_slashes("foo\\bar\\baz"), "foo/bar/baz");
}

#[test]
fn forward_slashes_already_forward() {
    assert_eq!(to_forward_slashes("foo/bar/baz"), "foo/bar/baz");
}

#[test]
fn forward_slashes_mixed() {
    assert_eq!(to_forward_slashes("foo\\bar/baz"), "foo/bar/baz");
}

#[test]
fn forward_slashes_empty() {
    assert_eq!(to_forward_slashes(""), "");
}

// ── Edge cases ──────────────────────────────────────────────────

#[test]
fn normalize_only_dots() {
    let p = normalize_path("./././.");
    assert_eq!(p, PathBuf::from(""));
}

#[test]
fn join_paths_with_trailing_slash() {
    let p = join_paths("base/", &["foo"]);
    let s = to_forward_slashes(&p);
    assert!(s.contains("base") && s.contains("foo"));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_normalize_is_idempotent(segments in proptest::collection::vec("[a-z]{1,5}", 1..5)) {
        let path = segments.join("/");
        let once = normalize_path(&path);
        let twice = normalize_path(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_forward_slashes_is_idempotent(s in "[a-z/\\\\]{0,30}") {
        let once = to_forward_slashes(&s);
        let twice = to_forward_slashes(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_forward_slashes_no_backslashes(s in "[a-z/\\\\]{0,30}") {
        let result = to_forward_slashes(&s);
        prop_assert!(!result.contains('\\'));
    }

    #[test]
    fn prop_is_absolute_xor_relative(path in "[a-zA-Z./\\\\]{1,30}") {
        prop_assert_eq!(is_absolute_path(&path), !is_relative_path(&path));
    }

    #[test]
    fn prop_join_paths_extends(
        base in "[a-z]{1,5}",
        segments in proptest::collection::vec("[a-z]{1,5}", 1..4)
    ) {
        let segment_refs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
        let joined = join_paths(&base, &segment_refs);
        let s = to_forward_slashes(&joined);
        prop_assert!(s.starts_with(&base));
        for seg in &segments {
            prop_assert!(s.contains(seg.as_str()));
        }
    }
}
