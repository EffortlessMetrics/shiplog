//! Property tests for shiplog-receipt.

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use shiplog_ids::EventId;
use shiplog_receipt::format_receipt_markdown;
use shiplog_schema::event::*;

fn build_pr_event(repo: &str, title: &str, number: u64, with_link: bool) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["pr", "prop", &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 3, 10, 0, 0).unwrap(),
        actor: Actor {
            login: "bot".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: Utc.with_ymd_and_hms(2025, 1, 3, 9, 0, 0).unwrap(),
            merged_at: Some(Utc.with_ymd_and_hms(2025, 1, 3, 10, 0, 0).unwrap()),
            additions: Some(1),
            deletions: Some(1),
            changed_files: Some(1),
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: if with_link {
            vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/{repo}/pull/{number}"),
            }]
        } else {
            vec![]
        },
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

proptest! {
    #[test]
    fn prop_pr_receipts_have_stable_prefix_and_date(
        owner in "[a-z0-9_-]{1,16}",
        repo in "[a-z0-9_-]{1,16}",
        title in ".{0,80}",
        number in 1u64..10_000,
        with_link in any::<bool>(),
    ) {
        let full_repo = format!("{owner}/{repo}");
        let event = build_pr_event(&full_repo, &title, number, with_link);
        let line = format_receipt_markdown(&event);

        prop_assert!(line.starts_with("- [PR] "));
        prop_assert!(line.contains("(2025-01-03)"));
        prop_assert!(line.contains(&full_repo));

        if with_link {
            prop_assert!(line.contains("https://github.com/"));
            prop_assert!(line.contains("/pull/"));
        }
    }

    #[test]
    fn prop_receipt_is_single_line_for_pr_titles(
        title in "[^\\r\\n]{0,200}",
    ) {
        let event = build_pr_event("acme/service", &title, 7, true);
        let line = format_receipt_markdown(&event);
        prop_assert!(!line.contains('\n'));
        prop_assert!(!line.contains('\r'));
    }
}
