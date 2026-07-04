#![allow(dead_code)]

use super::{
    ConfigGitSource, ConfigGithubSource, ConfigGitlabSource, ConfigJiraSource, ConfigJsonSource,
    ConfigLinearSource, ConfigManualSource, InitSource, IssueStatus, LinearIssueStatus, MrState,
    ShiplogConfig, config_base_dir, config_redaction_key_env, config_version_state,
    env_var_present, gitlab_api_base, optional_config_string, required_config_path,
};
use serde::Serialize;
use shiplog::ingest::manual::read_manual_events;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SetupStatus {
    pub(crate) overall_status: SetupOverallStatus,
    pub(crate) sources: Vec<SetupItem>,
    pub(crate) local_files: Vec<SetupItem>,
    pub(crate) credentials: Vec<SetupItem>,
    pub(crate) share_profiles: Vec<SetupItem>,
    pub(crate) next_actions: Vec<SetupNextAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupOverallStatus {
    Ready,
    ReadyWithCaveats,
    NeedsSetup,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupItemStatus {
    Ready,
    ReadyWithCaveats,
    Disabled,
    Unavailable,
    Blocked,
    StaleConfig,
    Unknown,
    Missing,
    Malformed,
    OptionalAbsent,
    NotGenerated,
}

impl SetupItemStatus {
    fn is_blocking(self) -> bool {
        matches!(self, Self::Blocked | Self::Malformed | Self::StaleConfig)
    }

    fn needs_setup(self) -> bool {
        matches!(self, Self::Unavailable | Self::Missing)
    }

    fn caveated(self) -> bool {
        matches!(
            self,
            Self::ReadyWithCaveats
                | Self::Disabled
                | Self::OptionalAbsent
                | Self::NotGenerated
                | Self::Unknown
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SetupItem {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) enabled: bool,
    pub(crate) status: SetupItemStatus,
    pub(crate) reason: String,
    pub(crate) next_action: Option<SetupNextAction>,
    pub(crate) writes: bool,
    pub(crate) receipt_refs: Vec<SetupReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SetupNextAction {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) writes: bool,
    pub(crate) reason: String,
    pub(crate) priority: u8,
    pub(crate) receipt_refs: Vec<SetupReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SetupReceiptRef {
    pub(crate) field: String,
    pub(crate) key: Option<String>,
    pub(crate) path: Option<PathBuf>,
}

#[derive(Default)]
struct SetupStatusBuilder {
    sources: Vec<SetupItem>,
    local_files: Vec<SetupItem>,
    credentials: Vec<SetupItem>,
    share_profiles: Vec<SetupItem>,
    next_actions: Vec<SetupNextAction>,
}

pub(crate) fn build_setup_status(
    config_path: &Path,
    selected_sources: &[InitSource],
) -> SetupStatus {
    let mut builder = SetupStatusBuilder::default();
    let config_ref = receipt(
        "config",
        Some("shiplog.toml"),
        Some(config_path.to_path_buf()),
    );

    if !config_path.exists() {
        let action = next_action(
            "init_guided",
            "Create guided setup files",
            "shiplog init --guided",
            true,
            "shiplog.toml is missing",
            1,
            vec![config_ref.clone()],
        );
        builder.push_local_file(item(
            "config",
            "Config",
            true,
            SetupItemStatus::Missing,
            format!("{} not found", config_path.display()),
            Some(action),
            vec![config_ref],
        ));
        return builder.finish();
    }

    let config_text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(err) => {
            let action = next_action(
                "config_validate",
                "Validate config",
                &format!(
                    "shiplog config validate --config {}",
                    quote_setup_value(&config_path.display().to_string())
                ),
                false,
                "config could not be read",
                1,
                vec![config_ref.clone()],
            );
            builder.push_local_file(item(
                "config",
                "Config",
                true,
                SetupItemStatus::Blocked,
                format!("read {}: {err}", config_path.display()),
                Some(action),
                vec![config_ref],
            ));
            return builder.finish();
        }
    };

    let config = match toml::from_str::<ShiplogConfig>(&config_text) {
        Ok(config) => config,
        Err(err) => {
            let action = next_action(
                "config_validate",
                "Validate config",
                &format!(
                    "shiplog config validate --config {}",
                    quote_setup_value(&config_path.display().to_string())
                ),
                false,
                "config could not be parsed",
                1,
                vec![config_ref.clone()],
            );
            builder.push_local_file(item(
                "config",
                "Config",
                true,
                SetupItemStatus::Malformed,
                format!("parse {}: {err}", config_path.display()),
                Some(action),
                vec![config_ref],
            ));
            return builder.finish();
        }
    };

    match config_version_state(&config) {
        Ok(version) => builder.push_local_file(item(
            "config",
            "Config",
            true,
            SetupItemStatus::Ready,
            format!("config_version {}", version.label()),
            None,
            vec![config_ref.clone()],
        )),
        Err(err) => {
            let action = next_action(
                "config_migrate",
                "Migrate config",
                &format!(
                    "shiplog config migrate --config {}",
                    quote_setup_value(&config_path.display().to_string())
                ),
                true,
                "config version is not supported",
                1,
                vec![config_ref.clone()],
            );
            builder.push_local_file(item(
                "config",
                "Config",
                true,
                SetupItemStatus::StaleConfig,
                err.to_string(),
                Some(action),
                vec![config_ref.clone()],
            ));
        }
    }

    let base_dir = config_base_dir(config_path);
    build_source_items(&mut builder, &config, &base_dir, selected_sources);
    build_credential_items(&mut builder, &config);
    build_share_profile_items(&mut builder, &config);
    builder.finish()
}

pub(crate) fn print_setup_status(status: &SetupStatus) {
    println!(
        "Setup readiness: {}",
        setup_overall_status_label(status.overall_status)
    );

    let items: Vec<&SetupItem> = status
        .sources
        .iter()
        .chain(status.local_files.iter())
        .chain(status.credentials.iter())
        .chain(status.share_profiles.iter())
        .collect();

    print_setup_group(SetupPrintGroup::Blocked, &items);
    print_setup_group(SetupPrintGroup::Unavailable, &items);
    print_setup_group(SetupPrintGroup::Ready, &items);
    print_setup_group(SetupPrintGroup::Disabled, &items);
    print_setup_group(SetupPrintGroup::Unknown, &items);

    println!();
    println!("Next:");
    if status.next_actions.is_empty() {
        println!("1. shiplog intake --last-6-months --explain [writes] - collect evidence");
        return;
    }
    for (index, action) in status.next_actions.iter().enumerate() {
        println!(
            "{}. {} [{}] - {}",
            index + 1,
            action.command,
            write_label(action.writes),
            action.label
        );
        println!("   Reason: {}", action.reason);
    }
}

pub(crate) fn setup_status_needs_action(status: &SetupStatus) -> bool {
    matches!(
        status.overall_status,
        SetupOverallStatus::NeedsSetup | SetupOverallStatus::Blocked
    )
}

/// Source-scoped projection of [`SetupStatus`] for the `sources status` command.
///
/// This carries the same source rows and deduplicated source next-actions that
/// the human `sources status` view prints, plus the read-only `needs_action`
/// exit signal, so agents and scripts get a stable machine contract that cannot
/// drift from the text output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SourcesStatusView {
    pub(crate) needs_action: bool,
    pub(crate) sources: Vec<SetupItem>,
    pub(crate) next_actions: Vec<SetupNextAction>,
}

pub(crate) fn build_sources_status_view(status: &SetupStatus) -> SourcesStatusView {
    let mut seen = BTreeSet::new();
    let next_actions = status
        .sources
        .iter()
        .filter_map(|source| source.next_action.clone())
        .filter(|action| seen.insert(action.command.clone()))
        .collect();
    SourcesStatusView {
        needs_action: source_status_needs_action(status),
        sources: status.sources.clone(),
        next_actions,
    }
}

pub(crate) fn print_sources_status(status: &SetupStatus) {
    let view = build_sources_status_view(status);
    println!("Source setup status:");
    println!(
        "{:<11} {:<7} {:<18} {:<15} reason",
        "source_key", "enabled", "status", "source_label"
    );
    for source in &view.sources {
        println!(
            "{:<11} {:<7} {:<18} {:<15} {}",
            source.key,
            if source.enabled { "yes" } else { "no" },
            setup_item_status_key(source.status),
            source.label,
            source.reason
        );
    }

    println!();
    println!("Next:");
    if view.next_actions.is_empty() {
        if view
            .sources
            .iter()
            .any(|source| source.status == SetupItemStatus::Ready)
        {
            println!("1. shiplog intake --last-6-months --explain [writes] - collect evidence");
        } else {
            println!("1. shiplog doctor --setup [read-only] - inspect setup prerequisites");
        }
        return;
    }

    for (index, action) in view.next_actions.iter().enumerate() {
        println!(
            "{}. {} [{}] - {}",
            index + 1,
            action.command,
            write_label(action.writes),
            action.label
        );
        println!("   Reason: {}", action.reason);
    }
}

pub(crate) fn source_status_needs_action(status: &SetupStatus) -> bool {
    let any_ready_source = status
        .sources
        .iter()
        .any(|source| source.status == SetupItemStatus::Ready);
    !any_ready_source
        || status.sources.iter().any(|source| {
            source.enabled
                && matches!(
                    source.status,
                    SetupItemStatus::Unavailable
                        | SetupItemStatus::Blocked
                        | SetupItemStatus::StaleConfig
                        | SetupItemStatus::Missing
                        | SetupItemStatus::Malformed
                )
        })
}

pub(crate) fn setup_overall_status_label(status: SetupOverallStatus) -> &'static str {
    match status {
        SetupOverallStatus::Ready => "Ready",
        SetupOverallStatus::ReadyWithCaveats => "Ready with caveats",
        SetupOverallStatus::NeedsSetup => "Needs setup",
        SetupOverallStatus::Blocked => "Blocked",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupPrintGroup {
    Blocked,
    Unavailable,
    Ready,
    Disabled,
    Unknown,
}

impl SetupPrintGroup {
    fn title(self) -> &'static str {
        match self {
            Self::Blocked => "Blocked",
            Self::Unavailable => "Unavailable",
            Self::Ready => "Ready",
            Self::Disabled => "Disabled",
            Self::Unknown => "Unknown",
        }
    }
}

fn print_setup_group(group: SetupPrintGroup, items: &[&SetupItem]) {
    let mut seen = BTreeSet::new();
    let mut printed_heading = false;
    for item in items
        .iter()
        .copied()
        .filter(|item| setup_print_group(item.status) == group)
    {
        let next_command = item
            .next_action
            .as_ref()
            .map(|action| action.command.clone())
            .unwrap_or_default();
        if !seen.insert((item.label.clone(), item.reason.clone(), next_command)) {
            continue;
        }
        if !printed_heading {
            println!();
            println!("{}:", group.title());
            printed_heading = true;
        }
        println!(
            "- {} [{}; {}]: {}",
            item.label,
            setup_item_status_label(item.status),
            if item.enabled { "enabled" } else { "disabled" },
            item.reason
        );
        if let Some(action) = &item.next_action {
            println!(
                "  Next ({}): {}",
                write_label(action.writes),
                action.command
            );
        }
    }
}

fn setup_print_group(status: SetupItemStatus) -> SetupPrintGroup {
    match status {
        SetupItemStatus::Blocked | SetupItemStatus::Malformed | SetupItemStatus::StaleConfig => {
            SetupPrintGroup::Blocked
        }
        SetupItemStatus::Unavailable | SetupItemStatus::Missing => SetupPrintGroup::Unavailable,
        SetupItemStatus::Ready | SetupItemStatus::ReadyWithCaveats => SetupPrintGroup::Ready,
        SetupItemStatus::Disabled
        | SetupItemStatus::OptionalAbsent
        | SetupItemStatus::NotGenerated => SetupPrintGroup::Disabled,
        SetupItemStatus::Unknown => SetupPrintGroup::Unknown,
    }
}

fn setup_item_status_label(status: SetupItemStatus) -> &'static str {
    match status {
        SetupItemStatus::Ready => "ready",
        SetupItemStatus::ReadyWithCaveats => "ready with caveats",
        SetupItemStatus::Disabled => "disabled",
        SetupItemStatus::Unavailable => "unavailable",
        SetupItemStatus::Blocked => "blocked",
        SetupItemStatus::StaleConfig => "stale config",
        SetupItemStatus::Unknown => "unknown",
        SetupItemStatus::Missing => "missing",
        SetupItemStatus::Malformed => "malformed",
        SetupItemStatus::OptionalAbsent => "optional absent",
        SetupItemStatus::NotGenerated => "not generated",
    }
}

fn setup_item_status_key(status: SetupItemStatus) -> &'static str {
    match status {
        SetupItemStatus::Ready => "ready",
        SetupItemStatus::ReadyWithCaveats => "ready_with_caveats",
        SetupItemStatus::Disabled => "disabled",
        SetupItemStatus::Unavailable => "unavailable",
        SetupItemStatus::Blocked => "blocked",
        SetupItemStatus::StaleConfig => "stale_config",
        SetupItemStatus::Unknown => "unknown",
        SetupItemStatus::Missing => "missing",
        SetupItemStatus::Malformed => "malformed",
        SetupItemStatus::OptionalAbsent => "optional_absent",
        SetupItemStatus::NotGenerated => "not_generated",
    }
}

fn write_label(writes: bool) -> &'static str {
    if writes { "writes" } else { "read-only" }
}

