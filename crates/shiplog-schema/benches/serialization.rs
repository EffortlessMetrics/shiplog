use chrono::{NaiveDate, TimeZone, Utc};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_schema::coverage::*;
use shiplog_schema::event::*;
use shiplog_schema::workstream::*;

fn make_pr_event(i: u64) -> EventEnvelope {
    let ts = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", "acme/widgets", &i.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: ts,
        actor: Actor {
            login: "octocat".into(),
            id: Some(1),
        },
        repo: RepoRef {
            full_name: "acme/widgets".into(),
            html_url: Some("https://github.com/acme/widgets".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: i,
            title: format!("Implement feature #{i}"),
            state: PullRequestState::Merged,
            created_at: ts,
            merged_at: Some(ts),
            additions: Some(150),
            deletions: Some(30),
            changed_files: Some(8),
            touched_paths_hint: vec![
                "src/lib.rs".into(),
                "src/api/handlers.rs".into(),
                "tests/integration.rs".into(),
            ],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec!["feature".into(), "backend".into()],
        links: vec![
            Link {
                label: "pr".into(),
                url: format!("https://github.com/acme/widgets/pull/{i}"),
            },
            Link {
                label: "issue".into(),
                url: format!("https://github.com/acme/widgets/issues/{}", i + 100),
            },
        ],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/acme/widgets/pulls/{i}"
            )),
            opaque_id: Some(format!("PR_node_{i}")),
        },
    }
}

fn make_review_event(i: u64) -> EventEnvelope {
    let ts = Utc.with_ymd_and_hms(2025, 2, 10, 9, 30, 0).unwrap();
    EventEnvelope {
        id: EventId::from_parts(["github", "review", "acme/widgets", &i.to_string(), "1"]),
        kind: EventKind::Review,
        occurred_at: ts,
        actor: Actor {
            login: "reviewer".into(),
            id: Some(42),
        },
        repo: RepoRef {
            full_name: "acme/widgets".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: i,
            pull_title: format!("Feature #{i}"),
            submitted_at: ts,
            state: "approved".into(),
            window: None,
        }),
        tags: vec!["review".into()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_events(n: usize) -> Vec<EventEnvelope> {
    (0..n)
        .map(|i| {
            if i % 3 == 0 {
                make_review_event(i as u64)
            } else {
                make_pr_event(i as u64)
            }
        })
        .collect()
}

fn bench_event_serialize_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_serialize_json");

    for count in [1, 10, 100] {
        let events = make_events(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &events, |b, evts| {
            b.iter(|| serde_json::to_string(black_box(evts)).unwrap());
        });
    }

    group.finish();
}

fn bench_event_deserialize_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_deserialize_json");

    for count in [1, 10, 100] {
        let events = make_events(count);
        let json = serde_json::to_string(&events).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(count), &json, |b, data| {
            b.iter(|| serde_json::from_str::<Vec<EventEnvelope>>(black_box(data)).unwrap());
        });
    }

    group.finish();
}

fn bench_event_jsonl_roundtrip(c: &mut Criterion) {
    let events = make_events(50);
    let jsonl: String = events
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n");

    c.bench_function("jsonl_roundtrip_50_events", |b| {
        b.iter(|| {
            let parsed: Vec<EventEnvelope> = black_box(&jsonl)
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(parsed.len(), 50);
        });
    });
}

fn bench_coverage_manifest_serialize(c: &mut Criterion) {
    let manifest = CoverageManifest {
        run_id: RunId("bench_run_123456".into()),
        generated_at: Utc::now(),
        user: "octocat".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: (0..12)
            .map(|i| CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1 + i).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 1, 2 + i).unwrap(),
                },
                query: format!(
                    "author:octocat is:merged created:2025-01-{:02}..2025-01-{:02}",
                    1 + i,
                    2 + i
                ),
                total_count: 10 + i as u64,
                fetched: 10 + i as u64,
                incomplete_results: Some(false),
                notes: vec!["all fetched".into()],
            })
            .collect(),
        warnings: vec![],
        completeness: Completeness::Complete,
    };

    let mut group = c.benchmark_group("coverage_manifest");
    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&manifest)).unwrap());
    });

    let json = serde_json::to_string(&manifest).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<CoverageManifest>(black_box(&json)).unwrap());
    });
    group.finish();
}

fn bench_workstreams_file_serialize(c: &mut Criterion) {
    let file = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: (0..10)
            .map(|i| Workstream {
                id: WorkstreamId::from_parts(["repo", &format!("org/repo-{i}")]),
                title: format!("Workstream {i}"),
                summary: Some(format!("Summary of workstream {i} covering multiple PRs")),
                tags: vec!["backend".into(), "infra".into()],
                stats: WorkstreamStats {
                    pull_requests: 8,
                    reviews: 3,
                    manual_events: 1,
                },
                events: (0..8)
                    .map(|j| {
                        EventId::from_parts([
                            "github",
                            "pr",
                            &format!("org/repo-{i}"),
                            &j.to_string(),
                        ])
                    })
                    .collect(),
                receipts: (0..4)
                    .map(|j| {
                        EventId::from_parts([
                            "github",
                            "pr",
                            &format!("org/repo-{i}"),
                            &j.to_string(),
                        ])
                    })
                    .collect(),
            })
            .collect(),
    };

    let mut group = c.benchmark_group("workstreams_file");
    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&file)).unwrap());
    });

    let json = serde_json::to_string(&file).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<WorkstreamsFile>(black_box(&json)).unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_event_serialize_json,
    bench_event_deserialize_json,
    bench_event_jsonl_roundtrip,
    bench_coverage_manifest_serialize,
    bench_workstreams_file_serialize,
);
criterion_main!(benches);
