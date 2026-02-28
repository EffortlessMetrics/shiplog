use proptest::prelude::*;
use shiplog_transform::{
    EnrichedEvent, RawEvent, TransformPipeline, TransformRule, TransformRuleType, add_metadata,
    add_tag, enrich_with_workstream,
};
use std::collections::HashMap;

// ── Helpers ────────────────────────────────────────────────────────

fn make_raw(event_type: &str, payload: serde_json::Value) -> RawEvent {
    RawEvent {
        event_type: event_type.to_string(),
        payload,
        source: "test".to_string(),
        timestamp: "2024-06-15T12:00:00Z".to_string(),
        metadata: HashMap::new(),
    }
}

fn make_enriched() -> EnrichedEvent {
    EnrichedEvent {
        event_type: "test".to_string(),
        payload: serde_json::json!({}),
        timestamp: chrono::DateTime::parse_from_rfc3339("2024-06-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        workstream_id: None,
        tags: Vec::new(),
        metadata: HashMap::new(),
    }
}

// ── Correctness tests ──────────────────────────────────────────────

#[test]
fn empty_pipeline_preserves_event_type() {
    let pipeline = TransformPipeline::new(vec![]);
    let raw = make_raw("pr_merged", serde_json::json!({"id": 1}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.event_type, "pr_merged");
}

#[test]
fn add_field_rule() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::AddField,
        source_field: None,
        target_field: Some("processed".to_string()),
        mappings: HashMap::new(),
        static_value: Some(serde_json::json!(true)),
    }];
    let pipeline = TransformPipeline::new(rules);
    let enriched = pipeline.transform(make_raw("test", serde_json::json!({})));
    assert_eq!(enriched.payload["processed"], true);
}

#[test]
fn remove_field_rule() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::RemoveField,
        source_field: Some("secret".to_string()),
        target_field: None,
        mappings: HashMap::new(),
        static_value: None,
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw(
        "test",
        serde_json::json!({"secret": "s3cr3t", "keep": "yes"}),
    );
    let enriched = pipeline.transform(raw);
    assert!(enriched.payload.get("secret").is_none());
    assert_eq!(enriched.payload["keep"], "yes");
}

#[test]
fn map_values_rule() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::MapValues,
        source_field: Some("status".to_string()),
        target_field: None,
        mappings: [("open".into(), "active".into())].into_iter().collect(),
        static_value: None,
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("issue", serde_json::json!({"status": "open"}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.payload["status"], "active");
}

#[test]
fn map_values_no_match_leaves_unchanged() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::MapValues,
        source_field: Some("status".to_string()),
        target_field: None,
        mappings: [("open".into(), "active".into())].into_iter().collect(),
        static_value: None,
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("issue", serde_json::json!({"status": "closed"}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.payload["status"], "closed");
}

#[test]
fn multiple_rules_compose() {
    let rules = vec![
        TransformRule {
            rule_type: TransformRuleType::AddField,
            source_field: None,
            target_field: Some("enriched".to_string()),
            mappings: HashMap::new(),
            static_value: Some(serde_json::json!(true)),
        },
        TransformRule {
            rule_type: TransformRuleType::RemoveField,
            source_field: Some("temp".to_string()),
            target_field: None,
            mappings: HashMap::new(),
            static_value: None,
        },
    ];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("test", serde_json::json!({"temp": "x", "keep": "y"}));
    let enriched = pipeline.transform(raw);
    assert!(enriched.payload.get("temp").is_none());
    assert_eq!(enriched.payload["enriched"], true);
    assert_eq!(enriched.payload["keep"], "y");
}

