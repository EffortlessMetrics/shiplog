use proptest::prelude::*;
use shiplog_stats::{StatsAnalyzer, StatsSummary};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_event_count_matches_records(n in 1usize..500) {
        let mut analyzer = StatsAnalyzer::new();
        for i in 0..n {
            analyzer.record_event("commit", &format!("src_{}", i % 3));
        }
        prop_assert_eq!(analyzer.event_stats().total_count, n);
    }

    #[test]
    fn prop_by_type_sum_equals_total(
        commit_count in 0usize..100,
        pr_count in 0usize..100,
        review_count in 0usize..100,
    ) {
        let mut analyzer = StatsAnalyzer::new();
        for _ in 0..commit_count {
            analyzer.record_event("commit", "github");
        }
        for _ in 0..pr_count {
            analyzer.record_event("pr", "github");
        }
        for _ in 0..review_count {
            analyzer.record_event("review", "github");
        }
        let by_type = &analyzer.event_stats().by_type;
        let sum: usize = by_type.values().sum();
        prop_assert_eq!(sum, analyzer.event_stats().total_count);
    }

    #[test]
    fn prop_by_source_sum_equals_total(
        gh_count in 0usize..100,
        gl_count in 0usize..100,
    ) {
        let mut analyzer = StatsAnalyzer::new();
        for _ in 0..gh_count {
            analyzer.record_event("commit", "github");
        }
        for _ in 0..gl_count {
            analyzer.record_event("commit", "gitlab");
        }
        let by_source = &analyzer.event_stats().by_source;
        let sum: usize = by_source.values().sum();
        prop_assert_eq!(sum, analyzer.event_stats().total_count);
    }

    #[test]
    fn prop_workstream_total_events(
        counts in proptest::collection::vec(0usize..100, 1..20),
    ) {
        let mut analyzer = StatsAnalyzer::new();
        let expected_total: usize = counts.iter().sum();
        for (i, &c) in counts.iter().enumerate() {
            analyzer.record_workstream(&format!("ws_{}", i), c);
        }
        prop_assert_eq!(analyzer.workstream_stats().total_events, expected_total);
        prop_assert_eq!(analyzer.workstream_stats().total_workstreams, counts.len());
    }

    #[test]
    fn prop_coverage_percentage_bounds(covered in 0usize..1000, total in 1usize..1000) {
        let mut analyzer = StatsAnalyzer::new();
        let covered = covered.min(total);
        analyzer.calculate_coverage(covered, total);
        let pct = analyzer.workstream_stats().coverage_percentage;
        prop_assert!(pct >= 0.0);
        prop_assert!(pct <= 100.0);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_event_stats() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_event("commit", "github");
    analyzer.record_event("commit", "github");
    analyzer.record_event("pr", "github");
    analyzer.record_event("commit", "gitlab");

    assert_eq!(analyzer.event_stats().total_count, 4);
    assert_eq!(analyzer.event_stats().by_type["commit"], 3);
    assert_eq!(analyzer.event_stats().by_type["pr"], 1);
    assert_eq!(analyzer.event_stats().by_source["github"], 3);
    assert_eq!(analyzer.event_stats().by_source["gitlab"], 1);
}

#[test]
fn known_answer_workstream_stats() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_workstream("backend", 10);
    analyzer.record_workstream("frontend", 5);
    analyzer.record_workstream("infra", 3);

    assert_eq!(analyzer.workstream_stats().total_workstreams, 3);
    assert_eq!(analyzer.workstream_stats().total_events, 18);
    assert_eq!(
        analyzer.workstream_stats().events_per_workstream["backend"],
        10
    );
}

#[test]
fn known_answer_coverage() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.calculate_coverage(75, 100);
    assert!((analyzer.workstream_stats().coverage_percentage - 75.0).abs() < f64::EPSILON);

    analyzer.calculate_coverage(1, 3);
    let pct = analyzer.workstream_stats().coverage_percentage;
    assert!((pct - 33.333333333333336).abs() < 1e-10);
}

#[test]
fn known_answer_summary() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_event("commit", "github");
    analyzer.record_workstream("backend", 5);
    analyzer.calculate_coverage(5, 10);

    let summary = analyzer.generate_summary();
    assert_eq!(summary.events.total_count, 1);
    assert_eq!(summary.workstreams.total_workstreams, 1);
    assert!((summary.workstreams.coverage_percentage - 50.0).abs() < f64::EPSILON);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_empty_analyzer() {
    let analyzer = StatsAnalyzer::new();
    assert_eq!(analyzer.event_stats().total_count, 0);
    assert!(analyzer.event_stats().by_type.is_empty());
    assert!(analyzer.event_stats().by_source.is_empty());
    assert_eq!(analyzer.workstream_stats().total_workstreams, 0);
    assert_eq!(analyzer.workstream_stats().total_events, 0);
    assert_eq!(analyzer.workstream_stats().coverage_percentage, 0.0);
}

#[test]
fn edge_coverage_zero_total() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.calculate_coverage(0, 0);
    assert_eq!(analyzer.workstream_stats().coverage_percentage, 0.0);
}

#[test]
fn edge_coverage_full() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.calculate_coverage(100, 100);
    assert!((analyzer.workstream_stats().coverage_percentage - 100.0).abs() < f64::EPSILON);
}

#[test]
fn edge_single_event() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_event("commit", "github");
    assert_eq!(analyzer.event_stats().total_count, 1);
    assert_eq!(analyzer.event_stats().by_type.len(), 1);
    assert_eq!(analyzer.event_stats().by_source.len(), 1);
}

#[test]
fn edge_single_workstream() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_workstream("only", 0);
    assert_eq!(analyzer.workstream_stats().total_workstreams, 1);
    assert_eq!(analyzer.workstream_stats().total_events, 0);
}

#[test]
fn edge_workstream_overwrite_same_name() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_workstream("ws", 10);
    analyzer.record_workstream("ws", 20);
    // Two calls increment total_workstreams by 2 and total_events by 30
    assert_eq!(analyzer.workstream_stats().total_workstreams, 2);
    assert_eq!(analyzer.workstream_stats().total_events, 30);
    // But the hashmap entry gets overwritten
    assert_eq!(analyzer.workstream_stats().events_per_workstream["ws"], 20);
}

#[test]
fn edge_default_impl() {
    let analyzer = StatsAnalyzer::default();
    assert_eq!(analyzer.event_stats().total_count, 0);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn summary_serde_round_trip() {
    let mut analyzer = StatsAnalyzer::new();
    analyzer.record_event("commit", "github");
    analyzer.record_workstream("backend", 5);
    let summary = analyzer.generate_summary();

    let json = serde_json::to_string(&summary).unwrap();
    let s2: StatsSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(s2.events.total_count, 1);
    assert_eq!(s2.workstreams.total_workstreams, 1);
}
