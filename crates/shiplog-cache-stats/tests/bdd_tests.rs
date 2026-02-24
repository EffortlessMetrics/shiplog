//! BDD tests for shiplog-cache-stats.

use shiplog_cache_stats::CacheStats;
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_raw_storage_counts(ctx: &mut ScenarioContext) {
    ctx.numbers.insert("normal_total".to_string(), 12);
    ctx.numbers.insert("normal_expired".to_string(), 3);
    ctx.numbers
        .insert("normal_bytes".to_string(), 4 * 1024 * 1024 + 12);
}

fn when_stats_are_normalized(ctx: &mut ScenarioContext) -> Result<(), String> {
    let total = assert_present(ctx.number("normal_total"), "normal_total")?;
    let expired = assert_present(ctx.number("normal_expired"), "normal_expired")?;
    let bytes = assert_present(ctx.number("normal_bytes"), "normal_bytes")?;

    let normal = CacheStats::from_raw_counts(total as i64, expired as i64, bytes as i64);
    let clamped = CacheStats::from_raw_counts(-4, 99, -10);

    ctx.numbers
        .insert("normalized_total".to_string(), normal.total_entries as u64);
    ctx.numbers.insert(
        "normalized_expired".to_string(),
        normal.expired_entries as u64,
    );
    ctx.numbers
        .insert("normalized_valid".to_string(), normal.valid_entries as u64);
    ctx.numbers
        .insert("normalized_mb".to_string(), normal.cache_size_mb);

    ctx.numbers
        .insert("clamped_total".to_string(), clamped.total_entries as u64);
    ctx.numbers.insert(
        "clamped_expired".to_string(),
        clamped.expired_entries as u64,
    );
    ctx.numbers
        .insert("clamped_valid".to_string(), clamped.valid_entries as u64);
    ctx.numbers
        .insert("clamped_mb".to_string(), clamped.cache_size_mb);
    Ok(())
}

fn then_stats_follow_the_contract(ctx: &ScenarioContext) -> Result<(), String> {
    assert_eq(
        assert_present(ctx.number("normalized_total"), "normalized_total")?,
        12,
        "normalized total entries",
    )?;
    assert_eq(
        assert_present(ctx.number("normalized_expired"), "normalized_expired")?,
        3,
        "normalized expired entries",
    )?;
    assert_eq(
        assert_present(ctx.number("normalized_valid"), "normalized_valid")?,
        9,
        "normalized valid entries",
    )?;
    assert_eq(
        assert_present(ctx.number("normalized_mb"), "normalized_mb")?,
        4,
        "normalized cache size MB",
    )?;

    assert_eq(
        assert_present(ctx.number("clamped_total"), "clamped_total")?,
        0,
        "clamped total entries",
    )?;
    assert_eq(
        assert_present(ctx.number("clamped_expired"), "clamped_expired")?,
        0,
        "clamped expired entries",
    )?;
    assert_eq(
        assert_present(ctx.number("clamped_valid"), "clamped_valid")?,
        0,
        "clamped valid entries",
    )?;
    assert_eq(
        assert_present(ctx.number("clamped_mb"), "clamped_mb")?,
        0,
        "clamped cache size MB",
    )?;

    let total = assert_present(ctx.number("normalized_total"), "normalized_total")?;
    let expired = assert_present(ctx.number("normalized_expired"), "normalized_expired")?;
    let valid = assert_present(ctx.number("normalized_valid"), "normalized_valid")?;
    assert_true(valid + expired == total, "valid + expired == total")
}

#[test]
fn bdd_cache_stats_contract_is_stable() {
    let scenario = Scenario::new("Cache stats normalization keeps canonical invariants")
        .given(
            "raw storage rows contain counts and bytes",
            given_raw_storage_counts,
        )
        .when("cache stats are normalized", when_stats_are_normalized)
        .then(
            "normalized outputs should match the stable contract",
            then_stats_follow_the_contract,
        );

    scenario.run().expect("BDD scenario should pass");
}
