use shiplog_workflow::{Task, Workflow, WorkflowEngine, WorkflowState};

// --- WorkflowState tests ---

#[test]
fn default_state_is_pending() {
    assert_eq!(WorkflowState::default(), WorkflowState::Pending);
}

#[test]
fn state_equality() {
    assert_eq!(WorkflowState::Pending, WorkflowState::Pending);
    assert_ne!(WorkflowState::Pending, WorkflowState::Running);
    assert_ne!(WorkflowState::Completed, WorkflowState::Failed);
}

// --- Workflow tests ---

#[test]
fn workflow_new_is_pending() {
    let w = Workflow::new("test");
    assert_eq!(w.name, "test");
    assert_eq!(w.state, WorkflowState::Pending);
    assert!(!w.id.is_empty());
    assert!(w.metadata.is_empty());
}

#[test]
fn workflow_state_transitions() {
    let mut w = Workflow::new("w");
    assert_eq!(w.state, WorkflowState::Pending);

    w.start();
    assert_eq!(w.state, WorkflowState::Running);

    w.complete();
    assert_eq!(w.state, WorkflowState::Completed);
}

#[test]
fn workflow_fail_transition() {
    let mut w = Workflow::new("w");
    w.start();
    w.fail();
    assert_eq!(w.state, WorkflowState::Failed);
}

#[test]
fn workflow_cancel_transition() {
    let mut w = Workflow::new("w");
    w.cancel();
    assert_eq!(w.state, WorkflowState::Cancelled);
}

#[test]
fn workflow_metadata() {
    let mut w = Workflow::new("w");
    w.set_metadata("env", "prod");
    w.set_metadata("user", "alice");
    assert_eq!(w.metadata.get("env"), Some(&"prod".to_string()));
    assert_eq!(w.metadata.get("user"), Some(&"alice".to_string()));
}

#[test]
fn workflow_metadata_overwrite() {
    let mut w = Workflow::new("w");
    w.set_metadata("key", "v1");
    w.set_metadata("key", "v2");
    assert_eq!(w.metadata.get("key"), Some(&"v2".to_string()));
}

#[test]
fn workflow_updated_at_changes() {
    let mut w = Workflow::new("w");
    let created = w.updated_at;
    std::thread::sleep(std::time::Duration::from_millis(10));
    w.start();
    assert!(w.updated_at >= created);
}

// --- Task tests ---

#[test]
fn task_new_is_pending() {
    let t = Task::new("step-1", 1);
    assert_eq!(t.name, "step-1");
    assert_eq!(t.order, 1);
    assert_eq!(t.state, WorkflowState::Pending);
    assert!(t.error.is_none());
}

#[test]
fn task_state_transitions() {
    let mut t = Task::new("t", 0);
    t.start();
    assert_eq!(t.state, WorkflowState::Running);

    t.complete();
    assert_eq!(t.state, WorkflowState::Completed);
}

#[test]
fn task_fail_with_error() {
    let mut t = Task::new("t", 0);
    t.fail("disk full");
    assert_eq!(t.state, WorkflowState::Failed);
    assert_eq!(t.error, Some("disk full".to_string()));
}

// --- WorkflowEngine tests ---

#[test]
fn engine_new_and_default() {
    let e1 = WorkflowEngine::new();
    let e2 = WorkflowEngine::default();
    assert!(e1.workflow_ids().is_empty());
    assert!(e2.workflow_ids().is_empty());
}

#[test]
fn engine_create_workflow() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("my-workflow");
    assert_eq!(w.name, "my-workflow");
    assert!(engine.get_workflow(&w.id).is_some());
}

#[test]
fn engine_get_nonexistent_workflow() {
    let engine = WorkflowEngine::new();
    assert!(engine.get_workflow("nonexistent").is_none());
}

#[test]
fn engine_start_workflow() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");
    let state = engine.start_workflow(&w.id);
    assert_eq!(state, Some(WorkflowState::Running));
}

#[test]
fn engine_complete_workflow() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");
    engine.start_workflow(&w.id);
    let state = engine.complete_workflow(&w.id);
    assert_eq!(state, Some(WorkflowState::Completed));
}

