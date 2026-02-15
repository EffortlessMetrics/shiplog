//! BDD-style testing helpers for shiplog.
//!
//! This module provides Given/When/Then style testing infrastructure
//! for behavior-driven development of user workflows.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A BDD scenario context that carries state through Given/When/Then steps.
#[derive(Debug, Default)]
pub struct ScenarioContext {
    /// String values (names, titles, etc.)
    pub strings: HashMap<String, String>,
    /// Numeric values (counts, IDs, etc.)
    pub numbers: HashMap<String, u64>,
    /// Boolean flags
    pub flags: HashMap<String, bool>,
    /// Paths to files/directories
    pub paths: HashMap<String, PathBuf>,
    /// Arbitrary data storage
    pub data: HashMap<String, Vec<u8>>,
}

impl ScenarioContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_string(mut self, key: &str, value: &str) -> Self {
        self.strings.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_number(mut self, key: &str, value: u64) -> Self {
        self.numbers.insert(key.to_string(), value);
        self
    }

    pub fn with_flag(mut self, key: &str, value: bool) -> Self {
        self.flags.insert(key.to_string(), value);
        self
    }

    pub fn with_path(mut self, key: &str, value: impl AsRef<Path>) -> Self {
        self.paths
            .insert(key.to_string(), value.as_ref().to_path_buf());
        self
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        self.strings.get(key).map(|s| s.as_str())
    }

    pub fn number(&self, key: &str) -> Option<u64> {
        self.numbers.get(key).copied()
    }

    pub fn flag(&self, key: &str) -> Option<bool> {
        self.flags.get(key).copied()
    }

    pub fn path(&self, key: &str) -> Option<&Path> {
        self.paths.get(key).map(|p| p.as_path())
    }
}

/// Step definition types for BDD scenarios
pub type GivenStep = fn(&mut ScenarioContext);
pub type WhenStep = fn(&mut ScenarioContext) -> Result<(), String>;
pub type ThenStep = fn(&ScenarioContext) -> Result<(), String>;

/// A BDD scenario with named steps
pub struct Scenario {
    pub name: String,
    pub given_steps: Vec<(&'static str, GivenStep)>,
    pub when_steps: Vec<(&'static str, WhenStep)>,
    pub then_steps: Vec<(&'static str, ThenStep)>,
}

impl Scenario {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            given_steps: Vec::new(),
            when_steps: Vec::new(),
            then_steps: Vec::new(),
        }
    }

    pub fn given(mut self, description: &'static str, step: GivenStep) -> Self {
        self.given_steps.push((description, step));
        self
    }

    pub fn when(mut self, description: &'static str, step: WhenStep) -> Self {
        self.when_steps.push((description, step));
        self
    }

    pub fn then(mut self, description: &'static str, step: ThenStep) -> Self {
        self.then_steps.push((description, step));
        self
    }

    pub fn run(&self) -> Result<(), String> {
        let mut ctx = ScenarioContext::new();

        // Run Given steps
        for (desc, step) in &self.given_steps {
            eprintln!("  Given: {}", desc);
            step(&mut ctx);
        }

        // Run When steps
        for (desc, step) in &self.when_steps {
            eprintln!("  When: {}", desc);
            step(&mut ctx)?;
        }

        // Run Then steps
        for (desc, step) in &self.then_steps {
            eprintln!("  Then: {}", desc);
            step(&ctx)?;
        }

        Ok(())
    }
}

/// Assertion helpers for BDD scenarios
pub mod assertions {
    use std::fmt::Debug;

    pub fn assert_present<T: Debug>(option: Option<T>, name: &str) -> Result<T, String> {
        option.ok_or_else(|| format!("Expected {} to be present, but was None", name))
    }

    pub fn assert_eq<T: Debug + PartialEq>(
        actual: T,
        expected: T,
        name: &str,
    ) -> Result<(), String> {
        if actual != expected {
            Err(format!(
                "Expected {} to be {:?}, but was {:?}",
                name, expected, actual
            ))
        } else {
            Ok(())
        }
    }

    pub fn assert_true(flag: bool, name: &str) -> Result<(), String> {
        if !flag {
            Err(format!("Expected {} to be true, but was false", name))
        } else {
            Ok(())
        }
    }

    pub fn assert_false(flag: bool, name: &str) -> Result<(), String> {
        if flag {
            Err(format!("Expected {} to be false, but was true", name))
        } else {
            Ok(())
        }
    }

    pub fn assert_contains(haystack: &str, needle: &str, name: &str) -> Result<(), String> {
        if !haystack.contains(needle) {
            Err(format!(
                "Expected {} to contain '{}', but it did not. Content: {}",
                name, needle, haystack
            ))
        } else {
            Ok(())
        }
    }

