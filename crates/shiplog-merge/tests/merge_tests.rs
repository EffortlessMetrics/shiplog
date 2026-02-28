use shiplog_merge::*;

#[test]
fn conflict_resolution_default_is_prefer_most_recent() {
    let cr = ConflictResolution::default();
    assert_eq!(cr, ConflictResolution::PreferMostRecent);
}

#[test]
fn merge_strategy_default_is_keep_last() {
    let ms = MergeStrategy::default();
    assert!(matches!(ms, MergeStrategy::KeepLast));
}

#[test]
fn conflict_resolution_to_merge_strategy() {
    let ms: MergeStrategy = ConflictResolution::PreferFirst.into();
    assert!(matches!(ms, MergeStrategy::KeepFirst));
    let ms: MergeStrategy = ConflictResolution::PreferMostRecent.into();
    assert!(matches!(ms, MergeStrategy::KeepLast));
    let ms: MergeStrategy = ConflictResolution::PreferMostComplete.into();
    assert!(matches!(ms, MergeStrategy::KeepMostComplete));
}

#[test]
fn merge_report_fields() {
    let report = MergeReport {
        source_count: 2,
        input_event_count: 10,
        output_event_count: 8,
        conflict_count: 2,
        skipped_events: 0,
        warning_count: 1,
    };
    assert_eq!(report.source_count, 2);
    assert_eq!(report.output_event_count, 8);
    assert_eq!(report.conflict_count, 2);
}
