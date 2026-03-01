use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use shiplog_ids::{EventId, RunId, WorkstreamId};

fn bench_event_id_from_parts(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_id_from_parts");

    // Typical PR event ID (4 parts)
    group.bench_function("pr_4_parts", |b| {
        b.iter(|| EventId::from_parts(black_box(["github", "pr", "acme/widgets", "42"])));
    });

    // Review event ID (5 parts)
    group.bench_function("review_5_parts", |b| {
        b.iter(|| {
            EventId::from_parts(black_box([
                "github",
                "review",
                "acme/widgets",
                "42",
                "12345",
            ]))
        });
    });

    // Single part (minimal)
    group.bench_function("single_part", |b| {
        b.iter(|| EventId::from_parts(black_box(["abc"])));
    });

    group.finish();
}

fn bench_workstream_id_from_parts(c: &mut Criterion) {
    let mut group = c.benchmark_group("workstream_id_from_parts");

    group.bench_function("repo_based", |b| {
        b.iter(|| WorkstreamId::from_parts(black_box(["repo", "acme/widgets"])));
    });

    group.bench_function("long_repo_name", |b| {
        b.iter(|| {
            WorkstreamId::from_parts(black_box([
                "repo",
                "very-long-organization-name/extremely-descriptive-repository-name",
            ]))
        });
    });

    group.finish();
}

fn bench_run_id_now(c: &mut Criterion) {
    c.bench_function("run_id_now", |b| {
        b.iter(|| RunId::now(black_box("shiplog")));
    });
}

fn bench_batch_event_ids(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_event_ids");

    for count in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &n| {
            b.iter(|| {
                (0..n)
                    .map(|i| EventId::from_parts(["github", "pr", "acme/widgets", &i.to_string()]))
                    .collect::<Vec<_>>()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_event_id_from_parts,
    bench_workstream_id_from_parts,
    bench_run_id_now,
    bench_batch_event_ids,
);
criterion_main!(benches);
