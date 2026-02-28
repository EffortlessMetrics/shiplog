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

#[test]
fn ingestor_repo_path_preserved() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/my/custom/path", since, until);
    assert_eq!(
        ingestor.repo_path,
        std::path::PathBuf::from("/my/custom/path")
    );
}

#[test]
fn ingestor_defaults_no_merges_no_author() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/repo", since, until);
    assert!(!ingestor.include_merges);
    assert!(ingestor.author.is_none());
}

#[test]
fn ingestor_with_merges_false_explicit() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/fake", since, until).with_merges(false);
    assert!(!ingestor.include_merges);
}

#[test]
fn ingestor_with_author_replaces_previous() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/fake", since, until)
        .with_author("first@example.com")
        .with_author("second@example.com");
    assert_eq!(ingestor.author, Some("second@example.com".to_string()));
}

#[test]
fn ingestor_date_range_preserved() {
    let since = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 9, 20).unwrap();
    let ingestor = LocalGitIngestor::new("/tmp/repo", since, until);
    assert_eq!(ingestor.since, since);
    assert_eq!(ingestor.until, until);
}

#[test]
fn ingestor_builder_order_independent() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

    let a = LocalGitIngestor::new("/tmp/fake", since, until)
        .with_author("alice@test.com")
        .with_merges(true);
    let b = LocalGitIngestor::new("/tmp/fake", since, until)
        .with_merges(true)
        .with_author("alice@test.com");

    assert_eq!(a.author, b.author);
    assert_eq!(a.include_merges, b.include_merges);
    assert_eq!(a.since, b.since);
    assert_eq!(a.until, b.until);
}

#[test]
fn ingestor_path_accepts_pathbuf() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();
    let path = std::path::PathBuf::from("/some/path");
    let ingestor = LocalGitIngestor::new(&path, since, until);
    assert_eq!(ingestor.repo_path, path);
}

#[test]
fn ingestor_with_author_string_and_str_equivalent() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

    let a = LocalGitIngestor::new("/tmp/fake", since, until).with_author("alice@test.com");
    let b = LocalGitIngestor::new("/tmp/fake", since, until)
        .with_author(String::from("alice@test.com"));

    assert_eq!(a.author, b.author);
}
