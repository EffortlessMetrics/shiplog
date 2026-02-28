//! BDD-style integration tests for the manual event ingestor.

use chrono::{NaiveDate, Utc};
use shiplog_ingest_manual::{ManualIngestor, create_entry, write_manual_events};
use shiplog_ports::Ingestor;
use shiplog_schema::event::{ManualDate, ManualEventType, ManualEventsFile};
use shiplog_testkit::bdd::assertions::{assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

// ---------------------------------------------------------------------------
// Scenario 1: Manual events with valid dates are ingested
// ---------------------------------------------------------------------------

fn given_yaml_with_valid_events(ctx: &mut ScenarioContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let mut file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: Vec::new(),
    };
    file.events.push(create_entry(
        "design-doc",
        ManualEventType::Design,
        ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 10).unwrap()),
        "Auth redesign document",
    ));
    file.events.push(create_entry(
        "incident-1",
        ManualEventType::Incident,
        ManualDate::Range {
            start: NaiveDate::from_ymd_opt(2025, 2, 20).unwrap(),
            end: NaiveDate::from_ymd_opt(2025, 2, 22).unwrap(),
        },
        "Production incident response",
    ));

    write_manual_events(&path, &file).unwrap();

    // Keep the tempdir alive by leaking it into the context.
    // (We store the PathBuf; the tempdir handle is kept as bytes.)
    let raw = Box::into_raw(Box::new(temp));
    ctx.data.insert(
        "_tempdir_handle".to_string(),
        (raw as usize).to_le_bytes().to_vec(),
    );
    ctx.paths
        .insert("events_path".to_string(), path);
    ctx.strings
        .insert("user".to_string(), "alice".to_string());
    ctx.strings
        .insert("since".to_string(), "2025-01-01".to_string());
    ctx.strings
        .insert("until".to_string(), "2025-03-01".to_string());
}

fn when_ingestor_runs(ctx: &mut ScenarioContext) -> Result<(), String> {
    let path = assert_present(ctx.path("events_path"), "events_path")?;
    let user = assert_present(ctx.string("user"), "user")?;
    let since = NaiveDate::parse_from_str(
        &assert_present(ctx.string("since"), "since")?,
        "%Y-%m-%d",
    )
    .map_err(|e| e.to_string())?;
    let until = NaiveDate::parse_from_str(
        &assert_present(ctx.string("until"), "until")?,
        "%Y-%m-%d",
    )
    .map_err(|e| e.to_string())?;

    let ing = ManualIngestor::new(path, user.to_string(), since, until);
    let output = ing.ingest().map_err(|e| e.to_string())?;

    ctx.numbers
        .insert("event_count".to_string(), output.events.len() as u64);
    ctx.numbers.insert(
        "warning_count".to_string(),
        output.coverage.warnings.len() as u64,
    );
    ctx.strings.insert(
        "completeness".to_string(),
        format!("{:?}", output.coverage.completeness),
    );
    // Store first event opaque_id if any
    if let Some(ev) = output.events.first() {
        if let Some(id) = &ev.source.opaque_id {
            ctx.strings
                .insert("first_event_id".to_string(), id.clone());
        }
    }
    Ok(())
}

fn then_all_valid_events_are_ingested(ctx: &ScenarioContext) -> Result<(), String> {
    let count = assert_present(ctx.number("event_count"), "event_count")?;
    assert_eq(count, 2, "event count")?;
    let comp = assert_present(ctx.string("completeness"), "completeness")?;
    assert_eq(comp, "Complete", "completeness")
}

#[test]
fn bdd_valid_events_are_ingested() {
    Scenario::new("Manual events with valid dates are ingested")
        .given(
            "a YAML file with two valid manual events inside the date window",
            given_yaml_with_valid_events,
        )
        .when("the manual ingestor runs", when_ingestor_runs)
        .then(
            "all events are included in the output",
            then_all_valid_events_are_ingested,
        )
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario 2: Events outside date range are filtered
// ---------------------------------------------------------------------------

fn given_yaml_with_mixed_dates(ctx: &mut ScenarioContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![
            create_entry(
                "in-range",
                ManualEventType::Note,
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()),
                "Inside range",
            ),
            create_entry(
                "before-range",
                ManualEventType::Note,
                ManualDate::Single(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()),
                "Before range",
            ),
            create_entry(
                "after-range",
                ManualEventType::Note,
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 12, 1).unwrap()),
                "After range",
            ),
        ],
    };
    write_manual_events(&path, &file).unwrap();

    let raw = Box::into_raw(Box::new(temp));
    ctx.data.insert(
        "_tempdir_handle".to_string(),
        (raw as usize).to_le_bytes().to_vec(),
    );
    ctx.paths.insert("events_path".to_string(), path);
    ctx.strings
        .insert("user".to_string(), "bob".to_string());
    ctx.strings
        .insert("since".to_string(), "2025-01-01".to_string());
    ctx.strings
        .insert("until".to_string(), "2025-06-01".to_string());
}

