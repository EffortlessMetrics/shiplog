//! BDD tests for shiplog-cache-expiry.

use chrono::{DateTime, Duration, Utc};
use shiplog_cache_expiry::CacheExpiryWindow;
use shiplog_testkit::bdd::assertions::{assert_false, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn dt(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

fn given_a_cache_window_with_90s_ttl(ctx: &mut ScenarioContext) {
    ctx.numbers.insert("base_secs".to_string(), 1_700_000_000);
    ctx.numbers.insert("ttl_secs".to_string(), 90);
}

fn when_validity_is_checked_at_various_times(ctx: &mut ScenarioContext) -> Result<(), String> {
    let base_secs = assert_present(ctx.number("base_secs"), "base_secs")? as i64;
    let ttl_secs = assert_present(ctx.number("ttl_secs"), "ttl_secs")? as i64;

    let base = dt(base_secs);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(ttl_secs));

    ctx.flags
        .insert("valid_at_base".to_string(), w.is_valid_at(base));
    ctx.flags.insert(
        "valid_before_expiry".to_string(),
        w.is_valid_at(base + Duration::seconds(ttl_secs - 1)),
    );
    ctx.flags.insert(
        "expired_at_boundary".to_string(),
        w.is_expired_at(base + Duration::seconds(ttl_secs)),
    );
    ctx.flags.insert(
        "expired_after".to_string(),
        w.is_expired_at(base + Duration::seconds(ttl_secs + 1)),
    );
    Ok(())
}

fn then_validity_follows_strict_boundary_contract(ctx: &ScenarioContext) -> Result<(), String> {
    let valid_at_base = assert_present(ctx.flag("valid_at_base"), "valid_at_base")?;
    let valid_before = assert_present(ctx.flag("valid_before_expiry"), "valid_before_expiry")?;
    let expired_at = assert_present(ctx.flag("expired_at_boundary"), "expired_at_boundary")?;
    let expired_after = assert_present(ctx.flag("expired_after"), "expired_after")?;

    assert_true(valid_at_base, "should be valid at base time")?;
    assert_true(valid_before, "should be valid 1s before expiry")?;
    assert_true(expired_at, "should be expired at exact boundary")?;
    assert_true(expired_after, "should be expired after boundary")?;
    Ok(())
}

#[test]
fn bdd_cache_expiry_boundary_contract() {
    Scenario::new("Cache expiry window follows strict boundary semantics")
        .given(
            "a cache window with 90-second TTL",
            given_a_cache_window_with_90s_ttl,
        )
        .when(
            "validity is checked at various times",
            when_validity_is_checked_at_various_times,
        )
        .then(
            "validity follows the strict boundary contract",
            then_validity_follows_strict_boundary_contract,
        )
        .run()
        .expect("BDD scenario should pass");
}

// ------------------------------------------------------------------
// Second scenario: zero-TTL window
// ------------------------------------------------------------------

fn given_zero_ttl_window(ctx: &mut ScenarioContext) {
    ctx.numbers.insert("base_secs".to_string(), 1_700_000_000);
    ctx.numbers.insert("ttl_secs".to_string(), 0);
}

fn when_checked_at_base_time(ctx: &mut ScenarioContext) -> Result<(), String> {
    let base_secs = assert_present(ctx.number("base_secs"), "base_secs")? as i64;
    let base = dt(base_secs);
    let w = CacheExpiryWindow::from_base(base, Duration::zero());

    ctx.flags
        .insert("expired_at_base".to_string(), w.is_expired_at(base));
    ctx.flags
        .insert("valid_at_base".to_string(), w.is_valid_at(base));
    Ok(())
}

fn then_zero_ttl_is_immediately_expired(ctx: &ScenarioContext) -> Result<(), String> {
    let expired = assert_present(ctx.flag("expired_at_base"), "expired_at_base")?;
    let valid = assert_present(ctx.flag("valid_at_base"), "valid_at_base")?;
    assert_true(expired, "zero-TTL should be expired at base")?;
    assert_false(valid, "zero-TTL should not be valid at base")
}

#[test]
fn bdd_zero_ttl_is_immediately_expired() {
    Scenario::new("Zero-TTL cache window is immediately expired")
        .given("a window with zero TTL", given_zero_ttl_window)
        .when("checked at the base time", when_checked_at_base_time)
        .then(
            "the window is immediately expired",
            then_zero_ttl_is_immediately_expired,
        )
        .run()
        .expect("BDD scenario should pass");
}
