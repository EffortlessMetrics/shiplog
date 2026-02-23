//! BDD tests for shiplog-redaction-profile.

use shiplog_redaction_profile::RedactionProfile;
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_manager_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), "manager".to_string());
}

fn given_unknown_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), "team-only".to_string());
}

fn when_profile_is_parsed(ctx: &mut ScenarioContext) -> Result<(), String> {
    let raw = assert_present(ctx.string("raw_profile"), "raw profile")?;
    let parsed = RedactionProfile::from_profile_str(raw);
    ctx.strings
        .insert("canonical_profile".to_string(), parsed.as_str().to_string());
    Ok(())
}

fn then_canonical_profile_is_manager(ctx: &ScenarioContext) -> Result<(), String> {
    let canonical = assert_present(ctx.string("canonical_profile"), "canonical profile")?;
    assert_eq(canonical, "manager", "canonical profile")
}

fn then_canonical_profile_is_public(ctx: &ScenarioContext) -> Result<(), String> {
    let canonical = assert_present(ctx.string("canonical_profile"), "canonical profile")?;
    assert_eq(canonical, "public", "canonical profile")
}

#[test]
fn bdd_known_profile_stays_specific() {
    let scenario = Scenario::new("Known manager profile remains manager")
        .given("a manager profile string", given_manager_profile_input)
        .when("the redaction profile is parsed", when_profile_is_parsed)
        .then(
            "the canonical profile is manager",
            then_canonical_profile_is_manager,
        );

    scenario.run().expect("BDD scenario should pass");
}

#[test]
fn bdd_unknown_profile_falls_back_to_public() {
    let scenario = Scenario::new("Unknown profile falls back to public")
        .given("an unknown profile string", given_unknown_profile_input)
        .when("the redaction profile is parsed", when_profile_is_parsed)
        .then(
            "the canonical profile is public",
            then_canonical_profile_is_public,
        );

    scenario.run().expect("BDD scenario should pass");
}