#[test]
fn engine_fail_workflow() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");
    let state = engine.fail_workflow(&w.id);
    assert_eq!(state, Some(WorkflowState::Failed));
}

#[test]
fn engine_operations_on_nonexistent_workflow() {
    let engine = WorkflowEngine::new();
    assert!(engine.start_workflow("x").is_none());
    assert!(engine.complete_workflow("x").is_none());
    assert!(engine.fail_workflow("x").is_none());
}

#[test]
fn engine_add_and_get_tasks() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");

    let t1 = engine.add_task(&w.id, "Step 1", 1).unwrap();
    let t2 = engine.add_task(&w.id, "Step 2", 2).unwrap();

    let tasks = engine.get_tasks(&w.id).unwrap();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].name, t1.name);
    assert_eq!(tasks[1].name, t2.name);
}

#[test]
fn engine_add_task_to_nonexistent_workflow() {
    let engine = WorkflowEngine::new();
    assert!(engine.add_task("nonexistent", "t", 0).is_none());
}

#[test]
fn engine_get_tasks_nonexistent() {
    let engine = WorkflowEngine::new();
    assert!(engine.get_tasks("nonexistent").is_none());
}

#[test]
fn engine_task_transitions() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");
    let t = engine.add_task(&w.id, "t", 0).unwrap();

    assert!(engine.start_task(&w.id, &t.id));
    assert!(engine.complete_task(&w.id, &t.id));

    let tasks = engine.get_tasks(&w.id).unwrap();
    assert_eq!(tasks[0].state, WorkflowState::Completed);
}

#[test]
fn engine_task_transition_nonexistent_returns_false() {
    let engine = WorkflowEngine::new();
    let w = engine.create_workflow("w");
    assert!(!engine.start_task(&w.id, "no-task"));
    assert!(!engine.complete_task(&w.id, "no-task"));
    assert!(!engine.start_task("no-wf", "no-task"));
}

#[test]
fn engine_workflow_ids() {
    let engine = WorkflowEngine::new();
    let w1 = engine.create_workflow("a");
    let w2 = engine.create_workflow("b");
    let ids = engine.workflow_ids();
    assert!(ids.contains(&w1.id));
    assert!(ids.contains(&w2.id));
}

#[test]
fn engine_workflows_by_state() {
    let engine = WorkflowEngine::new();
    let w1 = engine.create_workflow("a");
    let w2 = engine.create_workflow("b");
    engine.create_workflow("c");

    engine.start_workflow(&w1.id);
    engine.start_workflow(&w2.id);
    engine.complete_workflow(&w2.id);

    let running = engine.workflows_by_state(WorkflowState::Running);
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].id, w1.id);

    let completed = engine.workflows_by_state(WorkflowState::Completed);
    assert_eq!(completed.len(), 1);

    let pending = engine.workflows_by_state(WorkflowState::Pending);
    assert_eq!(pending.len(), 1);
}

// --- Thread safety ---

#[test]
fn engine_is_thread_safe() {
    use std::sync::Arc;
    use std::thread;

    let engine = Arc::new(WorkflowEngine::new());
    let mut handles = vec![];

    for i in 0..5 {
        let engine = engine.clone();
        handles.push(thread::spawn(move || {
            let w = engine.create_workflow(&format!("wf-{}", i));
            engine.start_workflow(&w.id);
            engine.add_task(&w.id, "task", 0);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(engine.workflow_ids().len(), 5);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn workflow_name_preserved(name in "[a-zA-Z0-9\\-]{1,50}") {
            let w = Workflow::new(&name);
            prop_assert_eq!(&w.name, &name);
            prop_assert_eq!(w.state, WorkflowState::Pending);
        }

        #[test]
        fn task_order_preserved(order in 0u32..1000) {
            let t = Task::new("t", order);
            prop_assert_eq!(t.order, order);
        }

        #[test]
        fn engine_n_workflows(n in 0usize..20) {
            let engine = WorkflowEngine::new();
            for i in 0..n {
                engine.create_workflow(&format!("w-{}", i));
            }
            prop_assert_eq!(engine.workflow_ids().len(), n);
        }
    }
}
