use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::event::*;
use shiplog_schema::coverage::TimeWindow;

/// Small helpers for building fixtures in tests.
///
/// Keeping these in a microcrate avoids copy-paste across render/cluster/redact tests.
pub fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor { login: "user".into(), id: None },
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
