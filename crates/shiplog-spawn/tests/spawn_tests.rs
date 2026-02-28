use shiplog_spawn::*;
use std::sync::Arc;

#[test]
fn spawn_config_defaults() {
    let config = SpawnConfig::default();
    assert_eq!(config.max_workers, 4);
    assert_eq!(config.queue_size, 100);
    assert_eq!(config.name_prefix, "shiplog-task");
}

#[test]
fn spawn_builder_fluent() {
    let config = SpawnBuilder::new()
        .max_workers(8)
        .queue_size(50)
        .name_prefix("test".to_string())
        .build();
    assert_eq!(config.max_workers, 8);
    assert_eq!(config.queue_size, 50);
}

#[test]
fn task_handle_lifecycle() {
    let handle: TaskHandle<i32> = TaskHandle::new(1);
    assert_eq!(handle.id(), 1);
    assert_eq!(handle.status(), TaskStatus::Pending);
    assert!(!handle.is_completed());
    assert!(!handle.is_failed());

    handle.set_result(42);
    handle.set_status(TaskStatus::Completed);
    assert!(handle.is_completed());
    assert_eq!(handle.result(), Some(42));
}

#[test]
fn task_handle_failed_status() {
    let handle: TaskHandle<i32> = TaskHandle::new(0);
    handle.set_status(TaskStatus::Failed("boom".to_string()));
    assert!(handle.is_failed());
    assert!(!handle.is_completed());
}

#[test]
fn spawner_basic() {
    let s = Spawner::new_spawner();
    assert_eq!(s.active_count(), 0);
    let h = s.spawn(|| 42).unwrap();
    assert_eq!(h.result(), Some(42));
    assert!(h.is_completed());
}

#[test]
fn spawner_increments_id() {
    let s = Spawner::new_spawner();
    let h1 = s.spawn(|| 1).unwrap();
    let h2 = s.spawn(|| 2).unwrap();
    assert_eq!(h1.id(), 0);
    assert_eq!(h2.id(), 1);
}

#[test]
fn shared_spawner_creation() {
    let s: SharedSpawner = spawner();
    let h = s.spawn(|| "hello").unwrap();
    assert_eq!(h.result(), Some("hello"));
}

#[test]
fn shared_spawner_is_arc() {
    let s: SharedSpawner = spawner();
    let s2: Arc<Spawner> = s.clone();
    let _ = s2.spawn(|| 1);
}
