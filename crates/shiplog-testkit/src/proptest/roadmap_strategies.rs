//! Proptest strategies for ROADMAP features
//!
//! This module provides property test strategies for ROADMAP features:
//! - GitLab API response parsing
//! - Jira/Linear API response parsing
//! - Multi-source merging
//! - Template rendering
//! - Plugin system

use proptest::prelude::*;
use serde::Deserialize;

// ============================================================================
// GitLab API Response Strategies
// ============================================================================

/// Strategy for generating GitLab merge request events
pub fn strategy_gitlab_mr_event() -> impl Strategy<Value = GitLabMergeRequestEvent> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        "[a-zA-Z0-9_-]{1,50}",
        any::<u64>(),
        any::<bool>(),
        any::<i64>(),
        any::<Option<i64>>(),
    )
        .prop_map(
            |(iid, title, number, merged, created_at, merged_at)| GitLabMergeRequestEvent {
                iid,
                title,
                number,
                merged,
                created_at,
                merged_at,
                author: GitLabAuthor {
                    id: any::<u64>().prop_map(|id| Some(id)).boxed(),
                    username: "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s).boxed(),
                },
                project: GitLabProject {
                    id: any::<u64>().prop_map(|id| Some(id)).boxed(),
                    name: "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s).boxed(),
                    web_url: "[a-zA-Z0-9_-]{10,100}".prop_map(|s| s).boxed(),
                },
            },
        )
}

/// Strategy for generating GitLab review events
pub fn strategy_gitlab_review_event() -> impl Strategy<Value = GitLabReviewEvent> {
    (
        any::<u64>(),
        "[a-zA-Z0-9_-]{1,50}",
        any::<i64>(),
        any::<bool>(),
    )
        .prop_map(
            |(mr_iid, body, created_at, approved)| GitLabReviewEvent {
                mr_iid,
                body,
                created_at,
                approved,
                author: GitLabAuthor {
                    id: any::<u64>().prop_map(|id| Some(id)).boxed(),
                    username: "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s).boxed(),
                },
            },
        )
}

// ============================================================================
// Jira/Linear API Response Strategies
// ============================================================================

/// Strategy for generating Jira issue events
pub fn strategy_jira_issue() -> impl Strategy<Value = JiraIssue> {
    (
        "[A-Z]+-[0-9]+",
        "[a-zA-Z0-9 ]{1,100}",
        any::<i64>(),
        any::<Option<i64>>(),
    )
        .prop_map(|(key, summary, created, updated)| JiraIssue {
            key,
            summary,
            created,
            updated,
            status: JiraStatus::Done,
            fields: JiraFields {
                description: "[a-zA-Z0-9 ]{0,500}".prop_map(|s| Some(s)).boxed(),
                priority: any::<u64>().prop_map(|p| Some(p)).boxed(),
            },
        })
}

/// Strategy for generating Linear issue events
pub fn strategy_linear_issue() -> impl Strategy<Value = LinearIssue> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        "[a-zA-Z0-9 ]{1,100}",
        any::<i64>(),
    )
        .prop_map(|(id, title, created)| LinearIssue {
            id,
            title,
            created,
            updated: any::<i64>().prop_map(|u| Some(u)).boxed(),
            status: LinearStatus::Completed,
            project_id: "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s).boxed(),
        })
}

// ============================================================================
// Multi-Source Merging Strategies
// ============================================================================

/// Strategy for generating source system types
pub fn strategy_source_system() -> impl Strategy<Value = SourceSystem> {
    prop_oneof![
        SourceSystem::Github,
        SourceSystem::Other("gitlab".to_string()),
        SourceSystem::Other("jira".to_string()),
        SourceSystem::Other("linear".to_string()),
        SourceSystem::LocalGit,
        SourceSystem::Manual,
    ]
}

// ============================================================================
// Template Rendering Strategies
// ============================================================================

/// Strategy for generating template strings
pub fn strategy_template() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_{}{% \n\\-]{1,500}").unwrap()
}

/// Strategy for generating template variables
pub fn strategy_template_variable() -> impl Strategy<Value = TemplateVariable> {
    (
        "[a-zA-Z_][a-zA-Z0-9_]{1,50}",
        prop_oneof![
            TemplateVariableType::String,
            TemplateVariableType::Number,
            TemplateVariableType::Boolean,
        ],
    )
        .prop_map(|(name, var_type)| TemplateVariable { name, var_type })
}

/// Strategy for generating template context
pub fn strategy_template_context() -> impl Strategy<Value = TemplateContext> {
    prop::collection::hash_map(
        "[a-zA-Z_][a-zA-Z0-9_]{1,50}",
        prop_oneof![
            TemplateValue::String("[a-zA-Z0-9 ]{1,100}".prop_map(|s| s).boxed()),
            TemplateValue::Number(any::<i64>().prop_map(|n| n).boxed()),
            TemplateValue::Boolean(any::<bool>().prop_map(|b| b).boxed()),
        ],
        0..10,
    )
    .prop_map(|map| TemplateContext { variables: map })
}