impl SetupStatusBuilder {
    fn push_source(&mut self, item: SetupItem) {
        self.push_item_next_action(&item);
        self.sources.push(item);
    }

    fn push_local_file(&mut self, item: SetupItem) {
        self.push_item_next_action(&item);
        self.local_files.push(item);
    }

    fn push_credential(&mut self, item: SetupItem) {
        self.push_item_next_action(&item);
        self.credentials.push(item);
    }

    fn push_share_profile(&mut self, item: SetupItem) {
        self.push_item_next_action(&item);
        self.share_profiles.push(item);
    }

    fn push_item_next_action(&mut self, item: &SetupItem) {
        if let Some(action) = &item.next_action {
            self.next_actions.push(action.clone());
        }
    }

    fn finish(mut self) -> SetupStatus {
        self.next_actions.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.key.cmp(&right.key))
                .then_with(|| left.command.cmp(&right.command))
        });
        self.next_actions
            .dedup_by(|left, right| left.command == right.command);

        let config_not_ready = self
            .local_files
            .iter()
            .find(|item| item.key == "config")
            .is_some_and(|item| item.status != SetupItemStatus::Ready);
        let any_ready_source = self
            .sources
            .iter()
            .any(|item| item.status == SetupItemStatus::Ready);
        let all_items = self
            .sources
            .iter()
            .chain(self.local_files.iter())
            .chain(self.credentials.iter())
            .chain(self.share_profiles.iter());
        let mut has_blocking = false;
        let mut has_needs_setup = false;
        let mut has_caveat = false;
        for item in all_items {
            has_blocking |= item.status.is_blocking();
            has_needs_setup |= item.status.needs_setup();
            has_caveat |= item.status.caveated();
        }

        let overall_status = if config_not_ready || !any_ready_source {
            SetupOverallStatus::Blocked
        } else if has_blocking || has_needs_setup {
            SetupOverallStatus::NeedsSetup
        } else if has_caveat {
            SetupOverallStatus::ReadyWithCaveats
        } else {
            SetupOverallStatus::Ready
        };

        SetupStatus {
            overall_status,
            sources: self.sources,
            local_files: self.local_files,
            credentials: self.credentials,
            share_profiles: self.share_profiles,
            next_actions: self.next_actions,
        }
    }
}

