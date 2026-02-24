//! BDD tests for shiplog-cache-key.

use shiplog_cache_key::CacheKey;
use shiplog_testkit::bdd::assertions::{assert_contains, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_canonical_inputs(ctx: &mut ScenarioContext) {
    ctx.strings.insert(
        "query".to_string(),
        "is:pr is:merged author:octocat".to_string(),
    );
    ctx.strings.insert(
        "pr_url".to_string(),
        "https://api.github.com/repos/octo/repo/pulls/42".to_string(),
    );
    ctx.numbers.insert("project_id".to_string(), 123);
    ctx.numbers.insert("mr_iid".to_string(), 456);
}

fn when_cache_keys_are_generated(ctx: &mut ScenarioContext) -> Result<(), String> {
    let query = assert_present(ctx.string("query"), "query")?;
    let pr_url = assert_present(ctx.string("pr_url"), "pr_url")?;
    let project_id = assert_present(ctx.number("project_id"), "project_id")?;
    let mr_iid = assert_present(ctx.number("mr_iid"), "mr_iid")?;

    let search = CacheKey::search(query, 2, 100);
    let details = CacheKey::pr_details(pr_url);
    let reviews = CacheKey::pr_reviews(pr_url, 2);
    let notes = CacheKey::mr_notes(project_id, mr_iid, 2);

    ctx.strings.insert("search".to_string(), search);
    ctx.strings.insert("details".to_string(), details);
    ctx.strings.insert("reviews".to_string(), reviews);
    ctx.strings.insert("notes".to_string(), notes);
    Ok(())
}

fn then_keys_follow_the_contract(ctx: &ScenarioContext) -> Result<(), String> {
    let search = assert_present(ctx.string("search"), "search")?;
    let details = assert_present(ctx.string("details"), "details")?;
    let reviews = assert_present(ctx.string("reviews"), "reviews")?;
    let notes = assert_present(ctx.string("notes"), "notes")?;

    assert_true(search.starts_with("search:"), "search prefix")?;
    assert_true(search.ends_with(":page2:per100"), "search paging suffix")?;
    assert_contains(details, "pr:details:", "details prefix")?;
    assert_contains(reviews, "pr:reviews:", "reviews prefix")?;
    assert_contains(notes, "gitlab:mr:notes:", "notes prefix")?;
    assert_true(details != reviews, "details and reviews are distinct")?;
    assert_true(search != notes, "search and notes are distinct")
}

#[test]
fn bdd_cache_key_contract_is_stable() {
    let scenario = Scenario::new("Cache key builders keep canonical shapes")
        .given("canonical GitHub and GitLab inputs", given_canonical_inputs)
        .when("cache keys are generated", when_cache_keys_are_generated)
        .then(
            "all generated keys satisfy the contract",
            then_keys_follow_the_contract,
        );

    scenario.run().expect("BDD scenario should pass");
}
