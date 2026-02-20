//! Workflow and orchestration utilities for shiplog pipeline execution.
//!
//! This crate provides workflow state management and task orchestration
//! utilities for coordinating multi-step operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Workflow states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for WorkflowState {
    fn default() -> Self {
        WorkflowState::Pending
    }
}

/// A workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow identifier.
    pub id: String,
    /// Workflow name.
    pub name: String,
    /// Current state.
    pub state: WorkflowState,
    /// When the workflow was created.
    pub created_at: DateTime<Utc>,
    /// When the workflow was last updated.
    pub updated_at: DateTime<Utc>,
    /// Workflow metadata.
    pub metadata: HashMap<String, String>,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid_simple(),
            name: name.to_string(),
            state: WorkflowState::Pending,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Start the workflow.
    pub fn start(&mut self) {
        self.state = WorkflowState::Running;
        self.updated_at = Utc::now();
    }

    /// Complete the workflow.
    pub fn complete(&mut self) {
        self.state = WorkflowState::Completed;
        self.updated_at = Utc::now();
    }

    /// Fail the workflow.
    pub fn fail(&mut self) {
        self.state = WorkflowState::Failed;
        self.updated_at = Utc::now();
    }

    /// Cancel the workflow.
    pub fn cancel(&mut self) {
        self.state = WorkflowState::Cancelled;
        self.updated_at = Utc::now();
    }

    /// Add metadata to the workflow.
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
        self.updated_at = Utc::now();
    }
}

/// A task within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Task name.
    pub name: String,
    /// Task state.
    pub state: WorkflowState,
    /// Task order in the workflow.
    pub order: u32,
    /// Error message if failed.
    pub error: Option<String>,
}

impl Task {
    /// Create a new task.
    pub fn new(name: &str, order: u32) -> Self {
        Self {
            id: uuid_simple(),
            name: name.to_string(),
            state: WorkflowState::Pending,
            order,
            error: None,
        }
    }

    /// Start the task.
    pub fn start(&mut self) {
        self.state = WorkflowState::Running;
    }

    /// Complete the task.
    pub fn complete(&mut self) {
        self.state = WorkflowState::Completed;
    }

    /// Fail the task with an error.
    pub fn fail(&mut self, error: &str) {
        self.state = WorkflowState::Failed;
        self.error = Some(error.to_string());
    }
}

/// Workflow engine for managing workflows.
pub struct WorkflowEngine {
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    tasks: Arc<RwLock<HashMap<String, Vec<Task>>>>,
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new workflow.
    pub fn create_workflow(&self, name: &str) -> Workflow {
        let workflow = Workflow::new(name);
        let id = workflow.id.clone();
        
        let mut workflows = self.workflows.write().unwrap();
        workflows.insert(id.clone(), workflow.clone());
        
        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(id, Vec::new());
        
        workflow
    }

    /// Get a workflow by ID.
    pub fn get_workflow(&self, id: &str) -> Option<Workflow> {
        let workflows = self.workflows.read().unwrap();
        workflows.get(id).cloned()
    }

    /// Start a workflow.
    pub fn start_workflow(&self, id: &str) -> Option<WorkflowState> {
        let mut workflows = self.workflows.write().unwrap();
        if let Some(workflow) = workflows.get_mut(id) {
            workflow.start();
            Some(workflow.state)
        } else {
            None
        }
    }

    /// Complete a workflow.
    pub fn complete_workflow(&self, id: &str) -> Option<WorkflowState> {
        let mut workflows = self.workflows.write().unwrap();
        if let Some(workflow) = workflows.get_mut(id) {
            workflow.complete();
            Some(workflow.state)
        } else {
            None
        }
    }

    /// Fail a workflow.
    pub fn fail_workflow(&self, id: &str) -> Option<WorkflowState> {
        let mut workflows = self.workflows.write().unwrap();
        if let Some(workflow) = workflows.get_mut(id) {
            workflow.fail();
            Some(workflow.state)
        } else {
            None
        }
    }

    /// Add a task to a workflow.
    pub fn add_task(&self, workflow_id: &str, name: &str, order: u32) -> Option<Task> {
        let task = Task::new(name, order);
        
        let mut tasks = self.tasks.write().unwrap();
        if let Some(workflow_tasks) = tasks.get_mut(workflow_id) {
            workflow_tasks.push(task.clone());
            Some(task)
        } else {
            None
        }
    }

