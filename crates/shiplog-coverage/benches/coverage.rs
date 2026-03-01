use chrono::NaiveDate;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use shiplog_coverage::{day_windows, month_windows, week_windows, window_len_days};

fn bench_month_windows(c: &mut Criterion) {
    let mut group = c.benchmark_group("month_windows");

    // Quarter (3 months)
    group.bench_function("quarter", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        b.iter(|| month_windows(black_box(since), black_box(until)));
    });

    // Full year
    group.bench_function("full_year", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        b.iter(|| month_windows(black_box(since), black_box(until)));
    });

    // Multi-year (3 years)
    group.bench_function("three_years", |b| {
        let since = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        b.iter(|| month_windows(black_box(since), black_box(until)));
    });

    // Mid-month start
    group.bench_function("mid_month_start", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 6, 20).unwrap();
        b.iter(|| month_windows(black_box(since), black_box(until)));
    });

    group.finish();
}

fn bench_week_windows(c: &mut Criterion) {
    let mut group = c.benchmark_group("week_windows");

    // One month
    group.bench_function("one_month", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        b.iter(|| week_windows(black_box(since), black_box(until)));
    });

    // Quarter
    group.bench_function("quarter", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        b.iter(|| week_windows(black_box(since), black_box(until)));
    });

    // Full year
    group.bench_function("full_year", |b| {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        b.iter(|| week_windows(black_box(since), black_box(until)));
    });

    group.finish();
}

fn bench_day_windows(c: &mut Criterion) {
    let mut group = c.benchmark_group("day_windows");

    for days in [7, 30, 90, 365] {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = since + chrono::Duration::days(days);
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{days}_days")),
            &(since, until),
            |b, &(s, u)| {
                b.iter(|| day_windows(black_box(s), black_box(u)));
            },
        );
    }

    group.finish();
}

fn bench_window_len_days(c: &mut Criterion) {
    let mut group = c.benchmark_group("window_len_days");

    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    group.bench_function("iterate_quarter_windows", |b| {
        b.iter(|| black_box(&windows).iter().map(window_len_days).sum::<i64>());
    });

    let year_since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let year_until = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let year_days = day_windows(year_since, year_until);

    group.bench_function("iterate_365_day_windows", |b| {
        b.iter(|| {
            black_box(&year_days)
                .iter()
                .map(window_len_days)
                .sum::<i64>()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_month_windows,
    bench_week_windows,
    bench_day_windows,
    bench_window_len_days,
);
criterion_main!(benches);
