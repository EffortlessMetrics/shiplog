//! BDD tests for shiplog-redaction-projector.

use chrono::Utc;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_projector::{project_events_with_aliases, project_workstreams_with_aliases};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_testkit::bdd::assertions::*;
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn bdd_alias(kind: &str, value: &str) -> String {
    let mut acc = 14695981039346656037u64;
    for byte in kind.bytes().chain(value.bytes()) {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(1099511628211);
    }
    format!("{kind}-{acc:016x}")
}

fn given_sensitive_inputs(ctx: &mut ScenarioContext) {
    let events = vec![
        EventEnvelope {
            id: EventId::from_parts(["bdd", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "dev".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/top-secret".into(),
                html_url: Some("https://github.com/acme/top-secret".into()),
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 42,
                title: "Top secret launch plan".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec!["secret/path.rs".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/acme/top-secret/pull/42".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/repos/acme/top-secret/pulls/42".into()),
                opaque_id: None,
            },
        },
        EventEnvelope {
            id: EventId::from_parts(["bdd", "2"]),
            kind: EventKind::Manual,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "dev".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/top-secret".into(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Incident,
                title: "Service outage".into(),
                description: Some("Sensitive root cause details".into()),
                started_at: None,
                ended_at: None,
                impact: Some("Customer-specific data".into()),
            }),
            tags: vec![],
            links: vec![Link {
                label: "incident".into(),
                url: "https://internal/wiki/incident".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: Some("https://internal/api/incidents/1".into()),
                opaque_id: None,
            },
        },
    ];

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "bdd"]),
            title: "Top Secret Program".into(),
            summary: Some("Private architecture summary".into()),
            tags: vec!["security".into(), "repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };

    let events_json = serde_json::to_vec(&events).expect("serialize events");
    let workstreams_json = serde_json::to_vec(&workstreams).expect("serialize workstreams");
    ctx.data.insert("events".to_string(), events_json);
    ctx.data.insert("workstreams".to_string(), workstreams_json);
}

fn given_unknown_profile_string(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("profile".to_string(), "team-only".to_string());
}

fn given_manager_profile_string(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("profile".to_string(), "manager".to_string());
}

fn when_projection_is_applied(ctx: &mut ScenarioContext) -> Result<(), String> {
    let profile = assert_present(ctx.string("profile"), "profile")?;
    let events_bytes = assert_present(ctx.data.get("events"), "events bytes")?;
    let workstreams_bytes = assert_present(ctx.data.get("workstreams"), "workstreams bytes")?;

    let events: Vec<EventEnvelope> =
        serde_json::from_slice(events_bytes).map_err(|e| e.to_string())?;
    let workstreams: WorkstreamsFile =
        serde_json::from_slice(workstreams_bytes).map_err(|e| e.to_string())?;

    let redacted_events = project_events_with_aliases(&events, profile, &bdd_alias);
    let redacted_workstreams = project_workstreams_with_aliases(&workstreams, profile, &bdd_alias);

    let events_json = serde_json::to_string(&redacted_events).map_err(|e| e.to_string())?;
    let workstreams_json =
        serde_json::to_string(&redacted_workstreams).map_err(|e| e.to_string())?;

    ctx.strings.insert("events_json".to_string(), events_json);
    ctx.strings
        .insert("workstreams_json".to_string(), workstreams_json);
    Ok(())
}

fn then_unknown_profile_falls_back_to_public(ctx: &ScenarioContext) -> Result<(), String> {
    let events_json = assert_present(ctx.string("events_json"), "events_json")?;
    let workstreams_json = assert_present(ctx.string("workstreams_json"), "workstreams_json")?;

    assert_not_contains(events_json, "Top secret launch plan", "events json")?;
    assert_not_contains(events_json, "acme/top-secret", "events json")?;
    assert_not_contains(
        events_json,
        "https://github.com/acme/top-secret",
        "events json",
    )?;
    assert_contains(events_json, "[redacted]", "events json")?;

    assert_not_contains(workstreams_json, "Top Secret Program", "workstreams json")?;
    assert_not_contains(
        workstreams_json,
        "Private architecture summary",
        "workstreams json",
    )?;
    assert_not_contains(workstreams_json, "\"repo\"", "workstreams json")
}