/// Strategy for generating malformed templates
pub fn strategy_malformed_template() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::string::string_regex(r"\{% if [a-z]+ %}[^{% endif %}]{1,200}").unwrap(),
        prop::string::string_regex(r"\{% for [a-z]+ [a-z]+ %}[^{% endfor %}]{1,200}").unwrap(),
        prop::string::string_regex(r"\{\{ [a-z_]+ \}\}{1,200}").unwrap(),
        prop::string::string_regex(r"\{% [a-z]+ %}\{% [a-z]+ %}").unwrap(),
    ]
}

// ============================================================================
// Plugin System Strategies
// ============================================================================

/// Strategy for generating plugin configuration
pub fn strategy_plugin_config() -> impl Strategy<Value = PluginConfig> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        "[a-zA-Z0-9._/-]{1,200}",
        prop::collection::vec("[a-zA-Z0-9_-]{1,50}", 0..5),
    )
        .prop_map(|(name, path, args)| PluginConfig {
            name,
            path,
            args,
            enabled: any::<bool>(),
        })
}

/// Strategy for generating plugin manifest
pub fn strategy_plugin_manifest() -> impl Strategy<Value = PluginManifest> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        "[0-9]+\\.[0-9]+\\.[0-9]+",
        "[0-9]+\\.[0-9]+\\.[0-9]+",
        any::<u64>(),
        any::<u64>(),
    )
        .prop_map(|(name, version, min_shiplog, api_version, schema_version)| PluginManifest {
            name,
            version,
            min_shiplog,
            api_version,
            schema_version,
            capabilities: prop::collection::btree_set(
                prop_oneof![
                    PluginCapability::Ingest,
                    PluginCapability::Render,
                    PluginCapability::Cluster,
                ],
                0..3,
            )
            .prop_map(|set| set)
            .boxed(),
        })
}

/// Strategy for generating plugin state
pub fn strategy_plugin_state() -> impl Strategy<Value = PluginState> {
    (
        prop::collection::hash_map("[a-zA-Z_][a-zA-Z0-9_]{1,50}", "[a-zA-Z0-9 ]{0,100}", 0..5),
        any::<bool>(),
        any::<i64>(),
    )
        .prop_map(|(data, enabled, last_activity)| PluginState {
            data,
            enabled,
            last_activity,
        })
}

// ============================================================================
// Mock Types for Property Testing
// ============================================================================

#[derive(Debug, Clone)]
pub struct GitLabMergeRequestEvent {
    pub iid: String,
    pub title: String,
    pub number: u64,
    pub merged: bool,
    pub created_at: i64,
    pub merged_at: Option<i64>,
    pub author: GitLabAuthor,
    pub project: GitLabProject,
}

#[derive(Debug, Clone)]
pub struct GitLabAuthor {
    pub id: Option<u64>,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct GitLabProject {
    pub id: Option<u64>,
    pub name: String,
    pub web_url: String,
}

#[derive(Debug, Clone)]
pub struct GitLabReviewEvent {
    pub mr_iid: u64,
    pub body: String,
    pub created_at: i64,
    pub approved: bool,
    pub author: GitLabAuthor,
}

#[derive(Debug, Clone)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub created: i64,
    pub updated: Option<i64>,
    pub status: JiraStatus,
    pub fields: JiraFields,
}

#[derive(Debug, Clone)]
pub enum JiraStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub struct JiraFields {
    pub description: Option<String>,
    pub priority: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct LinearIssue {
    pub id: String,
    pub title: String,
    pub created: i64,
    pub updated: Option<i64>,
    pub status: LinearStatus,
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub enum LinearStatus {
    Backlog,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceSystem {
    Github,
    Other(String),
    LocalGit,
    Manual,
}

#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: TemplateVariableType,
}

#[derive(Debug, Clone)]
pub enum TemplateVariableType {
    String,
    Number,
    Boolean,
}

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub variables: std::collections::HashMap<String, TemplateValue>,
}

#[derive(Debug, Clone)]
pub enum TemplateValue {
    String(String),
    Number(i64),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub name: String,
    pub path: String,
    pub args: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub min_shiplog: String,
    pub api_version: String,
    pub schema_version: String,
    pub capabilities: std::collections::BTreeSet<PluginCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginCapability {
    Ingest,
    Render,
    Cluster,
}

#[derive(Debug, Clone)]
pub struct PluginState {
    pub data: std::collections::HashMap<String, String>,
    pub enabled: bool,
    pub last_activity: i64,
}