#[test]
fn timestamp_parsing() {
    let pipeline = TransformPipeline::new(vec![]);
    let raw = make_raw("test", serde_json::json!({}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.timestamp.to_rfc3339(), "2024-06-15T12:00:00+00:00");
}

#[test]
fn invalid_timestamp_falls_back() {
    let pipeline = TransformPipeline::new(vec![]);
    let mut raw = make_raw("test", serde_json::json!({}));
    raw.timestamp = "not-a-timestamp".to_string();
    let enriched = pipeline.transform(raw);
    // Should not panic; falls back to Utc::now()
    assert!(enriched.timestamp.timestamp() > 0);
}

// ── Edge cases ─────────────────────────────────────────────────────

#[test]
fn add_field_missing_target_is_noop() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::AddField,
        source_field: None,
        target_field: None, // missing
        mappings: HashMap::new(),
        static_value: Some(serde_json::json!(true)),
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("test", serde_json::json!({"a": 1}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.payload, serde_json::json!({"a": 1}));
}

#[test]
fn remove_nonexistent_field_is_noop() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::RemoveField,
        source_field: Some("nonexistent".to_string()),
        target_field: None,
        mappings: HashMap::new(),
        static_value: None,
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("test", serde_json::json!({"a": 1}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.payload, serde_json::json!({"a": 1}));
}

#[test]
fn custom_rule_is_noop() {
    let rules = vec![TransformRule {
        rule_type: TransformRuleType::Custom,
        source_field: None,
        target_field: None,
        mappings: HashMap::new(),
        static_value: None,
    }];
    let pipeline = TransformPipeline::new(rules);
    let raw = make_raw("test", serde_json::json!({"x": 1}));
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.payload, serde_json::json!({"x": 1}));
}

// ── Tag / metadata / workstream helpers ────────────────────────────

#[test]
fn add_tag_deduplicates() {
    let mut event = make_enriched();
    add_tag(&mut event, "a");
    add_tag(&mut event, "b");
    add_tag(&mut event, "a");
    assert_eq!(event.tags, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn add_tag_empty_string() {
    let mut event = make_enriched();
    add_tag(&mut event, "");
    assert_eq!(event.tags.len(), 1);
}

#[test]
fn add_metadata_overwrites() {
    let mut event = make_enriched();
    add_metadata(&mut event, "k", serde_json::json!(1));
    add_metadata(&mut event, "k", serde_json::json!(2));
    assert_eq!(event.metadata["k"], 2);
}

#[test]
fn enrich_with_workstream_sets_id() {
    let mut event = make_enriched();
    assert!(event.workstream_id.is_none());
    enrich_with_workstream(&mut event, "ws-1");
    assert_eq!(event.workstream_id.as_deref(), Some("ws-1"));
}

#[test]
fn enrich_with_workstream_overwrites() {
    let mut event = make_enriched();
    enrich_with_workstream(&mut event, "ws-1");
    enrich_with_workstream(&mut event, "ws-2");
    assert_eq!(event.workstream_id.as_deref(), Some("ws-2"));
}

// ── Metadata preserved through pipeline ────────────────────────────

#[test]
fn metadata_preserved_through_transform() {
    let pipeline = TransformPipeline::new(vec![]);
    let mut raw = make_raw("test", serde_json::json!({}));
    raw.metadata.insert(
        "source_url".into(),
        serde_json::json!("https://example.com"),
    );
    let enriched = pipeline.transform(raw);
    assert_eq!(enriched.metadata["source_url"], "https://example.com");
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_empty_pipeline_preserves_event_type(et in "[a-z_]{1,20}") {
        let pipeline = TransformPipeline::new(vec![]);
        let raw = make_raw(&et, serde_json::json!({}));
        let enriched = pipeline.transform(raw);
        prop_assert_eq!(enriched.event_type, et);
    }

    #[test]
    fn prop_add_tag_idempotent(tag in "[a-z]{1,10}") {
        let mut event = make_enriched();
        add_tag(&mut event, &tag);
        add_tag(&mut event, &tag);
        prop_assert_eq!(event.tags.len(), 1);
    }

    #[test]
    fn prop_tags_never_shrink(tags in prop::collection::vec("[a-z]{1,5}", 0..20)) {
        let mut event = make_enriched();
        let mut prev_len = 0;
        for t in &tags {
            add_tag(&mut event, t);
            prop_assert!(event.tags.len() >= prev_len);
            prev_len = event.tags.len();
        }
    }
}
