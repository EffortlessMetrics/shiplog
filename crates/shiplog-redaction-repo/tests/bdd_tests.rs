//! BDD tests for shiplog-redaction-repo.

use shiplog_redaction_repo::redact_repo_public;
use shiplog_schema::event::{RepoRef, RepoVisibility};
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_private_repo(ctx: &mut ScenarioContext) {
    ctx.strings
        .insert("full_name".to_string(), "acme/private-repo".to_string());
    ctx.strings.insert(
        "html_url".to_string(),
        "https://github.com/acme/private-repo".to_string(),
    );
}

fn when_public_repo_redaction_is_applied(ctx: &mut ScenarioContext) -> Result<(), String> {
    let full_name = assert_present(ctx.string("full_name"), "full_name")?;
    let html_url = assert_present(ctx.string("html_url"), "html_url")?;

    let repo = RepoRef {
        full_name: full_name.to_string(),
        html_url: Some(html_url.to_string()),
        visibility: RepoVisibility::Private,
    };
    let alias = |kind: &str, value: &str| format!("{kind}:{}", value.replace('/', "_"));
    let redacted = redact_repo_public(&repo, &alias);

    ctx.strings
        .insert("aliased_name".to_string(), redacted.full_name);
    ctx.flags
        .insert("html_url_removed".to_string(), redacted.html_url.is_none());
    ctx.strings.insert(
        "visibility".to_string(),
        format!("{:?}", redacted.visibility),
    );
    Ok(())
}

fn then_public_repo_contract_is_preserved(ctx: &ScenarioContext) -> Result<(), String> {
    let aliased_name = assert_present(ctx.string("aliased_name"), "aliased_name")?;
    let visibility = assert_present(ctx.string("visibility"), "visibility")?;

    assert_eq(aliased_name, "repo:acme_private-repo", "aliased full name")?;
    assert_true(
        ctx.flag("html_url_removed").unwrap_or(false),
        "html_url removed",
    )?;
    assert_eq(visibility, "Unknown", "visibility reset to unknown")
}

#[test]
fn bdd_public_repo_redaction_contract_is_stable() {
    let scenario = Scenario::new("Public repo redaction keeps canonical contract")
        .given("a private repository reference", given_private_repo)
        .when(
            "public repository redaction is applied",
            when_public_repo_redaction_is_applied,
        )
        .then(
            "full name is aliased and sensitive fields are removed",
            then_public_repo_contract_is_preserved,
        );

    scenario.run().expect("BDD scenario should pass");
}