    /// Get tasks for a workflow.
    pub fn get_tasks(&self, workflow_id: &str) -> Option<Vec<Task>> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(workflow_id).cloned()
    }

    /// Start a task in a workflow.
    pub fn start_task(&self, workflow_id: &str, task_id: &str) -> bool {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(workflow_tasks) = tasks.get_mut(workflow_id) {
            if let Some(task) = workflow_tasks.iter_mut().find(|t| t.id == task_id) {
                task.start();
                return true;
            }
        }
        false
    }

    /// Complete a task in a workflow.
    pub fn complete_task(&self, workflow_id: &str, task_id: &str) -> bool {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(workflow_tasks) = tasks.get_mut(workflow_id) {
            if let Some(task) = workflow_tasks.iter_mut().find(|t| t.id == task_id) {
                task.complete();
                return true;
            }
        }
        false
    }

    /// Get all workflow IDs.
    pub fn workflow_ids(&self) -> Vec<String> {
        let workflows = self.workflows.read().unwrap();
        workflows.keys().cloned().collect()
    }

    /// Get workflows by state.
    pub fn workflows_by_state(&self, state: WorkflowState) -> Vec<Workflow> {
        let workflows = self.workflows.read().unwrap();
        workflows
            .values()
            .filter(|w| w.state == state)
            .cloned()
            .collect()
    }
}

/// Generate a simple UUID-like identifier.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}-{:x}", duration.as_secs(), duration.subsec_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("test-workflow");
        
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.state, WorkflowState::Pending);
        assert!(!workflow.id.is_empty());
    }

    #[test]
    fn test_workflow_state_transitions() {
        let mut workflow = Workflow::new("test");
        
        workflow.start();
        assert_eq!(workflow.state, WorkflowState::Running);
        
        workflow.complete();
        assert_eq!(workflow.state, WorkflowState::Completed);
        
        let mut workflow2 = Workflow::new("test2");
        workflow2.fail();
        assert_eq!(workflow2.state, WorkflowState::Failed);
        
        let mut workflow3 = Workflow::new("test3");
        workflow3.cancel();
        assert_eq!(workflow3.state, WorkflowState::Cancelled);
    }

    #[test]
    fn test_workflow_metadata() {
        let mut workflow = Workflow::new("test");
        workflow.set_metadata("key", "value");
        
        assert_eq!(workflow.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("task1", 1);
        
        assert_eq!(task.name, "task1");
        assert_eq!(task.order, 1);
        assert_eq!(task.state, WorkflowState::Pending);
    }

    #[test]
    fn test_task_state_transitions() {
        let mut task = Task::new("test", 0);
        
        task.start();
        assert_eq!(task.state, WorkflowState::Running);
        
        task.complete();
        assert_eq!(task.state, WorkflowState::Completed);
        
        let mut task2 = Task::new("test2", 0);
        task2.fail("error message");
        assert_eq!(task2.state, WorkflowState::Failed);
        assert_eq!(task2.error, Some("error message".to_string()));
    }

    #[test]
    fn test_workflow_engine_create() {
        let engine = WorkflowEngine::new();
        let workflow = engine.create_workflow("my-workflow");
        
        assert_eq!(workflow.name, "my-workflow");
        assert!(engine.get_workflow(&workflow.id).is_some());
    }

    #[test]
    fn test_workflow_engine_tasks() {
        let engine = WorkflowEngine::new();
        let workflow = engine.create_workflow("test");
        
        engine.add_task(&workflow.id, "Step 1", 1).unwrap();
        engine.add_task(&workflow.id, "Step 2", 2).unwrap();
        
        let tasks = engine.get_tasks(&workflow.id).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_workflow_engine_workflow_states() {
        let engine = WorkflowEngine::new();
        
        let w1 = engine.create_workflow("workflow-1");
        let w2 = engine.create_workflow("workflow-2");
        
        engine.start_workflow(&w1.id);
        engine.complete_workflow(&w2.id);
        
        let running = engine.workflows_by_state(WorkflowState::Running);
        let completed = engine.workflows_by_state(WorkflowState::Completed);
        
        assert_eq!(running.len(), 1);
        assert_eq!(completed.len(), 1);
    }

    #[test]
    fn test_workflow_engine_task_transitions() {
        let engine = WorkflowEngine::new();
        let workflow = engine.create_workflow("test");
        
        let task = engine.add_task(&workflow.id, "Step 1", 1).unwrap();
        
        engine.start_task(&workflow.id, &task.id);
        engine.complete_task(&workflow.id, &task.id);
        
        let tasks = engine.get_tasks(&workflow.id).unwrap();
        assert_eq!(tasks[0].state, WorkflowState::Completed);
    }
}
