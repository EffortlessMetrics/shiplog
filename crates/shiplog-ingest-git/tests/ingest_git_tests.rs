use chrono::NaiveDate;
use shiplog_ingest_git::LocalGitIngestor;

#[test]
fn ingestor_creation() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/fake-repo", since, until);
    assert_eq!(ingestor.since, since);
    assert_eq!(ingestor.until, until);
    assert!(ingestor.author.is_none());
    assert!(!ingestor.include_merges);
}

#[test]
fn ingestor_with_author() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor =
        LocalGitIngestor::new("/tmp/fake", since, until).with_author("alice@example.com");
    assert_eq!(ingestor.author, Some("alice@example.com".to_string()));
}

#[test]
fn ingestor_with_merges() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/fake", since, until).with_merges(true);
    assert!(ingestor.include_merges);
}

#[test]
fn ingestor_chained_builder() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/fake", since, until)
        .with_author("bob@test.com")
        .with_merges(true);
    assert_eq!(ingestor.author, Some("bob@test.com".to_string()));
    assert!(ingestor.include_merges);
}