fn then_only_in_range_event_is_present(ctx: &ScenarioContext) -> Result<(), String> {
    let count = assert_present(ctx.number("event_count"), "event_count")?;
    assert_eq(count, 1, "event count")?;
    let first_id = assert_present(ctx.string("first_event_id"), "first_event_id")?;
    assert_eq(first_id, "in-range", "first event id")
}

#[test]
fn bdd_events_outside_range_are_filtered() {
    Scenario::new("Events outside date range are filtered")
        .given(
            "a YAML file with events both inside and outside the date window",
            given_yaml_with_mixed_dates,
        )
        .when("the manual ingestor runs", when_ingestor_runs)
        .then(
            "only the in-range event is present",
            then_only_in_range_event_is_present,
        )
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario 3: Invalid YAML produces clear error
// ---------------------------------------------------------------------------

fn given_an_invalid_yaml_file(ctx: &mut ScenarioContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "this: is: [not: valid: yaml: for: manual events").unwrap();

    let raw = Box::into_raw(Box::new(temp));
    ctx.data.insert(
        "_tempdir_handle".to_string(),
        (raw as usize).to_le_bytes().to_vec(),
    );
    ctx.paths.insert("events_path".to_string(), path);
    ctx.strings
        .insert("user".to_string(), "carol".to_string());
    ctx.strings
        .insert("since".to_string(), "2025-01-01".to_string());
    ctx.strings
        .insert("until".to_string(), "2025-06-01".to_string());
}

fn when_ingestor_runs_and_captures_error(ctx: &mut ScenarioContext) -> Result<(), String> {
    let path = assert_present(ctx.path("events_path"), "events_path")?;
    let user = assert_present(ctx.string("user"), "user")?;
    let since = NaiveDate::parse_from_str(
        &assert_present(ctx.string("since"), "since")?,
        "%Y-%m-%d",
    )
    .map_err(|e| e.to_string())?;
    let until = NaiveDate::parse_from_str(
        &assert_present(ctx.string("until"), "until")?,
        "%Y-%m-%d",
    )
    .map_err(|e| e.to_string())?;

    let ing = ManualIngestor::new(path, user.to_string(), since, until);
    match ing.ingest() {
        Ok(_) => {
            ctx.flags.insert("errored".to_string(), false);
            ctx.strings
                .insert("error_msg".to_string(), String::new());
        }
        Err(e) => {
            ctx.flags.insert("errored".to_string(), true);
            ctx.strings
                .insert("error_msg".to_string(), format!("{e:#}"));
        }
    }
    Ok(())
}

fn then_a_clear_error_is_returned(ctx: &ScenarioContext) -> Result<(), String> {
    let errored = assert_present(ctx.flag("errored"), "errored")?;
    assert_true(errored, "ingest should have returned an error")?;
    let msg = assert_present(ctx.string("error_msg"), "error_msg")?;
    assert_true(
        msg.contains("parse") || msg.contains("yaml") || msg.contains("manual"),
        "error message should mention parsing or yaml",
    )
}

#[test]
fn bdd_invalid_yaml_produces_clear_error() {
    Scenario::new("Invalid YAML produces clear error")
        .given(
            "a file with invalid YAML content",
            given_an_invalid_yaml_file,
        )
        .when(
            "the manual ingestor attempts to run",
            when_ingestor_runs_and_captures_error,
        )
        .then(
            "a clear parsing error is returned",
            then_a_clear_error_is_returned,
        )
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario 4: Empty event list produces empty result
// ---------------------------------------------------------------------------

fn given_yaml_with_no_events(ctx: &mut ScenarioContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: Vec::new(),
    };
    write_manual_events(&path, &file).unwrap();

    let raw = Box::into_raw(Box::new(temp));
    ctx.data.insert(
        "_tempdir_handle".to_string(),
        (raw as usize).to_le_bytes().to_vec(),
    );
    ctx.paths.insert("events_path".to_string(), path);
    ctx.strings
        .insert("user".to_string(), "dave".to_string());
    ctx.strings
        .insert("since".to_string(), "2025-01-01".to_string());
    ctx.strings
        .insert("until".to_string(), "2025-06-01".to_string());
}

fn then_output_is_empty_with_complete_coverage(ctx: &ScenarioContext) -> Result<(), String> {
    let count = assert_present(ctx.number("event_count"), "event_count")?;
    assert_eq(count, 0, "event count")?;
    let warnings = assert_present(ctx.number("warning_count"), "warning_count")?;
    assert_eq(warnings, 0, "warning count")?;
    let comp = assert_present(ctx.string("completeness"), "completeness")?;
    assert_eq(comp, "Complete", "completeness")
}

#[test]
fn bdd_empty_event_list_produces_empty_result() {
    Scenario::new("Empty event list produces empty result")
        .given(
            "a YAML file with an empty events list",
            given_yaml_with_no_events,
        )
        .when("the manual ingestor runs", when_ingestor_runs)
        .then(
            "the output has zero events and Complete coverage",
            then_output_is_empty_with_complete_coverage,
        )
        .run()
        .expect("scenario should pass");
}
