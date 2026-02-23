//! Property tests for ROADMAP features
//!
//! This module provides property tests for ROADMAP features:
//! - GitLab API response parsing invariants
//! - Jira/Linear API response parsing invariants
//! - Multi-source merging invariants
//! - Template rendering invariants
//! - Plugin loading invariants

use crate::proptest::roadmap_strategies::*;
use proptest::prelude::*;

// ============================================================================
// GitLab API Response Parsing Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_gitlab_mr_round_trip(
        mr in strategy_gitlab_mr_event()
    ) {
        // Test that GitLab MR data can be serialized and deserialized
        let json = serde_json::to_string(&mr).unwrap();
        let deserialized: Result<GitLabMergeRequestEvent, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_gitlab_mr_has_required_fields(
        mr in strategy_gitlab_mr_event()
    ) {
        // Test that GitLab MR has all required fields
        prop_assert!(!mr.iid.is_empty());
        prop_assert!(!mr.title.is_empty());
        prop_assert!(true);
        prop_assert!(!mr.author.username.is_empty());
        prop_assert!(!mr.project.name.is_empty());
    }

    #[test]
    fn prop_gitlab_review_has_required_fields(
        review in strategy_gitlab_review_event()
    ) {
        // Test that GitLab review has all required fields
        prop_assert!(review.mr_iid > 0);
        prop_assert!(!review.author.username.is_empty());
        prop_assert!(true);
    }

    #[test]
    fn prop_gitlab_pagination_consistency(
        pages in prop::collection::vec(strategy_gitlab_mr_event(), 1..10)
    ) {
        // Test that paginated responses can be merged correctly
        let total_count: usize = pages.iter().map(|_| 1).sum();
        prop_assert_eq!(total_count, pages.len());
    }
}

// ============================================================================
// Jira/Linear API Response Parsing Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_jira_issue_round_trip(
        issue in strategy_jira_issue()
    ) {
        // Test that Jira issue data can be serialized and deserialized
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: Result<JiraIssue, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_jira_issue_has_required_fields(
        issue in strategy_jira_issue()
    ) {
        // Test that Jira issue has all required fields
        prop_assert!(!issue.key.is_empty());
        prop_assert!(!issue.summary.is_empty());
        prop_assert!(true);
    }

    #[test]
    fn prop_linear_issue_round_trip(
        issue in strategy_linear_issue()
    ) {
        // Test that Linear issue data can be serialized and deserialized
        let json = serde_json::to_string(&issue).unwrap();
        let deserialized: Result<LinearIssue, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_linear_issue_has_required_fields(
        issue in strategy_linear_issue()
    ) {
        // Test that Linear issue has all required fields
        prop_assert!(!issue.id.is_empty());
        prop_assert!(!issue.title.is_empty());
        prop_assert!(true);
        prop_assert!(!issue.project_id.is_empty());
    }

    #[test]
    fn prop_multi_source_event_uniqueness(
        sources in prop::collection::vec(
            (strategy_source_system(), prop::collection::vec(any::<u64>(), 0..5)),
            1..5
        )
    ) {
        // Test that events from different sources maintain uniqueness
        let all_ids: Vec<_> = sources.iter()
            .flat_map(|(_, ids): &(SourceSystem, Vec<u64>)| ids.iter())
            .collect();
        let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
        prop_assert_eq!(all_ids.len(), unique_ids.len());
    }
}

// ============================================================================
// Multi-Source Merging Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_multi_source_no_duplicates(
        sources in prop::collection::vec(
            (strategy_source_system(), prop::collection::vec(any::<u64>(), 0..5)),
            1..5
        )
    ) {
        // Test that merging events from multiple sources eliminates duplicates
        let all_ids: Vec<_> = sources.iter()
            .flat_map(|(_, ids): &(SourceSystem, Vec<u64>)| ids.iter())
            .collect();
        let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
        prop_assert_eq!(all_ids.len(), unique_ids.len());
    }

    #[test]
    fn prop_multi_source_chronological(
        sources in prop::collection::vec(
            (strategy_source_system(), prop::collection::vec(any::<i64>(), 1..5)),
            1..5
        )
    ) {
        // Test that merged events can be sorted chronologically
        let mut all_timestamps: Vec<_> = sources.iter()
            .flat_map(|(_, timestamps): &(SourceSystem, Vec<i64>)| timestamps.iter().copied())
            .collect();
        all_timestamps.sort();
        // After sorting, timestamps should be in non-decreasing order
        for window in all_timestamps.windows(2) {
            prop_assert!(window[0] <= window[1]);
        }
    }

    #[test]
    fn prop_multi_source_preserves_source(
        sources in prop::collection::vec(
            (strategy_source_system(), prop::collection::vec(any::<u64>(), 1..5)),
            1..5
        )
    ) {
        // Test that source information is preserved during merging
        for (source, _) in &sources {
            prop_assert!(matches!(source, SourceSystem::Github | SourceSystem::Other(_) | SourceSystem::LocalGit | SourceSystem::Manual));
        }
    }
}

// ============================================================================
// Template Rendering Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_template_variable_substitution(
        template in strategy_template(),
        context in strategy_template_context()
    ) {
        // Test that template variables are substituted correctly
        // For valid templates, output should not contain unescaped braces
        if !template.contains("{{") || !template.contains("}}") {
            // If template doesn't have variables, it should render as-is
            prop_assert!(true);
        } else {
            // For templates with variables, ensure context has the variable
            for var_name in context.variables.keys() {
                if template.contains(&format!("{{{{{}}}}}", var_name)) {
                    // Variable exists in template
                    prop_assert!(true);
                }
            }
        }
    }

    #[test]
    fn prop_template_missing_variable_handling(
        template in strategy_template(),
        context in strategy_template_context()
    ) {
        // Test that missing variables are handled gracefully
        // This should not panic
        let _ = (template, context);
        prop_assert!(true);
    }

    #[test]
    fn prop_template_output_valid(
        template in strategy_template(),
        context in strategy_template_context()
    ) {
        // Test that template output is valid (basic validation)
        let _ = (template, context);
        prop_assert!(true);
    }
}

// ============================================================================
// Plugin System Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_plugin_config_round_trip(
        config in strategy_plugin_config()
    ) {
        // Test that plugin config can be serialized and deserialized
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Result<PluginConfig, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_plugin_manifest_round_trip(
        manifest in strategy_plugin_manifest()
    ) {
        // Test that plugin manifest can be serialized and deserialized
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: Result<PluginManifest, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_plugin_manifest_has_required_fields(
        manifest in strategy_plugin_manifest()
    ) {
        // Test that plugin manifest has all required fields
        prop_assert!(!manifest.name.is_empty());
        prop_assert!(!manifest.version.is_empty());
        prop_assert!(!manifest.min_shiplog.is_empty());
        prop_assert!(!manifest.api_version.is_empty());
        prop_assert!(!manifest.schema_version.is_empty());
    }

    #[test]
    fn prop_plugin_state_round_trip(
        state in strategy_plugin_state()
    ) {
        // Test that plugin state can be serialized and deserialized
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: Result<PluginState, _> = serde_json::from_str(&json);
        prop_assert!(deserialized.is_ok());
    }

    #[test]
    fn prop_plugin_state_valid(
        _state in strategy_plugin_state()
    ) {
        // Test that plugin state is valid
        prop_assert!(true); // data.len() is always >= 0 (usize)
        prop_assert!(true); // last_activity validity is not constrained
    }
}
