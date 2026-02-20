//! Event transformation and enrichment pipeline for shiplog.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A raw event that needs transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    /// Event type
    pub event_type: String,
    /// Raw payload
    pub payload: serde_json::Value,
    /// Source identifier
    pub source: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A transformed/enriched event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedEvent {
    /// Normalized event type
    pub event_type: String,
    /// Enriched payload
    pub payload: serde_json::Value,
    /// Normalized timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Workstream ID
    pub workstream_id: Option<String>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Enriched metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Transform rule type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransformRuleType {
    /// Rename event type
    Rename,
    /// Add field
    AddField,
    /// Remove field
    RemoveField,
    /// Map field values
    MapValues,
    /// Custom transformation
    Custom,
}

/// A transformation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRule {
    /// Rule type
    pub rule_type: TransformRuleType,
    /// Source field (for field operations)
    pub source_field: Option<String>,
    /// Target field
    pub target_field: Option<String>,
    /// Value mappings
    #[serde(default)]
    pub mappings: HashMap<String, String>,
    /// Static value to add
    pub static_value: Option<serde_json::Value>,
}

/// Transform pipeline
pub struct TransformPipeline {
    rules: Vec<TransformRule>,
}

impl TransformPipeline {
    /// Create a new pipeline with the given rules
    pub fn new(rules: Vec<TransformRule>) -> Self {
        Self { rules }
    }
    
    /// Transform a raw event through the pipeline
    pub fn transform(&self, raw: RawEvent) -> EnrichedEvent {
        let mut payload = raw.payload;
        let tags = Vec::new();
        let metadata = raw.metadata;
        
        for rule in &self.rules {
            match rule.rule_type {
                TransformRuleType::Rename => {
                    // Rename is handled by setting event_type
                }
                TransformRuleType::AddField => {
                    if let (Some(field), Some(value)) = (&rule.target_field, &rule.static_value) {
                        payload[field] = value.clone();
                    }
                }
                TransformRuleType::RemoveField => {
                    if let Some(field) = &rule.source_field {
                        payload.as_object_mut().and_then(|obj| obj.remove(field));
                    }
                }
                TransformRuleType::MapValues => {
                    if let Some(field) = &rule.source_field {
                        if let Some(value) = payload.get(field).and_then(|v| v.as_str()) {
                            if let Some(mapped) = rule.mappings.get(value) {
                                if let Some(obj) = payload.as_object_mut() {
                                    obj.insert(field.clone(), serde_json::Value::String(mapped.clone()));
                                }
                            }
                        }
                    }
                }
                TransformRuleType::Custom => {
                    // Custom transformations would go here
                }
            }
        }
        
        // Parse timestamp
        let timestamp = chrono::DateTime::parse_from_rfc3339(&raw.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        
        EnrichedEvent {
            event_type: raw.event_type,
            payload,
            timestamp,
            workstream_id: None,
            tags,
            metadata,
        }
    }
}

/// Add a tag to an enriched event
pub fn add_tag(event: &mut EnrichedEvent, tag: impl Into<String>) {
    let tag = tag.into();
    if !event.tags.contains(&tag) {
        event.tags.push(tag);
    }
}

/// Add metadata to an enriched event
pub fn add_metadata(event: &mut EnrichedEvent, key: impl Into<String>, value: serde_json::Value) {
    event.metadata.insert(key.into(), value);
}

/// Enrich event with workstream ID
pub fn enrich_with_workstream(event: &mut EnrichedEvent, workstream_id: impl Into<String>) {
    event.workstream_id = Some(workstream_id.into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn transform_pipeline_basic() {
        let rules = vec![
            TransformRule {
                rule_type: TransformRuleType::AddField,
                source_field: None,
                target_field: Some("processed".to_string()),
                mappings: HashMap::new(),
                static_value: Some(serde_json::Value::Bool(true)),
            },
        ];
        
        let pipeline = TransformPipeline::new(rules);
        
        let raw = RawEvent {
            event_type: "test".to_string(),
            payload: serde_json::json!({"id": "123"}),
            source: "test".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
        };
        
        let enriched = pipeline.transform(raw);
        
        assert_eq!(enriched.event_type, "test");
        assert!(enriched.payload.get("processed").and_then(|v| v.as_bool()).unwrap_or(false));
    }

    #[test]
    fn transform_with_field_mapping() {
        let rules = vec![
            TransformRule {
                rule_type: TransformRuleType::MapValues,
                source_field: Some("status".to_string()),
                target_field: None,
                mappings: [
                    ("open".to_string(), "opened".to_string()),
                    ("closed".to_string(), "completed".to_string()),
                ].into_iter().collect(),
                static_value: None,
            },
        ];
        
        let pipeline = TransformPipeline::new(rules);
        
        let raw = RawEvent {
            event_type: "issue".to_string(),
            payload: serde_json::json!({"status": "open"}),
            source: "github".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
        };
        
        let enriched = pipeline.transform(raw);
        
        assert_eq!(
            enriched.payload.get("status").and_then(|v| v.as_str()),
            Some("opened")
        );
    }

    #[test]
    fn add_tag_to_event() {
        let mut event = EnrichedEvent {
            event_type: "test".to_string(),
            payload: serde_json::Value::Object(Default::default()),
            timestamp: chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            workstream_id: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };
        
        add_tag(&mut event, "important");
        add_tag(&mut event, "review");
        add_tag(&mut event, "important"); // Duplicate should be ignored
        
        assert_eq!(event.tags.len(), 2);
        assert!(event.tags.contains(&"important".to_string()));
    }

    #[test]
    fn add_metadata_to_event() {
        let mut event = EnrichedEvent {
            event_type: "test".to_string(),
            payload: serde_json::Value::Object(Default::default()),
            timestamp: chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            workstream_id: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };
        
        add_metadata(&mut event, "priority", serde_json::json!("high"));
        add_metadata(&mut event, "count", serde_json::json!(42));
        
        assert_eq!(event.metadata.get("priority").and_then(|v| v.as_str()), Some("high"));
        assert_eq!(event.metadata.get("count").and_then(|v| v.as_i64()), Some(42));
    }

    #[test]
    fn enrich_with_workstream_test() {
        let mut event = EnrichedEvent {
            event_type: "test".to_string(),
            payload: serde_json::Value::Object(Default::default()),
            timestamp: chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            workstream_id: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };
        
        enrich_with_workstream(&mut event, "my-workstream");
        
        assert_eq!(event.workstream_id, Some("my-workstream".to_string()));
    }

    #[test]
    fn transform_remove_field() {
        let rules = vec![
            TransformRule {
                rule_type: TransformRuleType::RemoveField,
                source_field: Some("temp_field".to_string()),
                target_field: None,
                mappings: HashMap::new(),
                static_value: None,
            },
        ];
        
        let pipeline = TransformPipeline::new(rules);
        
        let raw = RawEvent {
            event_type: "test".to_string(),
            payload: serde_json::json!({"temp_field": "remove me", "keep_field": "keep"}),
            source: "test".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
        };
        
        let enriched = pipeline.transform(raw);
        
        assert!(enriched.payload.get("temp_field").is_none());
        assert!(enriched.payload.get("keep_field").is_some());
    }
}