fn build_source_items(
    builder: &mut SetupStatusBuilder,
    config: &ShiplogConfig,
    base_dir: &Path,
    selected_sources: &[InitSource],
) {
    build_github_source(builder, config.sources.github.as_ref(), selected_sources);
    build_gitlab_source(builder, config.sources.gitlab.as_ref(), selected_sources);
    build_jira_source(builder, config.sources.jira.as_ref(), selected_sources);
    build_linear_source(builder, config.sources.linear.as_ref(), selected_sources);
    build_git_source(
        builder,
        config.sources.git.as_ref(),
        base_dir,
        selected_sources,
    );
    build_json_source(
        builder,
        config.sources.json.as_ref(),
        base_dir,
        selected_sources,
    );
    build_manual_source(
        builder,
        config.sources.manual.as_ref(),
        base_dir,
        selected_sources,
    );
}

fn build_github_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigGithubSource>,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Github) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("github", "GitHub", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "github",
            "GitHub",
            "disabled in shiplog.toml",
        ));
        return;
    }

    let user = optional_config_string(source.user.as_deref());
    let token_present = env_var_present("GITHUB_TOKEN");
    let (status, reason, action) = match (user.as_deref(), source.me, token_present) {
        (Some(_), true, _) => (
            SetupItemStatus::Blocked,
            "configure either sources.github.user or me = true, not both".to_string(),
            Some(config_explain_action("github identity")),
        ),
        (None, false, _) => (
            SetupItemStatus::Blocked,
            "set sources.github.user or me = true".to_string(),
            Some(config_explain_action("github identity")),
        ),
        (Some(user), false, false) => (
            SetupItemStatus::Unavailable,
            format!("GITHUB_TOKEN not set for configured user {user}"),
            Some(env_action("GITHUB_TOKEN", "github token")),
        ),
        (None, true, false) => (
            SetupItemStatus::Unavailable,
            "GITHUB_TOKEN not set for me identity discovery".to_string(),
            Some(env_action("GITHUB_TOKEN", "github token")),
        ),
        (Some(user), false, true) => (
            SetupItemStatus::Ready,
            format!("token present, user {user}"),
            None,
        ),
        (None, true, true) => (
            SetupItemStatus::Ready,
            "token present, me identity can be resolved during intake".to_string(),
            None,
        ),
    };
    builder.push_source(source_item(
        "github", "GitHub", true, status, reason, action,
    ));
}

