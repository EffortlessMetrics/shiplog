//! Test utilities for shiplog crates.

use anyhow::Context;
use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_ports::Renderer;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::coverage::TimeWindow;
use shiplog_schema::event::*;
use shiplog_schema::workstream::WorkstreamsFile;

pub mod bdd;
pub mod bdd_scenarios;
pub mod proptest;

// BDD scenario modules
pub mod scenarios {
    pub mod later;
    pub mod user_workflows;
    pub mod v02x;
    pub mod v03x;
}

/// Parse newline-delimited JSON event envelopes for lower-level fixture tests.
///
/// The product JSON adapter lives in the `shiplog` package. Lower-level crates
/// use this dev-only helper so their tests do not depend back on the CLI crate.
pub fn parse_events_jsonl_fixture(text: &str, source: &str) -> anyhow::Result<Vec<EventEnvelope>> {
    let mut events = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        events.push(
            serde_json::from_str(line)
                .with_context(|| format!("parse event json line {} in {source}", index + 1))?,
        );
    }
    Ok(events)
}

/// Minimal Markdown renderer for engine and workflow tests.
///
/// Product Markdown lives in the `shiplog` package. The testkit keeps this
/// small renderer so lower-level engine tests do not depend on the CLI package
/// just to prove pipeline orchestration.
#[derive(Debug, Default)]
pub struct TestMarkdownRenderer;

impl TestMarkdownRenderer {
    /// Create a test renderer.
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for TestMarkdownRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> anyhow::Result<String> {
        let mut out = String::new();
        out.push_str(&format!("# Test Packet for {user}\n\n"));
        out.push_str(&format!("Window: {window_label}\n"));
        out.push_str(&format!("Coverage: {:?}\n\n", coverage.completeness));
        out.push_str("## Workstreams\n");
        for workstream in &workstreams.workstreams {
            out.push_str(&format!("- {}\n", workstream.title));
        }
        out.push_str("\n## Events\n");
        for event in events {
            let title = match &event.payload {
                EventPayload::PullRequest(pr) => &pr.title,
                EventPayload::Review(review) => &review.pull_title,
                EventPayload::Manual(manual) => &manual.title,
            };
            out.push_str(&format!(
                "- {:?}: {} from {:?} - {}\n",
                event.kind, event.repo.full_name, event.source.system, title
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod bdd_tests;

#[cfg(test)]
mod bdd_v03x_tests;

#[cfg(test)]
mod bdd_later_tests;

#[cfg(test)]
mod bdd_user_workflow_tests;

/// Small helpers for building fixtures in tests.
///
/// Keeping these in the dev-only testkit avoids copy-paste across
/// render/cluster/redact tests.
pub fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
            additions: Some(1),
            deletions: Some(0),
            changed_files: Some(1),
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/...".into()),
            opaque_id: None,
        },
    }
}

/// Fixture builders for comprehensive test scenarios
pub mod fixtures {
    use super::*;
    use chrono::NaiveDate;
    use shiplog_ids::WorkstreamId;
    use shiplog_schema::coverage::{Completeness, CoverageManifest};
    use shiplog_schema::workstream::{Workstream, WorkstreamStats};

    /// Builder for creating test workstreams
    pub struct WorkstreamFixture {
        title: String,
        summary: Option<String>,
        tags: Vec<String>,
        events: Vec<EventId>,
        receipts: Vec<EventId>,
    }

    impl WorkstreamFixture {
        pub fn new(title: impl Into<String>) -> Self {
            Self {
                title: title.into(),
                summary: None,
                tags: vec![],
                events: vec![],
                receipts: vec![],
            }
        }

        pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
            self.summary = Some(summary.into());
            self
        }

        pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
            self.tags.push(tag.into());
            self
        }

        pub fn with_event(mut self, event: &EventEnvelope) -> Self {
            self.events.push(event.id.clone());
            self
        }

        pub fn with_receipt(mut self, event: &EventEnvelope) -> Self {
            self.receipts.push(event.id.clone());
            self
        }

        pub fn build(self) -> Workstream {
            Workstream {
                id: WorkstreamId::from_parts(["ws", &self.title.to_lowercase().replace(" ", "-")]),
                title: self.title,
                summary: self.summary,
                tags: self.tags,
                stats: WorkstreamStats {
                    pull_requests: self.events.len(),
                    reviews: 0,
                    manual_events: 0,
                },
                events: self.events,
                receipts: self.receipts,
            }
        }
    }

    /// Creates a realistic quarter's worth of PR events
    pub fn realistic_quarter_events(user: &str, repo: &str) -> Vec<EventEnvelope> {
        vec![
            // Feature work
            pr_event(
                repo,
                101,
                &format!("Add user authentication flow - {}", user),
            ),
            pr_event(repo, 102, "Implement OAuth2 integration"),
            pr_event(repo, 103, "Add session management"),
            // Performance work
            pr_event(repo, 104, "Optimize database queries"),
            pr_event(repo, 105, "Add caching layer"),
            // Infrastructure
            pr_event(repo, 106, "Setup CI/CD pipeline"),
            pr_event(repo, 107, "Add monitoring and alerting"),
        ]
    }

    /// Creates a coverage manifest for testing
    pub fn test_coverage(user: &str, completeness: Completeness) -> CoverageManifest {
        CoverageManifest {
            run_id: shiplog_ids::RunId::now("test"),
            generated_at: Utc::now(),
            user: user.to_string(),
            window: shiplog_schema::coverage::TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            },
            mode: "merged".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![],
            warnings: vec![],
            completeness,
        }
    }
}
