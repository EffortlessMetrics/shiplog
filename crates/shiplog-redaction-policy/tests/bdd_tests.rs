//! BDD tests for shiplog-redaction-policy.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_policy::{
    RedactionProfile, redact_events_with_aliases, redact_workstreams_with_aliases,
};
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
    let events = vec![EventEnvelope {
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
    }];

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

    let events_json = serde_json::to_vec(&events).expect("serialize events fixture");
    let workstreams_json = serde_json::to_vec(&workstreams).expect("serialize workstreams fixture");
    ctx.data.insert("events".to_string(), events_json);
    ctx.data.insert("workstreams".to_string(), workstreams_json);
}

fn when_public_profile_is_applied(ctx: &mut ScenarioContext) -> Result<(), String> {
    let events_bytes = assert_present(ctx.data.get("events"), "events bytes")?;
    let workstreams_bytes = assert_present(ctx.data.get("workstreams"), "workstreams bytes")?;

    let events: Vec<EventEnvelope> =
        serde_json::from_slice(events_bytes).map_err(|e| e.to_string())?;
    let workstreams: WorkstreamsFile =
        serde_json::from_slice(workstreams_bytes).map_err(|e| e.to_string())?;

    let redacted_events = redact_events_with_aliases(&events, RedactionProfile::Public, &bdd_alias);
    let redacted_workstreams =
        redact_workstreams_with_aliases(&workstreams, RedactionProfile::Public, &bdd_alias);

    let events_json = serde_json::to_string(&redacted_events).map_err(|e| e.to_string())?;
    let workstreams_json =
        serde_json::to_string(&redacted_workstreams).map_err(|e| e.to_string())?;

    ctx.strings.insert("events_json".to_string(), events_json);
    ctx.strings
        .insert("workstreams_json".to_string(), workstreams_json);
    Ok(())
}

fn then_public_projection_removes_sensitive_fields(ctx: &ScenarioContext) -> Result<(), String> {
    let events_json = assert_present(ctx.string("events_json"), "events_json")?;
    let workstreams_json = assert_present(ctx.string("workstreams_json"), "workstreams_json")?;

    assert_not_contains(events_json, "Top secret launch plan", "public events json")?;
    assert_not_contains(events_json, "acme/top-secret", "public events json")?;
    assert_not_contains(
        events_json,
        "https://github.com/acme/top-secret",
        "public events json",
    )?;
    assert_contains(events_json, "[redacted]", "public events json")?;

    assert_not_contains(
        workstreams_json,
        "Top Secret Program",
        "public workstreams json",
    )?;
    assert_not_contains(
        workstreams_json,
        "Private architecture summary",
        "public workstreams json",
    )?;
    assert_not_contains(workstreams_json, "\"repo\"", "public workstreams json")
}

fn when_manager_profile_is_applied(ctx: &mut ScenarioContext) -> Result<(), String> {
    let events_bytes = assert_present(ctx.data.get("events"), "events bytes")?;

    let mut events: Vec<EventEnvelope> =
        serde_json::from_slice(events_bytes).map_err(|e| e.to_string())?;
    events[0].payload = EventPayload::Manual(ManualEvent {
        event_type: ManualEventType::Incident,
        title: "Service outage".into(),
        description: Some("Sensitive root cause details".into()),
        started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
        ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
        impact: Some("Customer-specific data".into()),
    });

    let redacted_events =
        redact_events_with_aliases(&events, RedactionProfile::Manager, &bdd_alias);
    let events_json = serde_json::to_string(&redacted_events).map_err(|e| e.to_string())?;

    ctx.strings
        .insert("manager_events_json".to_string(), events_json);
    Ok(())
}

fn then_manager_projection_keeps_context_but_strips_sensitive_detail(
    ctx: &ScenarioContext,
) -> Result<(), String> {
    let manager_events_json =
        assert_present(ctx.string("manager_events_json"), "manager_events_json")?;

    assert_contains(
        manager_events_json,
        "Service outage",
        "manager events json keeps title",
    )?;
    assert_contains(
        manager_events_json,
        "acme/top-secret",
        "manager events json keeps repo",
    )?;
    assert_not_contains(
        manager_events_json,
        "Sensitive root cause details",
        "manager events json strips description",
    )?;
    assert_not_contains(
        manager_events_json,
        "Customer-specific data",
        "manager events json strips impact",
    )
}

#[test]
fn bdd_public_projection_strips_sensitive_data() {
    let scenario = Scenario::new("Public projection strips sensitive data")
        .given(
            "sensitive event and workstream inputs",
            given_sensitive_inputs,
        )
        .when(
            "public redaction profile is applied",
            when_public_profile_is_applied,
        )
        .then(
            "sensitive fields are removed and aliases are used",
            then_public_projection_removes_sensitive_fields,
        );

    scenario.run().expect("BDD scenario should pass");
}

#[test]
fn bdd_manager_projection_keeps_context_and_strips_sensitive_detail() {
    let scenario = Scenario::new("Manager projection keeps context and strips detail")
        .given("sensitive event input", given_sensitive_inputs)
        .when(
            "manager redaction profile is applied",
            when_manager_profile_is_applied,
        )
        .then(
            "titles and repos remain while details are removed",
            then_manager_projection_keeps_context_but_strips_sensitive_detail,
        );

    scenario.run().expect("BDD scenario should pass");
}

fn when_internal_profile_is_applied(ctx: &mut ScenarioContext) -> Result<(), String> {
    let events_bytes = assert_present(ctx.data.get("events"), "events bytes")?;

    let events: Vec<EventEnvelope> =
        serde_json::from_slice(events_bytes).map_err(|e| e.to_string())?;

    let redacted_events =
        redact_events_with_aliases(&events, RedactionProfile::Internal, &bdd_alias);

    ctx.flags.insert(
        "events_unchanged".to_string(),
        serde_json::to_string(&events).unwrap() == serde_json::to_string(&redacted_events).unwrap(),
    );
    Ok(())
}

fn then_internal_projection_is_identity(ctx: &ScenarioContext) -> Result<(), String> {
    assert_true(
        ctx.flag("events_unchanged").unwrap_or(false),
        "internal profile produces identical events",
    )
}

#[test]
fn bdd_internal_projection_is_identity() {
    let scenario = Scenario::new("Internal projection is identity transform")
        .given("sensitive event input", given_sensitive_inputs)
        .when(
            "internal redaction profile is applied",
            when_internal_profile_is_applied,
        )
        .then("events are unchanged", then_internal_projection_is_identity);

    scenario.run().expect("BDD scenario should pass");
}