fn build_gitlab_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigGitlabSource>,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Gitlab) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("gitlab", "GitLab", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "gitlab",
            "GitLab",
            "disabled in shiplog.toml",
        ));
        return;
    }

    let instance =
        optional_config_string(source.instance.as_deref()).unwrap_or_else(|| "gitlab.com".into());
    if let Err(err) = gitlab_api_base(&instance) {
        builder.push_source(source_item(
            "gitlab",
            "GitLab",
            true,
            SetupItemStatus::StaleConfig,
            err.to_string(),
            Some(config_explain_action("gitlab instance")),
        ));
        return;
    }
    if let Some(state) = optional_config_string(source.state.as_deref())
        && let Err(err) = state.parse::<MrState>()
    {
        builder.push_source(source_item(
            "gitlab",
            "GitLab",
            true,
            SetupItemStatus::StaleConfig,
            format!("parse state {state:?}: {err}"),
            Some(config_explain_action("gitlab state")),
        ));
        return;
    }

    let user = optional_config_string(source.user.as_deref());
    let token_present = env_var_present("GITLAB_TOKEN");
    let (status, reason, action) = match (user.as_deref(), source.me, token_present) {
        (Some(_), true, _) => (
            SetupItemStatus::Blocked,
            "configure either sources.gitlab.user or me = true, not both".to_string(),
            Some(config_explain_action("gitlab identity")),
        ),
        (None, false, _) => (
            SetupItemStatus::Blocked,
            "set sources.gitlab.user or me = true".to_string(),
            Some(config_explain_action("gitlab identity")),
        ),
        (Some(user), false, false) => (
            SetupItemStatus::Unavailable,
            format!("GITLAB_TOKEN not set for configured user {user}"),
            Some(env_action("GITLAB_TOKEN", "gitlab token")),
        ),
        (None, true, false) => (
            SetupItemStatus::Unavailable,
            "GITLAB_TOKEN not set for me identity discovery".to_string(),
            Some(env_action("GITLAB_TOKEN", "gitlab token")),
        ),
        (Some(user), false, true) => (
            SetupItemStatus::Ready,
            format!("token present, user {user}, instance {instance}"),
            None,
        ),
        (None, true, true) => (
            SetupItemStatus::Ready,
            format!("token present, me identity can be resolved for {instance} during intake"),
            None,
        ),
    };
    builder.push_source(source_item(
        "gitlab", "GitLab", true, status, reason, action,
    ));
}

