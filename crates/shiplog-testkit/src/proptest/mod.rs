//! Proptest strategies for shiplog property-based testing
//!
//! This module provides reusable proptest strategies for generating valid test data
//! across all shiplog crates.

pub mod roadmap_strategies;
pub mod strategies;

#[cfg(test)]
mod roadmap_property_tests;

// Re-export all strategies from strategies module
pub use strategies::{
    strategy_actor, strategy_api_url, strategy_cache_entry, strategy_cache_key,
    strategy_completeness, strategy_coverage_manifest, strategy_coverage_slice,
    strategy_date_range, strategy_datetime_utc, strategy_event_envelope, strategy_event_id_parts,
    strategy_event_kind, strategy_event_payload, strategy_event_vec, strategy_link,
    strategy_manual_payload, strategy_naive_date, strategy_non_empty_string,
    strategy_positive_count, strategy_pr_number, strategy_pr_payload, strategy_pr_state,
    strategy_repo_name, strategy_repo_ref, strategy_repo_visibility, strategy_review_payload,
    strategy_source_ref, strategy_source_system as base_strategy_source_system,
    strategy_time_window, strategy_ttl_duration, strategy_url, strategy_workstream,
    strategy_workstream_id_parts, strategy_workstream_stats, strategy_workstreams_file,
};

// Re-export all strategies from roadmap_strategies module
pub use roadmap_strategies::{
    strategy_gitlab_mr_event, strategy_gitlab_review_event, strategy_jira_issue,
    strategy_linear_issue, strategy_malformed_template, strategy_plugin_config,
    strategy_plugin_manifest, strategy_plugin_state,
    strategy_source_system as roadmap_strategy_source_system, strategy_template,
    strategy_template_context, strategy_template_variable,
};

// Re-export the more specific source_system strategy
pub use roadmap_strategy_source_system as strategy_source_system;