    pub fn assert_not_contains(haystack: &str, needle: &str, name: &str) -> Result<(), String> {
        if haystack.contains(needle) {
            Err(format!(
                "Expected {} NOT to contain '{}', but it did. Content: {}",
                name, needle, haystack
            ))
        } else {
            Ok(())
        }
    }
}

/// Fixture builders for common test scenarios
pub mod builders {
    use chrono::{NaiveDate, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
    use shiplog_schema::event::*;
    use shiplog_schema::workstream::WorkstreamsFile;

    pub struct EventBuilder {
        repo: String,
        number: u64,
        title: String,
        kind: EventKind,
    }

    impl EventBuilder {
        pub fn new(repo: &str, number: u64, title: &str) -> Self {
            Self {
                repo: repo.to_string(),
                number,
                title: title.to_string(),
                kind: EventKind::PullRequest,
            }
        }

        pub fn kind(mut self, kind: EventKind) -> Self {
            self.kind = kind;
            self
        }

        pub fn build(self) -> EventEnvelope {
            let id = EventId::from_parts([
                "github",
                match self.kind {
                    EventKind::PullRequest => "pr",
                    EventKind::Review => "review",
                    EventKind::Manual => "manual",
                },
                &self.repo,
                &self.number.to_string(),
            ]);

            let repo_ref = RepoRef {
                full_name: self.repo.clone(),
                html_url: Some(format!("https://github.com/{}", self.repo)),
                visibility: RepoVisibility::Public,
            };

            let occurred_at = Utc::now();

            match self.kind {
                EventKind::PullRequest => EventEnvelope {
                    id,
                    kind: EventKind::PullRequest,
                    occurred_at,
                    actor: Actor {
                        login: "user".into(),
                        id: None,
                    },
                    repo: repo_ref,
                    payload: EventPayload::PullRequest(PullRequestEvent {
                        number: self.number,
                        title: self.title,
                        state: PullRequestState::Merged,
                        created_at: occurred_at,
                        merged_at: Some(occurred_at),
                        additions: Some(100),
                        deletions: Some(20),
                        changed_files: Some(5),
                        touched_paths_hint: vec![],
                        window: None,
                    }),
                    tags: vec![],
                    links: vec![Link {
                        label: "pr".into(),
                        url: format!("https://github.com/{}/pull/{}", self.repo, self.number),
                    }],
                    source: SourceRef {
                        system: SourceSystem::Github,
                        url: None,
                        opaque_id: None,
                    },
                },
                _ => panic!("Builder not implemented for {:?}", self.kind),
            }
        }
    }

    pub struct CoverageBuilder {
        user: String,
        since: NaiveDate,
        until: NaiveDate,
        completeness: Completeness,
    }

    impl CoverageBuilder {
        pub fn new(user: &str) -> Self {
            Self {
                user: user.to_string(),
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                completeness: Completeness::Complete,
            }
        }

        pub fn dates(mut self, since: NaiveDate, until: NaiveDate) -> Self {
            self.since = since;
            self.until = until;
            self
        }

        pub fn completeness(mut self, c: Completeness) -> Self {
            self.completeness = c;
            self
        }

        pub fn build(self) -> CoverageManifest {
            CoverageManifest {
                run_id: shiplog_ids::RunId::now("test"),
                generated_at: Utc::now(),
                user: self.user,
                window: TimeWindow {
                    since: self.since,
                    until: self.until,
                },
                mode: "merged".to_string(),
                sources: vec!["github".to_string()],
                slices: vec![],
                warnings: vec![],
                completeness: self.completeness,
            }
        }
    }

    pub struct WorkstreamsBuilder {
        version: u32,
    }

    impl Default for WorkstreamsBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl WorkstreamsBuilder {
        pub fn new() -> Self {
            Self { version: 1 }
        }

        pub fn build(self) -> WorkstreamsFile {
            WorkstreamsFile {
                workstreams: vec![],
                version: self.version,
                generated_at: Utc::now(),
            }
        }

        pub fn with_workstream(self, _title: &str) -> Self {
            // This is a simplified version - in real usage you'd add to workstreams
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::assertions::*;
    use super::{Scenario, ScenarioContext};

    #[test]
    fn bdd_scenario_example() {
        let scenario = Scenario::new("User collects and renders a packet")
            .given("a GitHub user with recent PRs", |ctx| {
                ctx.strings
                    .insert("username".to_string(), "testuser".to_string());
                ctx.strings
                    .insert("repo".to_string(), "test/repo".to_string());
            })
            .when("they collect events for the past month", |ctx| {
                // In real tests, this would call the actual collect logic
                ctx.numbers.insert("event_count".to_string(), 5);
                Ok(())
            })
            .then(
                "the packet should contain their PRs",
                |ctx: &ScenarioContext| {
                    let count = assert_present(ctx.number("event_count"), "event_count")?;
                    assert_true(count > 0, "event count > 0")
                },
            );

        scenario.run().expect("scenario should pass");
    }
}