fn then_manager_profile_keeps_context_strips_detail(ctx: &ScenarioContext) -> Result<(), String> {
    let events_json = assert_present(ctx.string("events_json"), "events_json")?;
    let workstreams_json = assert_present(ctx.string("workstreams_json"), "workstreams_json")?;

    assert_contains(
        events_json,
        "Top secret launch plan",
        "manager events keeps pr title",
    )?;
    assert_contains(events_json, "acme/top-secret", "manager events keeps repo")?;
    assert_not_contains(
        events_json,
        "Sensitive root cause details",
        "manager events strips description",
    )?;
    assert_not_contains(
        events_json,
        "Customer-specific data",
        "manager events strips impact",
    )?;
    assert_contains(events_json, "\"links\":[]", "manager events strips links")?;

    assert_contains(
        workstreams_json,
        "Top Secret Program",
        "manager workstreams keeps title",
    )?;
    assert_not_contains(
        workstreams_json,
        "Private architecture summary",
        "manager workstreams strips summary",
    )
}

#[test]
fn bdd_unknown_profile_uses_public_projection() {
    let scenario = Scenario::new("Unknown profile uses public projection")
        .given("sensitive fixtures", given_sensitive_inputs)
        .given("an unknown profile string", given_unknown_profile_string)
        .when("projection is applied", when_projection_is_applied)
        .then(
            "sensitive fields are removed and aliases are used",
            then_unknown_profile_falls_back_to_public,
        );

    scenario.run().expect("BDD scenario should pass");
}

#[test]
fn bdd_manager_profile_keeps_context_and_strips_detail() {
    let scenario = Scenario::new("Manager profile keeps context and strips detail")
        .given("sensitive fixtures", given_sensitive_inputs)
        .given("a manager profile string", given_manager_profile_string)
        .when("projection is applied", when_projection_is_applied)
        .then(
            "titles and repos remain while detail fields are removed",
            then_manager_profile_keeps_context_strips_detail,
        );

    scenario.run().expect("BDD scenario should pass");
}

fn given_internal_profile_string(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("profile".to_string(), "internal".to_string());
}

fn then_internal_profile_preserves_all_fields(ctx: &ScenarioContext) -> Result<(), String> {
    let events_json = assert_present(ctx.string("events_json"), "events_json")?;
    let workstreams_json = assert_present(ctx.string("workstreams_json"), "workstreams_json")?;

    // Internal profile preserves everything
    assert_contains(
        events_json,
        "Top secret launch plan",
        "internal events keeps pr title",
    )?;
    assert_contains(
        events_json,
        "acme/top-secret",
        "internal events keeps repo",
    )?;
    assert_contains(
        events_json,
        "Sensitive root cause details",
        "internal events keeps description",
    )?;
    assert_contains(
        events_json,
        "Customer-specific data",
        "internal events keeps impact",
    )?;
    assert_contains(
        events_json,
        "https://github.com/acme/top-secret/pull/42",
        "internal events keeps links",
    )?;

    assert_contains(
        workstreams_json,
        "Top Secret Program",
        "internal workstreams keeps title",
    )?;
    assert_contains(
        workstreams_json,
        "Private architecture summary",
        "internal workstreams keeps summary",
    )
}

#[test]
fn bdd_internal_profile_preserves_all_data() {
    let scenario = Scenario::new("Internal profile preserves all data")
        .given("sensitive fixtures", given_sensitive_inputs)
        .given("an internal profile string", given_internal_profile_string)
        .when("projection is applied", when_projection_is_applied)
        .then(
            "all fields including sensitive data are preserved",
            then_internal_profile_preserves_all_fields,
        );

    scenario.run().expect("BDD scenario should pass");
}