fn build_jira_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigJiraSource>,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Jira) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("jira", "Jira", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source("jira", "Jira", "disabled in shiplog.toml"));
        return;
    }
    if optional_config_string(source.user.as_deref()).is_none() {
        builder.push_source(source_item(
            "jira",
            "Jira",
            true,
            SetupItemStatus::Blocked,
            "set sources.jira.user".to_string(),
            Some(config_explain_action("jira user")),
        ));
        return;
    }
    if optional_config_string(source.instance.as_deref()).is_none() {
        builder.push_source(source_item(
            "jira",
            "Jira",
            true,
            SetupItemStatus::Blocked,
            "set sources.jira.instance".to_string(),
            Some(config_explain_action("jira instance")),
        ));
        return;
    }
    let status = source.status.as_deref().unwrap_or("done");
    if let Err(err) = status.parse::<IssueStatus>() {
        builder.push_source(source_item(
            "jira",
            "Jira",
            true,
            SetupItemStatus::StaleConfig,
            format!("parse status {status:?}: {err}"),
            Some(config_explain_action("jira status")),
        ));
        return;
    }
    if !env_var_present("JIRA_TOKEN") {
        builder.push_source(source_item(
            "jira",
            "Jira",
            true,
            SetupItemStatus::Unavailable,
            "JIRA_TOKEN not set".to_string(),
            Some(env_action("JIRA_TOKEN", "jira token")),
        ));
        return;
    }
    builder.push_source(source_item(
        "jira",
        "Jira",
        true,
        SetupItemStatus::Ready,
        "token and required config present".to_string(),
        None,
    ));
}

fn build_linear_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigLinearSource>,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Linear) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("linear", "Linear", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "linear",
            "Linear",
            "disabled in shiplog.toml",
        ));
        return;
    }
    if optional_config_string(source.user_id.as_deref()).is_none() {
        builder.push_source(source_item(
            "linear",
            "Linear",
            true,
            SetupItemStatus::Blocked,
            "set sources.linear.user_id".to_string(),
            Some(config_explain_action("linear user")),
        ));
        return;
    }
    let status = source.status.as_deref().unwrap_or("done");
    if let Err(err) = status.parse::<LinearIssueStatus>() {
        builder.push_source(source_item(
            "linear",
            "Linear",
            true,
            SetupItemStatus::StaleConfig,
            format!("parse status {status:?}: {err}"),
            Some(config_explain_action("linear status")),
        ));
        return;
    }
    if !env_var_present("LINEAR_API_KEY") {
        builder.push_source(source_item(
            "linear",
            "Linear",
            true,
            SetupItemStatus::Unavailable,
            "LINEAR_API_KEY not set".to_string(),
            Some(env_action("LINEAR_API_KEY", "linear token")),
        ));
        return;
    }
    builder.push_source(source_item(
        "linear",
        "Linear",
        true,
        SetupItemStatus::Ready,
        "token and required config present".to_string(),
        None,
    ));
}

fn build_git_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigGitSource>,
    base_dir: &Path,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Git) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("git", "Local git", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "git",
            "Local git",
            "disabled in shiplog.toml",
        ));
        return;
    }
    let repo = match required_config_path(base_dir, "git", "repo", source.repo.as_ref()) {
        Ok(repo) => repo,
        Err(err) => {
            builder.push_source(source_item(
                "git",
                "Local git",
                true,
                SetupItemStatus::Blocked,
                err.to_string(),
                Some(config_explain_action("git repo")),
            ));
            return;
        }
    };
    if !repo.exists() {
        builder.push_source(source_item(
            "git",
            "Local git",
            true,
            SetupItemStatus::Unavailable,
            format!("{} not found", repo.display()),
            Some(config_explain_action("git repo")),
        ));
        return;
    }
    if !repo.is_dir() {
        builder.push_source(source_item(
            "git",
            "Local git",
            true,
            SetupItemStatus::Blocked,
            format!("{} is not a directory", repo.display()),
            Some(config_explain_action("git repo")),
        ));
        return;
    }
    match git2::Repository::open(&repo) {
        Ok(_) => builder.push_source(source_item(
            "git",
            "Local git",
            true,
            SetupItemStatus::Ready,
            format!("repo {} readable", repo.display()),
            None,
        )),
        Err(err) => builder.push_source(source_item(
            "git",
            "Local git",
            true,
            SetupItemStatus::Blocked,
            format!("{} is not a readable git repo: {err}", repo.display()),
            Some(config_explain_action("git repo")),
        )),
    }
}

fn build_json_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigJsonSource>,
    base_dir: &Path,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Json) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source("json", "JSON import", "not configured"));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "json",
            "JSON import",
            "disabled in shiplog.toml",
        ));
        return;
    }
    let events = required_config_path(base_dir, "json", "events", source.events.as_ref());
    let coverage = required_config_path(base_dir, "json", "coverage", source.coverage.as_ref());
    match (events, coverage) {
        (Ok(events), Ok(coverage)) if events.exists() && coverage.exists() => {
            builder.push_source(source_item(
                "json",
                "JSON import",
                true,
                SetupItemStatus::Ready,
                format!(
                    "events {}, coverage {} readable",
                    events.display(),
                    coverage.display()
                ),
                None,
            ));
        }
        (Ok(events), _) if !events.exists() => builder.push_source(source_item(
            "json",
            "JSON import",
            true,
            SetupItemStatus::Unavailable,
            format!("{} not found", events.display()),
            Some(config_explain_action("json events")),
        )),
        (_, Ok(coverage)) if !coverage.exists() => builder.push_source(source_item(
            "json",
            "JSON import",
            true,
            SetupItemStatus::Unavailable,
            format!("{} not found", coverage.display()),
            Some(config_explain_action("json coverage")),
        )),
        (Err(err), _) | (_, Err(err)) => builder.push_source(source_item(
            "json",
            "JSON import",
            true,
            SetupItemStatus::Blocked,
            err.to_string(),
            Some(config_explain_action("json paths")),
        )),
        _ => builder.push_source(source_item(
            "json",
            "JSON import",
            true,
            SetupItemStatus::Unknown,
            "json source state could not be determined".to_string(),
            None,
        )),
    }
}

