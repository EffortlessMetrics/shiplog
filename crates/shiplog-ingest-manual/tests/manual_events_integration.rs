//! Integration test proving manual ingest delegates to the extracted microcrate contract.

use chrono::NaiveDate;
use shiplog_ingest_manual::{ManualIngestor, create_empty_file, create_entry, write_manual_events};
use shiplog_ports::Ingestor;
use tempfile::tempdir;

#[test]
fn ingest_manual_events_uses_microcrate_contract() {
    let temp = tempdir().expect("temporary directory");
    let path = temp.path().join("manual_events.yaml");
    let mut file = create_empty_file();
    file.events.push(create_entry(
        "inside",
        shiplog_schema::event::ManualEventType::Note,
        shiplog_schema::event::ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
        "In window",
    ));
    file.events.push(create_entry(
        "outside",
        shiplog_schema::event::ManualEventType::Incident,
        shiplog_schema::event::ManualDate::Single(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
        "Out of window",
    ));

    write_manual_events(&path, &file).expect("write manual events");
    let ingestor = ManualIngestor::new(
        &path,
        "contract-user".to_string(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    );
    let output = ingestor.ingest().expect("ingest success");

    assert_eq!(output.events.len(), 1);
    assert_eq!(output.events[0].source.opaque_id.as_deref(), Some("inside"));
    assert_eq!(output.coverage.slices[0].total_count, 2);
}
