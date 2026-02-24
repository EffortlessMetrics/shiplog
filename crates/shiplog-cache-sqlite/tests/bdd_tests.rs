//! BDD tests for shiplog-cache-sqlite.

use chrono::Duration;
use shiplog_cache_sqlite::ApiCache;
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_cache_inputs(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("cache_key".to_string(), "contract-key".to_string());
    ctx.strings
        .insert("cache_value".to_string(), "contract-value".to_string());
}

fn when_cache_stores_and_loads_value(ctx: &mut ScenarioContext) -> Result<(), String> {
    let key = assert_present(ctx.string("cache_key"), "cache_key")?;
    let value = assert_present(ctx.string("cache_value"), "cache_value")?;

    let cache = ApiCache::open_in_memory().map_err(|err| err.to_string())?;
    cache
        .set_with_ttl(key, &value.to_string(), Duration::seconds(60))
        .map_err(|err| err.to_string())?;

    let reloaded = cache.get::<String>(key).map_err(|err| err.to_string())?;
    let contains = cache.contains(key).map_err(|err| err.to_string())?;
    let stats = cache.stats().map_err(|err| err.to_string())?;

    ctx.data.insert(
        "cache_reloaded".to_string(),
        reloaded.unwrap_or_default().into_bytes(),
    );
    ctx.flags.insert("contains".to_string(), contains);
    ctx.numbers
        .insert("total_entries".to_string(), stats.total_entries as u64);
    ctx.numbers
        .insert("valid_entries".to_string(), stats.valid_entries as u64);
    ctx.numbers
        .insert("expired_entries".to_string(), stats.expired_entries as u64);
    Ok(())
}

fn then_cache_contract_remains_stable(ctx: &ScenarioContext) -> Result<(), String> {
    let expected = assert_present(ctx.string("cache_value"), "cache_value")?;
    let reloaded = assert_present(
        ctx.data
            .get("cache_reloaded")
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
        "cache_reloaded",
    )?;

    assert_eq(reloaded, expected.to_string(), "stored and loaded value")?;
    assert_true(
        assert_present(ctx.flag("contains"), "cache_contains")?,
        "cache contains key after write",
    )?;
    assert_true(
        assert_present(ctx.number("total_entries"), "total_entries")? > 0,
        "at least one cache entry stored",
    )?;
    assert_eq(
        assert_present(ctx.number("expired_entries"), "expired_entries")?,
        0,
        "stored entries are initially not expired",
    )?;
    assert_eq(
        assert_present(ctx.number("valid_entries"), "valid_entries")?,
        1,
        "stored entry is valid",
    )
}

#[test]
fn bdd_cache_sqlite_contract_is_stable() {
    let scenario =
        Scenario::new("SQLite cache storage keeps persistence and lookup contracts stable")
            .given("a cache key and value", given_cache_inputs)
            .when(
                "the cache stores and loads the value",
                when_cache_stores_and_loads_value,
            )
            .then(
                "the contract properties remain stable",
                then_cache_contract_remains_stable,
            );

    scenario.run().expect("BDD scenario should pass");
}
