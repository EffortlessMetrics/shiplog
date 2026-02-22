//! Proptest strategies for ROADMAP features
//!
//! This module provides property test strategies for ROADMAP features:
//! - GitLab API response parsing
//! - Jira/Linear API response parsing
//! - Multi-source merging
//! - Template rendering
//! - Plugin system

use proptest::prelude::*;

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
        any::<Option<u64>>(),
        any::<Option<u64>>(),
    )
        .prop_map(
            |(iid, title, number, merged, created_at, merged_at, author_id, project_id)| {
                GitLabMergeRequestEvent {
                    iid,
                    title,
                    number,
                    merged,
                    created_at,
                    merged_at,
                    author: GitLabAuthor {
                        id: author_id,
                        username: "[a-zA-Z0-9_-]{1,50}".to_string(),
                    },
                    project: GitLabProject {
                        id: project_id,
                        name: "[a-zA-Z0-9_-]{1,50}".to_string(),
                        web_url: "[a-zA-Z0-9_-]{10,100}".to_string(),
                    },
                }
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
        any::<Option<u64>>(),
    )
        .prop_map(
            |(mr_iid, body, created_at, approved, author_id)| GitLabReviewEvent {
                mr_iid,
                body,
                created_at,
                approved,
                author: GitLabAuthor {
                    id: author_id,
                    username: "[a-zA-Z0-9_-]{1,50}".to_string(),
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
        any::<Option<String>>(),
        any::<Option<u64>>(),
    )
        .prop_map(
            |(key, summary, created, updated, description, priority)| JiraIssue {
                key,
                summary,
                created,
                updated,
                status: JiraStatus::Done,
                fields: JiraFields {
                    description,
                    priority,
                },
            },
        )
}

/// Strategy for generating Linear issue events
pub fn strategy_linear_issue() -> impl Strategy<Value = LinearIssue> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        "[a-zA-Z0-9 ]{1,100}",
        any::<i64>(),
        any::<Option<i64>>(),
        "[a-zA-Z0-9_-]{1,50}",
    )
        .prop_map(|(id, title, created, updated, project_id)| LinearIssue {
            id: id.to_string(),
            title: title.to_string(),
            created,
            updated,
            status: LinearStatus::Completed,
            project_id: project_id.to_string(),
        })
}

// ============================================================================
// Multi-Source Merging Strategies
// ============================================================================

/// Strategy for generating source system types
pub fn strategy_source_system() -> impl Strategy<Value = SourceSystem> {
    prop_oneof![
        Just(SourceSystem::Github),
        Just(SourceSystem::Other("gitlab".to_string())),
        Just(SourceSystem::Other("jira".to_string())),
        Just(SourceSystem::Other("linear".to_string())),
        Just(SourceSystem::LocalGit),
        Just(SourceSystem::Manual),
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
            Just(TemplateVariableType::String),
            Just(TemplateVariableType::Number),
            Just(TemplateVariableType::Boolean),
        ],
    )
        .prop_map(|(name, var_type)| TemplateVariable { name, var_type })
}

/// Strategy for generating template context
pub fn strategy_template_context() -> impl Strategy<Value = TemplateContext> {
    prop::collection::hash_map(
        "[a-zA-Z_][a-zA-Z0-9_]{1,50}",
        prop_oneof![
            "[a-zA-Z0-9 ]{1,100}".prop_map(TemplateValue::String),
            any::<i64>().prop_map(TemplateValue::Number),
            any::<bool>().prop_map(TemplateValue::Boolean),
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
        any::<bool>(),
    )
        .prop_map(|(name, path, args, enabled)| PluginConfig {
            name,
            path,
            args,
            enabled,
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
        prop::collection::btree_set(
            prop_oneof![
                Just(PluginCapability::Ingest),
                Just(PluginCapability::Render),
                Just(PluginCapability::Cluster),
            ],
            0..3,
        ),
    )
        .prop_map(
            |(name, version, min_shiplog, api_version, schema_version, capabilities)| {
                PluginManifest {
                    name,
                    version,
                    min_shiplog,
                    api_version: api_version.to_string(),
                    schema_version: schema_version.to_string(),
                    capabilities,
                }
            },
        )
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitLabAuthor {
    pub id: Option<u64>,
    pub username: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitLabProject {
    pub id: Option<u64>,
    pub name: String,
    pub web_url: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitLabReviewEvent {
    pub mr_iid: u64,
    pub body: String,
    pub created_at: i64,
    pub approved: bool,
    pub author: GitLabAuthor,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub created: i64,
    pub updated: Option<i64>,
    pub status: JiraStatus,
    pub fields: JiraFields,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum JiraStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JiraFields {
    pub description: Option<String>,
    pub priority: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinearIssue {
    pub id: String,
    pub title: String,
    pub created: i64,
    pub updated: Option<i64>,
    pub status: LinearStatus,
    pub project_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LinearStatus {
    Backlog,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SourceSystem {
    Github,
    Other(String),
    LocalGit,
    Manual,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: TemplateVariableType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TemplateVariableType {
    String,
    Number,
    Boolean,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateContext {
    pub variables: std::collections::HashMap<String, TemplateValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TemplateValue {
    String(String),
    Number(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub path: String,
    pub args: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub min_shiplog: String,
    pub api_version: String,
    pub schema_version: String,
    pub capabilities: std::collections::BTreeSet<PluginCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum PluginCapability {
    Ingest,
    Render,
    Cluster,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginState {
    pub data: std::collections::HashMap<String, String>,
    pub enabled: bool,
    pub last_activity: i64,
}
