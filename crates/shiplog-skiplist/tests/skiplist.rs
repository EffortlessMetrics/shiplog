use proptest::prelude::*;
use shiplog_skiplist::SkipList;

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn size_matches_insert_count(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut list = SkipList::new();
        for v in &values {
            list.insert(*v);
        }
        prop_assert_eq!(list.size(), values.len());
    }

    #[test]
    fn not_empty_after_insert(value: i32) {
        let mut list = SkipList::new();
        list.insert(value);
        prop_assert!(!list.is_empty());
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn new_skiplist_is_empty() {
    let list: SkipList<i32> = SkipList::new();
    assert!(list.is_empty());
    assert_eq!(list.size(), 0);
}

#[test]
fn default_skiplist_is_empty() {
    let list: SkipList<i32> = SkipList::default();
    assert!(list.is_empty());
}

#[test]
fn single_insert() {
    let mut list = SkipList::new();
    list.insert(42);
    assert_eq!(list.size(), 1);
    assert!(!list.is_empty());
}

#[test]
fn multiple_inserts() {
    let mut list = SkipList::new();
    for i in 0..50 {
        list.insert(i);
    }
    assert_eq!(list.size(), 50);
}

#[test]
fn search_returns_false_on_empty() {
    let list: SkipList<i32> = SkipList::new();
    assert!(!list.search(&42));
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_many_inserts() {
    let mut list = SkipList::new();
    for i in 0..10_000 {
        list.insert(i);
    }
    assert_eq!(list.size(), 10_000);
}