fn build_manual_source(
    builder: &mut SetupStatusBuilder,
    source: Option<&ConfigManualSource>,
    base_dir: &Path,
    selected_sources: &[InitSource],
) {
    if !selected_source_includes(selected_sources, InitSource::Manual) {
        return;
    }
    let Some(source) = source else {
        builder.push_source(disabled_source(
            "manual",
            "Manual journal",
            "not configured",
        ));
        builder.push_local_file(local_file_item(
            "manual_events",
            "Manual journal",
            false,
            SetupItemStatus::OptionalAbsent,
            "manual source not configured",
            None,
            None,
        ));
        return;
    };
    if !source.enabled {
        builder.push_source(disabled_source(
            "manual",
            "Manual journal",
            "disabled in shiplog.toml",
        ));
        builder.push_local_file(local_file_item(
            "manual_events",
            "Manual journal",
            false,
            SetupItemStatus::OptionalAbsent,
            "manual source disabled",
            None,
            None,
        ));
        return;
    }
    let events = match required_config_path(base_dir, "manual", "events", source.events.as_ref()) {
        Ok(events) => events,
        Err(err) => {
            builder.push_source(source_item(
                "manual",
                "Manual journal",
                true,
                SetupItemStatus::Blocked,
                err.to_string(),
                Some(config_explain_action("manual events")),
            ));
            builder.push_local_file(local_file_item(
                "manual_events",
                "Manual journal",
                true,
                SetupItemStatus::Missing,
                err.to_string(),
                Some(config_explain_action("manual events")),
                None,
            ));
            return;
        }
    };
    if !events.exists() {
        let action = next_action(
            "init_guided",
            "Create guided setup files",
            "shiplog init --guided",
            true,
            "manual journal is missing",
            2,
            vec![receipt(
                "local_file",
                Some("manual_events"),
                Some(events.clone()),
            )],
        );
        builder.push_source(source_item(
            "manual",
            "Manual journal",
            true,
            SetupItemStatus::Unavailable,
            format!("{} not found", events.display()),
            Some(action.clone()),
        ));
        builder.push_local_file(local_file_item(
            "manual_events",
            "Manual journal",
            true,
            SetupItemStatus::Missing,
            format!("{} not found", events.display()),
            Some(action),
            Some(events),
        ));
        return;
    }
    match read_manual_events(&events) {
        Ok(_) => {
            builder.push_source(source_item(
                "manual",
                "Manual journal",
                true,
                SetupItemStatus::Ready,
                format!("{} valid", events.display()),
                None,
            ));
            builder.push_local_file(local_file_item(
                "manual_events",
                "Manual journal",
                true,
                SetupItemStatus::Ready,
                format!("{} valid", events.display()),
                None,
                Some(events),
            ));
        }
        Err(err) => {
            let action = next_action(
                "repair_manual_journal",
                "Repair manual journal schema",
                "shiplog doctor --setup",
                false,
                "manual journal is malformed",
                1,
                vec![receipt(
                    "local_file",
                    Some("manual_events"),
                    Some(events.clone()),
                )],
            );
            builder.push_source(source_item(
                "manual",
                "Manual journal",
                true,
                SetupItemStatus::Blocked,
                format!("manual_events.yaml malformed: {err:#}"),
                Some(action.clone()),
            ));
            builder.push_local_file(local_file_item(
                "manual_events",
                "Manual journal",
                true,
                SetupItemStatus::Malformed,
                format!("manual_events.yaml malformed: {err:#}"),
                Some(action),
                Some(events),
            ));
        }
    }
}

fn build_credential_items(builder: &mut SetupStatusBuilder, config: &ShiplogConfig) {
    credential_item(
        builder,
        "github_token",
        "GitHub token",
        config
            .sources
            .github
            .as_ref()
            .is_some_and(|source| source.enabled),
        "GITHUB_TOKEN",
    );
    credential_item(
        builder,
        "gitlab_token",
        "GitLab token",
        config
            .sources
            .gitlab
            .as_ref()
            .is_some_and(|source| source.enabled),
        "GITLAB_TOKEN",
    );
    credential_item(
        builder,
        "jira_token",
        "Jira token",
        config
            .sources
            .jira
            .as_ref()
            .is_some_and(|source| source.enabled),
        "JIRA_TOKEN",
    );
    credential_item(
        builder,
        "linear_api_key",
        "Linear API key",
        config
            .sources
            .linear
            .as_ref()
            .is_some_and(|source| source.enabled),
        "LINEAR_API_KEY",
    );
    let redaction_env = config_redaction_key_env(config);
    credential_item(
        builder,
        "redaction_key",
        "Redaction key",
        true,
        &redaction_env,
    );
}

