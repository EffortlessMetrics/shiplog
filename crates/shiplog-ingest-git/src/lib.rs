//! Local git repository ingestor.
//!
//! Collects commit history from local git repositories without
//! requiring GitHub API access or authentication.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use git2::{Repository, Time};
use shiplog_ids::{EventId, RunId};
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use std::path::{Path, PathBuf};

/// Local git repository ingestor.
///
/// Collects commit history from a local git repository and converts it
/// into shiplog events.
#[derive(Debug)]
pub struct LocalGitIngestor {
    /// Path to the git repository
    pub repo_path: PathBuf,
    /// Start date for commit collection
    pub since: NaiveDate,
    /// End date for commit collection
    pub until: NaiveDate,
    /// Optional author email filter
    pub author: Option<String>,
    /// Include merge commits in the output
    pub include_merges: bool,
}

impl LocalGitIngestor {
    /// Create a new local git ingestor.
    pub fn new(repo_path: impl AsRef<Path>, since: NaiveDate, until: NaiveDate) -> Self {
        Self {
            repo_path: repo_path.as_ref().to_path_buf(),
            since,
            until,
            author: None,
            include_merges: false,
        }
    }

    /// Filter commits by author email.
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Include merge commits in the output.
    pub fn with_merges(mut self, include: bool) -> Self {
        self.include_merges = include;
        self
    }

    /// Open the git repository.
    fn open_repo(&self) -> Result<Repository> {
        let path = &self.repo_path;
        if !path.exists() {
            return Err(anyhow!("Path does not exist: {}", path.display()));
        }

        Repository::open(path)
            .with_context(|| format!("Failed to open git repository at {}", path.display()))
    }

    /// Get the repository name from the git config.
    #[mutants::skip]
    fn get_repo_name(&self, repo: &Repository) -> Result<String> {
        // Try to get the remote URL and extract the repo name
        if let Ok(remote) = repo.find_remote("origin")
            && let Some(url) = remote.url()
        {
            // Extract repo name from URL like:
            // https://github.com/owner/repo.git
            // git@github.com:owner/repo.git
            if let Some(name) = url.split('/').next_back() {
                return Ok(name.trim_end_matches(".git").to_string());
            }
        }

        // Fallback to directory name
        self.repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Could not determine repository name"))
    }

