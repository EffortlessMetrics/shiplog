//! Backward-compatibility tests for shiplog-workstreams.
//!
//! Verifies that workstreams.yaml from older schema versions still
//! deserialises correctly and that new fields have sensible defaults
//! when absent.

use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ── Older YAML formats ──────────────────────────────────────────────────

/// v0.1-era workstreams.yaml: no summary field, minimal stats.
#[test]
fn v01_workstreams_yaml_no_summary() {
    let yaml = r#"
version: 1
generated_at: "2025-01-01T00:00:00Z"
workstreams:
  - id: "sha256:ws-auth"
    title: "Authentication work"
    summary: null
    tags: []
    stats:
      pull_requests: 5
      reviews: 2
      manual_events: 0
    events:
      - "sha256:evt-1"
      - "sha256:evt-2"
    receipts: []
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(wf.version, 1);
    assert_eq!(wf.workstreams.len(), 1);
    let ws = &wf.workstreams[0];
    assert_eq!(ws.title, "Authentication work");
    assert!(ws.summary.is_none());
    assert_eq!(ws.stats.pull_requests, 5);
    assert_eq!(ws.events.len(), 2);
}

/// v0.2-era with summary and tags populated.
#[test]
fn v02_workstreams_yaml_with_summary_and_tags() {
    let yaml = r#"
version: 1
generated_at: "2025-03-15T12:00:00Z"
workstreams:
  - id: "sha256:ws-infra"
    title: "Infrastructure"
    summary: "Kubernetes migration and monitoring"
    tags: ["infra", "k8s"]
    stats:
      pull_requests: 10
      reviews: 4
      manual_events: 2
    events:
      - "sha256:e1"
      - "sha256:e2"
      - "sha256:e3"
    receipts:
      - "sha256:e1"
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    let ws = &wf.workstreams[0];
    assert_eq!(
        ws.summary.as_deref(),
        Some("Kubernetes migration and monitoring")
    );
    assert_eq!(ws.tags, vec!["infra", "k8s"]);
    assert_eq!(ws.receipts.len(), 1);
}

/// Multiple workstreams in a single file.
#[test]
fn multi_workstream_yaml() {
    let yaml = r#"
version: 1
generated_at: "2025-06-01T00:00:00Z"
workstreams:
  - id: "sha256:ws-a"
    title: "Alpha"
    summary: null
    tags: []
    stats: { pull_requests: 1, reviews: 0, manual_events: 0 }
    events: ["sha256:ea"]
    receipts: []
  - id: "sha256:ws-b"
    title: "Beta"
    summary: "B work"
    tags: ["beta"]
    stats: { pull_requests: 0, reviews: 1, manual_events: 1 }
    events: ["sha256:eb"]
    receipts: ["sha256:eb"]
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(wf.workstreams.len(), 2);
    assert_eq!(wf.workstreams[0].title, "Alpha");
    assert_eq!(wf.workstreams[1].title, "Beta");
}

// ── Forward compat: unknown fields ignored ──────────────────────────────

#[test]
fn workstreams_file_ignores_unknown_top_level_fields() {
    let yaml = r#"
version: 1
generated_at: "2025-06-01T00:00:00Z"
schema_version: "0.3.0"
metadata:
  author: "bot"
workstreams: []
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(wf.version, 1);
    assert!(wf.workstreams.is_empty());
}

#[test]
fn workstream_ignores_unknown_fields() {
    let yaml = r#"
id: "sha256:ws-x"
title: "X"
summary: null
tags: []
stats:
  pull_requests: 0
  reviews: 0
  manual_events: 0
events: []
receipts: []
priority: high
risk_score: 0.7
"#;
    let ws: Workstream = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(ws.title, "X");
}

#[test]
fn workstream_stats_ignores_unknown_fields() {
    let json = r#"{"pull_requests": 1, "reviews": 2, "manual_events": 3, "deployments": 5}"#;
    let stats: WorkstreamStats = serde_json::from_str(json).unwrap();
    assert_eq!(stats.pull_requests, 1);
    assert_eq!(stats.manual_events, 3);
}

// ── New fields with sane defaults ───────────────────────────────────────

#[test]
fn workstreams_file_empty_workstreams_list() {
    let yaml = r#"
version: 1
generated_at: "2025-01-01T00:00:00Z"
workstreams: []
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert!(wf.workstreams.is_empty());
}

#[test]
fn workstream_with_empty_tags_events_receipts() {
    let json = r#"{
        "id": "sha256:ws-empty",
        "title": "Empty",
        "summary": null,
        "tags": [],
        "stats": { "pull_requests": 0, "reviews": 0, "manual_events": 0 },
        "events": [],
        "receipts": []
    }"#;
    let ws: Workstream = serde_json::from_str(json).unwrap();
    assert!(ws.tags.is_empty());
    assert!(ws.events.is_empty());
    assert!(ws.receipts.is_empty());
    assert_eq!(ws.stats, WorkstreamStats::zero());
}

// ── JSON round-trip stability ───────────────────────────────────────────

#[test]
fn workstreams_file_json_roundtrip() {
    let yaml = r#"
version: 1
generated_at: "2025-06-01T12:00:00Z"
workstreams:
  - id: "sha256:ws-rt"
    title: "Roundtrip"
    summary: "test"
    tags: ["a"]
    stats: { pull_requests: 1, reviews: 1, manual_events: 1 }
    events: ["sha256:e1"]
    receipts: ["sha256:e1"]
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    let json = serde_json::to_string(&wf).unwrap();
    let back: WorkstreamsFile = serde_json::from_str(&json).unwrap();
    assert_eq!(wf, back);
}