fn credential_item(
    builder: &mut SetupStatusBuilder,
    key: &str,
    label: &str,
    enabled: bool,
    env_var: &str,
) {
    let receipt_ref = receipt("env", Some(env_var), None);
    if !enabled {
        builder.push_credential(item(
            key,
            label,
            false,
            SetupItemStatus::Disabled,
            format!("{env_var} not required by enabled setup"),
            None,
            vec![receipt_ref],
        ));
        return;
    }
    if env_var_present(env_var) {
        builder.push_credential(item(
            key,
            label,
            true,
            SetupItemStatus::Ready,
            format!("{env_var} present"),
            None,
            vec![receipt_ref],
        ));
    } else {
        builder.push_credential(item(
            key,
            label,
            true,
            SetupItemStatus::Unavailable,
            format!("{env_var} not set"),
            Some(env_action(env_var, label)),
            vec![receipt_ref],
        ));
    }
}

fn build_share_profile_items(builder: &mut SetupStatusBuilder, config: &ShiplogConfig) {
    let key_env = config_redaction_key_env(config);
    build_share_profile_item(
        builder,
        "manager",
        "Manager share",
        &key_env,
        "manager share needs redaction before rendering",
        "manager share rendering is blocked",
    );
    build_share_profile_item(
        builder,
        "public",
        "Public share",
        &key_env,
        "public share also needs strict verification before sharing",
        "strict verification requires a rendered public packet",
    );
}

fn build_share_profile_item(
    builder: &mut SetupStatusBuilder,
    key: &str,
    label: &str,
    key_env: &str,
    ready_reason: &str,
    blocked_reason: &str,
) {
    let receipt_ref = receipt("share_profile", Some(key), None);
    if env_var_present(key_env) {
        builder.push_share_profile(item(
            key,
            label,
            true,
            SetupItemStatus::ReadyWithCaveats,
            format!("{key_env} present; {ready_reason}"),
            Some(next_action(
                &format!("share_explain_{key}"),
                &format!("Explain {key} share posture"),
                &format!("shiplog share explain {key} --latest"),
                false,
                "read profile posture before rendering",
                8,
                vec![receipt_ref.clone()],
            )),
            vec![receipt_ref],
        ));
    } else {
        builder.push_share_profile(item(
            key,
            label,
            true,
            SetupItemStatus::Blocked,
            format!("{key_env} not set; {blocked_reason}"),
            Some(env_action(key_env, "redaction key")),
            vec![receipt_ref],
        ));
    }
}

fn selected_source_includes(selected_sources: &[InitSource], source: InitSource) -> bool {
    selected_sources.is_empty() || selected_sources.contains(&source)
}

fn disabled_source(key: &str, label: &str, reason: &str) -> SetupItem {
    source_item(
        key,
        label,
        false,
        SetupItemStatus::Disabled,
        reason.to_string(),
        None,
    )
}

fn source_item(
    key: &str,
    label: &str,
    enabled: bool,
    status: SetupItemStatus,
    reason: String,
    next_action: Option<SetupNextAction>,
) -> SetupItem {
    item(
        key,
        label,
        enabled,
        status,
        reason,
        next_action,
        vec![receipt("source", Some(key), None)],
    )
}

fn local_file_item(
    key: &str,
    label: &str,
    enabled: bool,
    status: SetupItemStatus,
    reason: impl Into<String>,
    next_action: Option<SetupNextAction>,
    path: Option<PathBuf>,
) -> SetupItem {
    item(
        key,
        label,
        enabled,
        status,
        reason.into(),
        next_action,
        vec![receipt("local_file", Some(key), path)],
    )
}

fn item(
    key: &str,
    label: &str,
    enabled: bool,
    status: SetupItemStatus,
    reason: impl Into<String>,
    next_action: Option<SetupNextAction>,
    receipt_refs: Vec<SetupReceiptRef>,
) -> SetupItem {
    let writes = next_action.as_ref().is_some_and(|action| action.writes);
    SetupItem {
        key: key.to_string(),
        label: label.to_string(),
        enabled,
        status,
        reason: reason.into(),
        next_action,
        writes,
        receipt_refs,
    }
}

fn config_explain_action(reason: &str) -> SetupNextAction {
    next_action(
        "config_explain",
        "Inspect config",
        "shiplog config explain --config shiplog.toml",
        false,
        reason,
        3,
        vec![receipt("config", Some("shiplog.toml"), None)],
    )
}

fn env_action(env_var: &str, label: &str) -> SetupNextAction {
    next_action(
        &format!("set_{}", env_var.to_ascii_lowercase()),
        &format!("Set {label}"),
        &format!("set {env_var}"),
        false,
        &format!("{env_var} is missing"),
        4,
        vec![receipt("env", Some(env_var), None)],
    )
}

fn next_action(
    key: &str,
    label: &str,
    command: &str,
    writes: bool,
    reason: &str,
    priority: u8,
    receipt_refs: Vec<SetupReceiptRef>,
) -> SetupNextAction {
    SetupNextAction {
        key: key.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        writes,
        reason: reason.to_string(),
        priority,
        receipt_refs,
    }
}

fn receipt(field: &str, key: Option<&str>, path: Option<PathBuf>) -> SetupReceiptRef {
    SetupReceiptRef {
        field: field.to_string(),
        key: key.map(ToOwned::to_owned),
        path,
    }
}