    /// Convert git2 Time to DateTime<Utc>.
    fn git_time_to_datetime(time: &Time) -> DateTime<Utc> {
        DateTime::from_timestamp(time.seconds(), 0)
            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap())
    }

    /// Check if a commit is within the date range.
    fn is_in_date_range(&self, commit_time: &DateTime<Utc>) -> bool {
        let commit_date = commit_time.date_naive();
        commit_date >= self.since && commit_date <= self.until
    }

    /// Check if a commit matches the author filter.
    fn matches_author(&self, commit: &git2::Commit) -> bool {
        if let Some(ref author_email) = self.author {
            let author = commit.author();
            let email_matches = author
                .email()
                .map(|e| e.to_lowercase() == author_email.to_lowercase())
                .unwrap_or(false);
            let name_matches = author
                .name()
                .map(|n| n.to_lowercase() == author_email.to_lowercase())
                .unwrap_or(false);
            email_matches || name_matches
        } else {
            true
        }
    }

    /// Check if a commit is a merge commit.
    fn is_merge_commit(commit: &git2::Commit) -> bool {
        commit.parent_count() > 1
    }

    /// Create an event envelope from a git commit.
    #[mutants::skip]
    fn commit_to_event(
        &self,
        commit: &git2::Commit,
        repo_name: &str,
        _run_id: &RunId,
    ) -> Result<EventEnvelope> {
        let commit_time = Self::git_time_to_datetime(&commit.time());
        let commit_hash = commit.id().to_string();

        // Extract first line of commit message as title
        let title = commit.summary().unwrap_or("<no message>").to_string();

        let author = commit.author();
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();

        // Create actor (use email as login if name is not available)
        let actor_login = if !author_email.is_empty() {
            author_email.clone()
        } else {
            author_name.clone()
        };

        // Create event ID from commit hash
        let event_id = EventId::from_parts(["local_git", &commit_hash]);

        // Create source reference
        let source = SourceRef {
            system: SourceSystem::LocalGit,
            url: None,
            opaque_id: Some(commit_hash),
        };

        // Create repository reference
        let repo = RepoRef {
            full_name: repo_name.to_string(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        };

        // Create actor
        let actor = Actor {
            login: actor_login,
            id: None,
        };

        // Create pull request event (using commit as proxy for PR)
        // This is a simplification - in a real implementation, we might
        // want to distinguish between regular commits and PR commits
        let payload = EventPayload::PullRequest(PullRequestEvent {
            number: 0, // No PR number for local commits
            title,
            state: PullRequestState::Merged, // All commits are "merged" in this context
            created_at: commit_time,
            merged_at: Some(commit_time),
            additions: None,
            deletions: None,
            changed_files: None,
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: self.since,
                until: self.until,
            }),
        });

        // Create links (could include link to commit if remote URL is available)
        let links = vec![];

        Ok(EventEnvelope {
            id: event_id,
            kind: EventKind::PullRequest,
            occurred_at: commit_time,
            actor,
            repo,
            payload,
            tags: vec![],
            links,
            source,
        })
    }

    /// Collect commits from the repository.
    #[mutants::skip]
    fn collect_commits(&self, repo: &Repository, run_id: &RunId) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();
        let repo_name = self.get_repo_name(repo)?;

        // Get the HEAD reference
        let head = repo.head().context("Failed to get HEAD reference")?;
        let head_commit = head.peel_to_commit().context("Failed to peel to commit")?;

        // Walk the commit history
        let mut revwalk = repo.revwalk().context("Failed to create revwalk")?;
        revwalk
            .push(head_commit.id())
            .context("Failed to push HEAD to revwalk")?;

        for commit_id in revwalk {
            let commit_id = commit_id.context("Failed to get commit id")?;
            let commit = repo
                .find_commit(commit_id)
                .context("Failed to find commit")?;

            let commit_time = Self::git_time_to_datetime(&commit.time());

            // Stop if we've gone past the since date
            if commit_time.date_naive() < self.since {
                break;
            }

            // Check if commit is in date range
            if !self.is_in_date_range(&commit_time) {
                continue;
            }

            // Check author filter
            if !self.matches_author(&commit) {
                continue;
            }

            // Check merge commit filter
            if !self.include_merges && Self::is_merge_commit(&commit) {
                continue;
            }

            // Convert commit to event
            match self.commit_to_event(&commit, &repo_name, run_id) {
                Ok(event) => events.push(event),
                Err(e) => {
                    // Log error but continue processing other commits
                    eprintln!("Warning: Failed to convert commit to event: {}", e);
                }
            }
        }

        // Sort events by timestamp (newest first)
        events.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));

        Ok(events)
    }
}

