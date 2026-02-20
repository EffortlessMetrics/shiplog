//! Pre/post processing hooks for shiplog pipeline.
//!
//! Provides a hook system for extending the shiplog pipeline with
//! custom pre-processing and post-processing logic.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Hook context containing data passed to hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub pipeline_stage: PipelineStage,
    pub data: serde_json::Value,
    pub metadata: HookMetadata,
}

/// Metadata about the hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub correlation_id: Option<String>,
}

impl HookMetadata {
    /// Create new metadata with current timestamp.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            source: source.into(),
            correlation_id: None,
        }
    }

    /// Set the correlation ID.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Pipeline stages where hooks can be attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    PreIngest,
    PostIngest,
    PreTransform,
    PostTransform,
    PreRender,
    PostRender,
    PreBundle,
    PostBundle,
    PreExport,
    PostExport,
}

impl fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineStage::PreIngest => write!(f, "pre-ingest"),
            PipelineStage::PostIngest => write!(f, "post-ingest"),
            PipelineStage::PreTransform => write!(f, "pre-transform"),
            PipelineStage::PostTransform => write!(f, "post-transform"),
            PipelineStage::PreRender => write!(f, "pre-render"),
            PipelineStage::PostRender => write!(f, "post-render"),
            PipelineStage::PreBundle => write!(f, "pre-bundle"),
            PipelineStage::PostBundle => write!(f, "post-bundle"),
            PipelineStage::PreExport => write!(f, "pre-export"),
            PipelineStage::PostExport => write!(f, "post-export"),
        }
    }
}

/// Result of hook execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub success: bool,
    pub modified: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl HookResult {
    /// Create a successful result.
    pub fn success() -> Self {
        Self {
            success: true,
            modified: false,
            message: None,
            data: None,
        }
    }

    /// Create a successful result with modified data.
    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            modified: true,
            message: None,
            data: Some(data),
        }
    }

    /// Create a failure result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            modified: false,
            message: Some(message.into()),
            data: None,
        }
    }

    /// Create a skipped result.
    pub fn skipped(message: impl Into<String>) -> Self {
        Self {
            success: true,
            modified: false,
            message: Some(message.into()),
            data: None,
        }
    }
}

/// A hook that can be executed at pipeline stages.
pub trait Hook: Send + Sync {
    /// Get the hook name.
    fn name(&self) -> &str;

    /// Execute the hook with the given context.
    fn execute(&self, context: &HookContext) -> HookResult;
}

/// A hook registry for managing hooks.
pub struct HookRegistry {
    hooks: std::collections::HashMap<PipelineStage, Vec<Box<dyn Hook>>>,
}

impl HookRegistry {
    /// Create a new hook registry.
    pub fn new() -> Self {
        Self {
            hooks: std::collections::HashMap::new(),
        }
    }

    /// Register a hook for a pipeline stage.
    pub fn register<H: Hook + 'static>(&mut self, stage: PipelineStage, hook: H) {
        self.hooks
            .entry(stage)
            .or_insert_with(Vec::new)
            .push(Box::new(hook));
    }

    /// Execute all hooks for a pipeline stage.
    pub fn execute(&self, stage: PipelineStage, mut context: HookContext) -> HookResult {
        let stage_hooks = match self.hooks.get(&stage) {
            Some(hooks) => hooks,
            None => return HookResult::success(),
        };

        context.pipeline_stage = stage;

        let mut final_data = context.data.clone();
        let mut all_success = true;

        for hook in stage_hooks {
            context.data = final_data.clone();
            let result = hook.execute(&context);
            
            if !result.success {
                all_success = false;
            }
            
            if let Some(data) = result.data {
                final_data = data;
            }
        }

        if all_success {
            HookResult::success_with_data(final_data)
        } else {
            HookResult::failure("One or more hooks failed")
        }
    }

    /// Get the number of registered hooks.
    pub fn len(&self) -> usize {
        self.hooks.values().map(|v| v.len()).sum()
    }

    /// Check if there are no hooks registered.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple hook implementation for closures.
pub struct ClosureHook<F>
where
    F: Fn(&HookContext) -> HookResult + Send + Sync,
{
    name: String,
    closure: F,
}

impl<F> ClosureHook<F>
where
    F: Fn(&HookContext) -> HookResult + Send + Sync,
{
    pub fn new(name: impl Into<String>, closure: F) -> Self {
        Self {
            name: name.into(),
            closure,
        }
    }
}

impl<F> Hook for ClosureHook<F>
where
    F: Fn(&HookContext) -> HookResult + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, context: &HookContext) -> HookResult {
        (self.closure)(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_metadata() {
        let metadata = HookMetadata::new("test");
        assert_eq!(metadata.source, "test");
        assert!(metadata.correlation_id.is_none());

        let metadata = metadata.with_correlation_id("corr-123");
        assert_eq!(metadata.correlation_id, Some("corr-123".to_string()));
    }

    #[test]
    fn test_hook_result() {
        let success = HookResult::success();
        assert!(success.success);
        assert!(!success.modified);

        let with_data = HookResult::success_with_data(serde_json::json!({"key": "value"}));
        assert!(with_data.success);
        assert!(with_data.modified);
        assert!(with_data.data.is_some());

        let failure = HookResult::failure("something went wrong");
        assert!(!failure.success);
        assert!(failure.message.is_some());
    }

    #[test]
    fn test_hook_registry() {
        let mut registry = HookRegistry::new();

        // Add a closure hook
        let hook = ClosureHook::new("test-hook", |_| HookResult::success());
        registry.register(PipelineStage::PreIngest, hook);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        // Execute hooks
        let context = HookContext {
            pipeline_stage: PipelineStage::PreIngest,
            data: serde_json::json!({"test": true}),
            metadata: HookMetadata::new("test"),
        };

        let result = registry.execute(PipelineStage::PreIngest, context);
        assert!(result.success);
    }

    #[test]
    fn test_pipeline_stage_display() {
        assert_eq!(format!("{}", PipelineStage::PreIngest), "pre-ingest");
        assert_eq!(format!("{}", PipelineStage::PostExport), "post-export");
    }

    #[test]
    fn test_closure_hook_execution() {
        let hook = ClosureHook::new("logging-hook", |ctx| {
            println!("Hook executed at stage: {}", ctx.pipeline_stage);
            HookResult::success()
        });

        let context = HookContext {
            pipeline_stage: PipelineStage::PreRender,
            data: serde_json::json!({}),
            metadata: HookMetadata::new("test"),
        };

        let result = hook.execute(&context);
        assert!(result.success);
        assert_eq!(hook.name(), "logging-hook");
    }

    #[test]
    fn test_hook_registry_multiple_hooks() {
        let mut registry = HookRegistry::new();

        registry.register(
            PipelineStage::PreIngest,
            ClosureHook::new("hook1", |_| HookResult::success()),
        );
        registry.register(
            PipelineStage::PreIngest,
            ClosureHook::new("hook2", |_| HookResult::success()),
        );

        let context = HookContext {
            pipeline_stage: PipelineStage::PreIngest,
            data: serde_json::json!({"value": 1}),
            metadata: HookMetadata::new("test"),
        };

        let result = registry.execute(PipelineStage::PreIngest, context);
        assert!(result.success);
        assert!(result.modified);
    }
}