fn quote_setup_value(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '\\' | ':'))
    {
        value.to_string()
    } else {
        format!("{value:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog::ingest::manual::write_manual_events;
    use shiplog::schema::event::ManualEventsFile;

    #[test]
    fn setup_status_blocks_missing_config_with_guided_init_action() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let config = temp.path().join("shiplog.toml");

        let status = build_setup_status(&config, &[]);

        assert_eq!(status.overall_status, SetupOverallStatus::Blocked);
        let config_item = find_item(&status.local_files, "config")?;
        assert_eq!(config_item.status, SetupItemStatus::Missing);
        let action = config_item
            .next_action
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("config item should have a next action"))?;
        assert_eq!(action.command, "shiplog init --guided");
        assert!(action.writes);
        Ok(())
    }

    #[test]
    fn setup_status_marks_local_git_and_manual_ready() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        git2::Repository::init(temp.path())?;
        let manual_path = temp.path().join("manual_events.yaml");
        write_manual_events(
            &manual_path,
            &ManualEventsFile {
                version: 1,
                generated_at: chrono::Utc::now(),
                events: Vec::new(),
            },
        )?;
        std::fs::write(
            temp.path().join("shiplog.toml"),
            r#"[shiplog]
config_version = 1

[defaults]
profile = "internal"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
        )?;

        let status = build_setup_status(&temp.path().join("shiplog.toml"), &[]);

        assert_ne!(status.overall_status, SetupOverallStatus::Blocked);
        assert_eq!(
            find_item(&status.sources, "git")?.status,
            SetupItemStatus::Ready
        );
        assert_eq!(
            find_item(&status.sources, "manual")?.status,
            SetupItemStatus::Ready
        );
        assert_eq!(
            find_item(&status.local_files, "manual_events")?.status,
            SetupItemStatus::Ready
        );
        Ok(())
    }

    #[test]
    fn setup_status_reports_missing_json_file_as_unavailable() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("shiplog.toml"),
            r#"[shiplog]
config_version = 1

[defaults]
profile = "internal"

[sources.json]
enabled = true
events = "./missing.events.jsonl"
coverage = "./missing.coverage.json"
"#,
        )?;

        let status = build_setup_status(&temp.path().join("shiplog.toml"), &[]);

        assert_eq!(
            find_item(&status.sources, "json")?.status,
            SetupItemStatus::Unavailable
        );
        assert_eq!(status.overall_status, SetupOverallStatus::Blocked);
        Ok(())
    }

    #[test]
    fn setup_status_does_not_validate_disabled_manual_journal() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("shiplog.toml"),
            r#"[shiplog]
config_version = 1

[defaults]
profile = "internal"

[sources.manual]
enabled = false
events = "./missing_manual_events.yaml"
"#,
        )?;

        let status = build_setup_status(&temp.path().join("shiplog.toml"), &[]);

        let manual_source = find_item(&status.sources, "manual")?;
        assert_eq!(manual_source.status, SetupItemStatus::Disabled);
        let manual_file = find_item(&status.local_files, "manual_events")?;
        assert_eq!(manual_file.status, SetupItemStatus::OptionalAbsent);
        Ok(())
    }

    #[test]
    fn setup_status_blocks_malformed_manual_journal() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(temp.path().join("manual_events.yaml"), "version: nope\n")?;
        std::fs::write(
            temp.path().join("shiplog.toml"),
            r#"[shiplog]
config_version = 1

[defaults]
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
        )?;

        let status = build_setup_status(&temp.path().join("shiplog.toml"), &[]);

        assert_eq!(
            find_item(&status.sources, "manual")?.status,
            SetupItemStatus::Blocked
        );
        assert_eq!(
            find_item(&status.local_files, "manual_events")?.status,
            SetupItemStatus::Malformed
        );
        assert_eq!(status.overall_status, SetupOverallStatus::Blocked);
        Ok(())
    }

    #[test]
    fn sources_status_view_dedupes_actions_and_scopes_to_sources() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        std::fs::write(
            temp.path().join("shiplog.toml"),
            r#"[shiplog]
config_version = 1

[defaults]
profile = "internal"

[sources.git]
enabled = true
repo = "."

[sources.github]
enabled = true
user = "octo"
"#,
        )?;
        git2::Repository::init(temp.path())?;

        let status = build_setup_status(
            &temp.path().join("shiplog.toml"),
            &[InitSource::Git, InitSource::Github],
        );
        let view = build_sources_status_view(&status);

        // The view carries exactly the source rows, not credentials/share noise.
        assert_eq!(view.sources, status.sources);
        assert!(view.needs_action, "missing token should require action");

        // git is ready (no action); github needs a token (one action).
        assert_eq!(
            find_item(&view.sources, "git")?.status,
            SetupItemStatus::Ready
        );
        assert_eq!(
            find_item(&view.sources, "github")?.status,
            SetupItemStatus::Unavailable
        );

        // next_actions are deduped by command and contain the github token action.
        let mut commands: Vec<&str> = view
            .next_actions
            .iter()
            .map(|action| action.command.as_str())
            .collect();
        commands.sort_unstable();
        let mut deduped = commands.clone();
        deduped.dedup();
        assert_eq!(commands, deduped, "next_actions must be deduped by command");
        assert!(commands.contains(&"set GITHUB_TOKEN"));
        Ok(())
    }

    fn find_item<'a>(items: &'a [SetupItem], key: &str) -> anyhow::Result<&'a SetupItem> {
        items
            .iter()
            .find(|item| item.key == key)
            .ok_or_else(|| anyhow::anyhow!("missing setup item {key}"))
    }
}