impl Ingestor for LocalGitIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let repo = self.open_repo()?;
        let run_id = RunId::now("shiplog");

        // Collect commits
        let events = self.collect_commits(&repo, &run_id)?;

        // Create coverage slice
        let coverage_slice = CoverageSlice {
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            query: format!("local_git:{}", self.repo_path.display()),
            total_count: events.len() as u64,
            fetched: events.len() as u64,
            incomplete_results: Some(false),
            notes: vec![],
        };

        // Create coverage manifest
        let coverage = CoverageManifest {
            run_id: run_id.clone(),
            generated_at: Utc::now(),
            user: "local".to_string(),
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            mode: "local".to_string(),
            sources: vec!["local_git".to_string()],
            slices: vec![coverage_slice],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        Ok(IngestOutput { events, coverage })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, Repository)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(dir.path())?;

        // Configure the repo
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        // Create initial commit
        let sig = repo.signature()?;
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        {
            let tree = repo.find_tree(tree_id)?;
            let _oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;
        }

        // Create a second commit
        let tree_id = {
            let oid = repo.head()?.peel_to_commit()?;
            oid.tree_id()
        };
        {
            let tree = repo.find_tree(tree_id)?;
            let _oid = repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Second commit",
                &tree,
                &[&repo.head()?.peel_to_commit()?],
            )?;
        }

        Ok((dir, repo))
    }

    /// Create a repo with commits from multiple authors and a merge commit.
    fn create_multi_author_repo() -> Result<(TempDir, Repository)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(dir.path())?;

        let mut config = repo.config()?;
        config.set_str("user.name", "Alice")?;
        config.set_str("user.email", "alice@example.com")?;

        let alice = Signature::now("Alice", "alice@example.com")?;
        let bob = Signature::now("Bob", "bob@example.com")?;

        // Initial commit by Alice
        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;

        let c1 = {
            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &alice, &alice, "Alice initial", &tree, &[])?
        };

        // Commit by Bob
        let c2 = {
            let tree = repo.find_tree(tree_id)?;
            let c1_commit = repo.find_commit(c1)?;
            repo.commit(
                Some("HEAD"),
                &bob,
                &bob,
                "Bob feature work",
                &tree,
                &[&c1_commit],
            )?
        };

        // Another commit by Alice
        let c3 = {
            let tree = repo.find_tree(tree_id)?;
            let c2_commit = repo.find_commit(c2)?;
            repo.commit(
                Some("HEAD"),
                &alice,
                &alice,
                "Alice second commit",
                &tree,
                &[&c2_commit],
            )?
        };

        // Create a branch for the merge
        let branch_commit = {
            let tree = repo.find_tree(tree_id)?;
            let c3_commit = repo.find_commit(c3)?;
            repo.commit(
                None, // don't update HEAD
                &bob,
                &bob,
                "Bob branch commit",
                &tree,
                &[&c3_commit],
            )?
        };

        // Merge commit (two parents)
        {
            let tree = repo.find_tree(tree_id)?;
            let c3_commit = repo.find_commit(c3)?;
            let branch_commit_obj = repo.find_commit(branch_commit)?;
            let _merge = repo.commit(
                Some("HEAD"),
                &alice,
                &alice,
                "Merge branch into main",
                &tree,
                &[&c3_commit, &branch_commit_obj],
            )?;
        }

        Ok((dir, repo))
    }

    #[test]
    fn test_open_repo() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();
        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        );

        let result = ingestor.open_repo();
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_nonexistent_repo() {
        let ingestor = LocalGitIngestor::new(
            "/nonexistent/path",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        );

        let result = ingestor.open_repo();
        assert!(result.is_err());
    }

    #[test]
    fn test_git_time_to_datetime() {
        let time = Time::new(1704067200, 0); // 2024-01-01 00:00:00 UTC
        let dt = LocalGitIngestor::git_time_to_datetime(&time);
        assert_eq!(dt.timestamp(), 1704067200);
    }

    #[test]
    fn test_is_in_date_range() {
        let ingestor = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
        );

        let inside = DateTime::from_timestamp(1735689600, 0).unwrap(); // 2025-01-01
        let before = DateTime::from_timestamp(1733011200, 0).unwrap(); // 2024-12-01
        let after = DateTime::from_timestamp(1738368000, 0).unwrap(); // 2025-02-01

        assert!(!ingestor.is_in_date_range(&before));
        assert!(ingestor.is_in_date_range(&inside));
        assert!(!ingestor.is_in_date_range(&after));
    }

    #[test]
    fn test_matches_author() {
        let (_dir, repo) = create_test_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        let ingestor = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        )
        .with_author("test@example.com");

        assert!(ingestor.matches_author(&commit));

        let ingestor = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        )
        .with_author("other@example.com");

        assert!(!ingestor.matches_author(&commit));
    }

    #[test]
    fn test_is_merge_commit() {
        let (_dir, repo) = create_test_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        // Regular commit with one parent
        assert!(!LocalGitIngestor::is_merge_commit(&commit));
    }

    #[test]
    fn test_ingest() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();
        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        );

        let result = ingestor.ingest();
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.events.is_empty());
        assert_eq!(output.coverage.slices.len(), 1);
        assert_eq!(output.coverage.sources, vec!["local_git"]);
    }

    #[test]
    fn test_ingest_with_author_filter() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();
        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_author("test@example.com");

        let result = ingestor.ingest();
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.events.is_empty());
    }

    #[test]
    fn test_ingest_invalid_date_range() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();
        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        );

        let result = ingestor.ingest();
        assert!(result.is_err());
    }

    // ── Property tests ──────────────────────────────────────────────────

    proptest! {
        #[test]
        fn git_time_to_datetime_always_valid(secs in 0i64..=4_102_444_800i64) {
            let time = Time::new(secs, 0);
            let dt = LocalGitIngestor::git_time_to_datetime(&time);
            prop_assert_eq!(dt.timestamp(), secs);
        }

        #[test]
        fn git_time_to_datetime_negative_yields_epoch(secs in i64::MIN..0i64) {
            let time = Time::new(secs, 0);
            let dt = LocalGitIngestor::git_time_to_datetime(&time);
            // Negative timestamps either map correctly or fall back to epoch 0
            prop_assert!(dt.timestamp() == secs || dt.timestamp() == 0);
        }

        #[test]
        fn is_in_date_range_boundary_inclusive(
            day_offset in 0u32..365,
        ) {
            let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
            let ingestor = LocalGitIngestor::new("/tmp", since, until);

            let test_date = since + chrono::Duration::days(day_offset as i64);
            // Build a DateTime at midnight UTC on that date
            let dt = test_date.and_hms_opt(0, 0, 0).unwrap().and_utc();

            if test_date >= since && test_date <= until {
                prop_assert!(ingestor.is_in_date_range(&dt));
            } else {
                prop_assert!(!ingestor.is_in_date_range(&dt));
            }
        }

        #[test]
        fn builder_preserves_author(author in "[a-z]+@[a-z]+\\.[a-z]+") {
            let ingestor = LocalGitIngestor::new(
                "/tmp",
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            )
            .with_author(&author);
            prop_assert_eq!(ingestor.author.as_deref(), Some(author.as_str()));
        }

        #[test]
        fn builder_preserves_merges(flag in proptest::bool::ANY) {
            let ingestor = LocalGitIngestor::new(
                "/tmp",
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            )
            .with_merges(flag);
            prop_assert_eq!(ingestor.include_merges, flag);
        }
    }

    // ── Integration tests with fixture repos ────────────────────────────

    #[test]
    fn ingest_author_filter_isolates_single_author() {
        let (_dir, repo) = create_multi_author_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        // Only Alice's commits
        let alice_ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_author("alice@example.com");
        let alice_out = alice_ingestor.ingest().unwrap();

        // Only Bob's commits
        let bob_ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_author("bob@example.com");
        let bob_out = bob_ingestor.ingest().unwrap();

        // All commits (no author filter)
        let all_ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_merges(true);
        let all_out = all_ingestor.ingest().unwrap();

        assert!(!alice_out.events.is_empty());
        assert!(!bob_out.events.is_empty());
        // Author-filtered sets should be subsets of all events
        assert!(alice_out.events.len() + bob_out.events.len() <= all_out.events.len());
    }

    #[test]
    fn author_matching_is_case_insensitive() {
        let (_dir, repo) = create_test_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        let upper = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        )
        .with_author("TEST@EXAMPLE.COM");
        assert!(upper.matches_author(&commit));

        let mixed = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        )
        .with_author("Test@Example.Com");
        assert!(mixed.matches_author(&commit));
    }

    #[test]
    fn author_matching_by_name() {
        let (_dir, repo) = create_test_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        // Match by name instead of email
        let by_name = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        )
        .with_author("Test User");
        assert!(by_name.matches_author(&commit));
    }

    #[test]
    fn no_author_filter_matches_all() {
        let (_dir, repo) = create_test_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        let ingestor = LocalGitIngestor::new(
            "/tmp",
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        );
        // No author set → matches everything
        assert!(ingestor.matches_author(&commit));
    }

    #[test]
    fn merge_commit_detected_in_multi_author_repo() {
        let (_dir, repo) = create_multi_author_repo().unwrap();
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();

        // The HEAD in multi_author_repo is the merge commit
        assert!(LocalGitIngestor::is_merge_commit(&commit));
    }

    #[test]
    fn ingest_excludes_merges_by_default() {
        let (_dir, repo) = create_multi_author_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let no_merge = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        );
        let with_merge = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_merges(true);

        let no_merge_out = no_merge.ingest().unwrap();
        let with_merge_out = with_merge.ingest().unwrap();

        // Including merges should produce at least one more event
        assert!(with_merge_out.events.len() > no_merge_out.events.len());
    }

    #[test]
    fn ingest_narrow_date_range_filters_correctly() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        // Use a date range far in the past — no commits should match
        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2000, 1, 2).unwrap(),
        );
        let output = ingestor.ingest().unwrap();
        assert!(output.events.is_empty());
        assert_eq!(output.coverage.slices[0].total_count, 0);
    }

    #[test]
    fn ingest_nonexistent_author_yields_empty() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        )
        .with_author("nobody@nowhere.com");
        let output = ingestor.ingest().unwrap();
        assert!(output.events.is_empty());
    }

    #[test]
    fn coverage_manifest_populated_correctly() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let since = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();
        let ingestor = LocalGitIngestor::new(&repo_path, since, until);
        let output = ingestor.ingest().unwrap();

        assert_eq!(output.coverage.window.since, since);
        assert_eq!(output.coverage.window.until, until);
        assert_eq!(output.coverage.user, "local");
        assert_eq!(output.coverage.mode, "local");
        assert_eq!(output.coverage.sources, vec!["local_git"]);
        assert_eq!(output.coverage.slices.len(), 1);

        let slice = &output.coverage.slices[0];
        assert_eq!(slice.total_count, slice.fetched);
        assert_eq!(slice.total_count, output.events.len() as u64);
        assert_eq!(slice.incomplete_results, Some(false));
    }

    #[test]
    fn events_sorted_newest_first() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        );
        let output = ingestor.ingest().unwrap();

        for pair in output.events.windows(2) {
            assert!(pair[0].occurred_at >= pair[1].occurred_at);
        }
    }

    #[test]
    fn all_events_have_local_git_source() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let ingestor = LocalGitIngestor::new(
            &repo_path,
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2030, 12, 31).unwrap(),
        );
        let output = ingestor.ingest().unwrap();

        for event in &output.events {
            assert_eq!(event.source.system, SourceSystem::LocalGit);
            assert!(event.source.opaque_id.is_some());
            assert_eq!(event.kind, EventKind::PullRequest);
        }
    }

    // ── Error handling tests ────────────────────────────────────────────

    #[test]
    fn ingest_equal_dates_errors() {
        let (_dir, repo) = create_test_repo().unwrap();
        let repo_path = repo.path().parent().unwrap().to_path_buf();

        let same_date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ingestor = LocalGitIngestor::new(&repo_path, same_date, same_date);
        let err = ingestor.ingest().unwrap_err();
        assert!(err.to_string().contains("since must be < until"));
    }

    #[test]
    fn open_path_exists_but_not_a_repo() {
        let dir = TempDir::new().unwrap();
        let ingestor = LocalGitIngestor::new(
            dir.path(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
        );

        let result = ingestor.open_repo();
        let err = result.err().expect("expected an error");
        assert!(err.to_string().contains("Failed to open git repository"));
    }

    #[test]
    fn git_time_to_datetime_at_epoch() {
        let time = Time::new(0, 0);
        let dt = LocalGitIngestor::git_time_to_datetime(&time);
        assert_eq!(dt.timestamp(), 0);
    }
}
