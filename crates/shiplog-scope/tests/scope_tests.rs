use shiplog_scope::*;
use std::sync::Arc;

#[test]
fn scope_config_defaults() {
    let config = ScopeConfig::default();
    assert_eq!(config.max_tasks, 100);
    assert!(!config.propagate_panic);
    assert!(config.await_all);
}

#[test]
fn scope_builder_fluent() {
    let config = ScopeBuilder::new()
        .max_tasks(10)
        .propagate_panic(true)
        .await_all(false)
        .build();
    assert_eq!(config.max_tasks, 10);
    assert!(config.propagate_panic);
    assert!(!config.await_all);
}

#[test]
fn task_scope_lifecycle() {
    let scope = TaskScope::default_scope();
    assert_eq!(scope.active_count(), 0);
    assert_eq!(scope.completed_count(), 0);
    assert!(!scope.has_active());

    assert!(scope.task_started());
    assert_eq!(scope.active_count(), 1);
    assert!(scope.has_active());

    scope.task_completed();
    assert_eq!(scope.active_count(), 0);
    assert_eq!(scope.completed_count(), 1);
}

#[test]
fn task_scope_max_tasks_enforced() {
    let config = ScopeConfig {
        max_tasks: 2,
        ..Default::default()
    };
    let scope = TaskScope::new(config);
    assert!(scope.task_started());
    assert!(scope.task_started());
    assert!(!scope.task_started());
}

#[test]
fn task_scope_error_recording() {
    let scope = TaskScope::default_scope();
    assert!(!scope.has_errors());
    scope.record_error("err1".to_string());
    scope.record_error("err2".to_string());
    assert!(scope.has_errors());
    assert_eq!(scope.errors().len(), 2);
}

#[test]
fn scoped_task_with_result() {
    let scope = Arc::new(TaskScope::default_scope());
    let task = scope.spawn(|_| 42).unwrap();
    assert_eq!(task.result(), Some(42));
}

#[test]
fn scope_ext_returns_none_when_full() {
    let config = ScopeConfig {
        max_tasks: 0,
        ..Default::default()
    };
    let scope = Arc::new(TaskScope::new(config));
    assert!(scope.spawn(|_| 1).is_none());
}
