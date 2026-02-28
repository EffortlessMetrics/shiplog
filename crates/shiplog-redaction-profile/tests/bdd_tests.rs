//! BDD tests for shiplog-redaction-profile.

use shiplog_redaction_profile::RedactionProfile;
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_manager_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), "manager".to_string());
}

fn given_unknown_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), "team-only".to_string());
}

fn given_internal_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), "internal".to_string());
}

fn given_empty_profile_input(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("raw_profile".to_string(), String::new());
}

fn when_profile_is_parsed(ctx: &mut ScenarioContext) -> Result<(), String> {
    let raw = assert_present(ctx.string("raw_profile"), "raw profile")?;
    let parsed = RedactionProfile::from_profile_str(raw);
    ctx.strings
        .insert("canonical_profile".to_string(), parsed.as_str().to_string());
    Ok(())
}

fn when_profile_is_serialized_and_deserialized(ctx: &mut ScenarioContext) -> Result<(), String> {
    let raw = assert_present(ctx.string("raw_profile"), "raw profile")?;
    let parsed = RedactionProfile::from_profile_str(raw);
    let json = serde_json::to_string(&parsed).map_err(|e| e.to_string())?;
    let decoded: RedactionProfile = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    ctx.strings
        .insert("canonical_profile".to_string(), decoded.as_str().to_string());
    ctx.strings.insert("json_output".to_string(), json);
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

fn then_canonical_profile_is_internal(ctx: &ScenarioContext) -> Result<(), String> {
    let canonical = assert_present(ctx.string("canonical_profile"), "canonical profile")?;
    assert_eq(canonical, "internal", "canonical profile")
}

fn then_json_is_valid(ctx: &ScenarioContext) -> Result<(), String> {
    let json = assert_present(ctx.string("json_output"), "json output")?;
    assert_true(!json.is_empty(), "json output is non-empty")
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

#[test]
fn bdd_internal_profile_is_parsed_correctly() {
    let scenario = Scenario::new("Internal profile is parsed correctly")
        .given("an internal profile string", given_internal_profile_input)
        .when("the redaction profile is parsed", when_profile_is_parsed)
        .then(
            "the canonical profile is internal",
            then_canonical_profile_is_internal,
        );

    scenario.run().expect("BDD scenario should pass");
}

#[test]
fn bdd_empty_profile_falls_back_to_public() {
    let scenario = Scenario::new("Empty profile falls back to public")
        .given("an empty profile string", given_empty_profile_input)
        .when("the redaction profile is parsed", when_profile_is_parsed)
        .then(
            "the canonical profile is public",
            then_canonical_profile_is_public,
        );

    scenario.run().expect("BDD scenario should pass");
}

#[test]
fn bdd_serde_roundtrip_preserves_profile() {
    let scenario = Scenario::new("Serde roundtrip preserves profile identity")
        .given("a manager profile string", given_manager_profile_input)
        .when(
            "the profile is serialized and deserialized",
            when_profile_is_serialized_and_deserialized,
        )
        .then(
            "the canonical profile is manager",
            then_canonical_profile_is_manager,
        )
        .then("the JSON output is valid", then_json_is_valid);

    scenario.run().expect("BDD scenario should pass");
}
