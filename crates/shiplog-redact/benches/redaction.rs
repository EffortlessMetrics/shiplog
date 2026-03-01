use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;
use shiplog_schema::event::EventEnvelope;

fn sample_events(n: usize) -> Vec<EventEnvelope> {
    (0..n)
        .map(|i| {
            shiplog_testkit::pr_event(
                &format!("org-{}/repo-{}", i % 5, i % 10),
                i as u64,
                &format!("PR title for change #{i}"),
            )
        })
        .collect()
}

fn bench_redact_events_by_profile(c: &mut Criterion) {
    let mut group = c.benchmark_group("redact_events_profile");
    let redactor = DeterministicRedactor::new(b"bench-key-2025");
    let events = sample_events(50);

    for profile in ["internal", "manager", "public"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(profile),
            &profile,
            |b, &prof| {
                b.iter(|| {
                    redactor
                        .redact_events(black_box(&events), black_box(prof))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_redact_events_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("redact_events_scaling");
    let redactor = DeterministicRedactor::new(b"bench-key-2025");

    for count in [10, 50, 200] {
        let events = sample_events(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &events, |b, evts| {
            b.iter(|| {
                redactor
                    .redact_events(black_box(evts), black_box("public"))
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_redact_workstreams(c: &mut Criterion) {
    let redactor = DeterministicRedactor::new(b"bench-key-2025");
    let events = sample_events(30);

    let workstreams_file = shiplog_schema::workstream::WorkstreamsFile {
        version: 1,
        generated_at: chrono::Utc::now(),
        workstreams: (0..5)
            .map(|i| {
                let ws_events: Vec<_> = events
                    .iter()
                    .skip(i * 6)
                    .take(6)
                    .map(|e| e.id.clone())
                    .collect();
                shiplog_schema::workstream::Workstream {
                    id: shiplog_ids::WorkstreamId::from_parts(["repo", &format!("org/repo-{i}")]),
                    title: format!("Workstream {i}"),
                    summary: Some(format!("Summary for workstream {i}")),
                    tags: vec!["infra".into(), "backend".into()],
                    stats: shiplog_schema::workstream::WorkstreamStats {
                        pull_requests: 6,
                        reviews: 0,
                        manual_events: 0,
                    },
                    events: ws_events.clone(),
                    receipts: ws_events,
                }
            })
            .collect(),
    };

    let mut group = c.benchmark_group("redact_workstreams");
    for profile in ["internal", "manager", "public"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(profile),
            &profile,
            |b, &prof| {
                b.iter(|| {
                    redactor
                        .redact_workstreams(black_box(&workstreams_file), black_box(prof))
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bench_redactor_construction(c: &mut Criterion) {
    c.bench_function("redactor_new", |b| {
        b.iter(|| DeterministicRedactor::new(black_box(b"my-secret-key-for-benchmarks")));
    });
}

criterion_group!(
    benches,
    bench_redact_events_by_profile,
    bench_redact_events_scaling,
    bench_redact_workstreams,
    bench_redactor_construction,
);
criterion_main!(benches);
