use chrono::{Duration, Utc};
use shiplog_scheduler::{DayOfWeek, ScheduleFrequency, ScheduledTask, Scheduler};

// --- ScheduleFrequency next_occurrence tests ---

#[test]
fn once_future_returns_some() {
    let future = Utc::now() + Duration::hours(1);
    let freq = ScheduleFrequency::Once(future);
    assert_eq!(freq.next_occurrence(&Utc::now()), Some(future));
}

#[test]
fn once_past_returns_none() {
    let past = Utc::now() - Duration::hours(1);
    let freq = ScheduleFrequency::Once(past);
    assert!(freq.next_occurrence(&Utc::now()).is_none());
}

#[test]
fn daily_returns_correct_time() {
    let freq = ScheduleFrequency::Daily {
        hour: 14,
        minute: 30,
    };
    let now = Utc::now();
    let next = freq.next_occurrence(&now).unwrap();
    assert_eq!(next.format("%H:%M").to_string(), "14:30");
}

#[test]
fn interval_minutes() {
    let freq = ScheduleFrequency::IntervalMinutes(60);
    let now = Utc::now();
    let next = freq.next_occurrence(&now).unwrap();
    let diff = (next - now).num_minutes();
    assert_eq!(diff, 60);
}

#[test]
fn interval_hours() {
    let freq = ScheduleFrequency::IntervalHours(2);
    let now = Utc::now();
    let next = freq.next_occurrence(&now).unwrap();
    let diff = (next - now).num_hours();
    assert_eq!(diff, 2);
}

#[test]
fn weekly_returns_correct_day() {
    let freq = ScheduleFrequency::Weekly {
        day: DayOfWeek::Monday,
        hour: 9,
        minute: 0,
    };
    let now = Utc::now();
    let next = freq.next_occurrence(&now);
    assert!(next.is_some());
    let next = next.unwrap();
    assert_eq!(next.format("%H:%M").to_string(), "09:00");
}

// --- DayOfWeek tests ---

#[test]
fn day_of_week_display_all() {
    let days = [
        (DayOfWeek::Monday, "Monday"),
        (DayOfWeek::Tuesday, "Tuesday"),
        (DayOfWeek::Wednesday, "Wednesday"),
        (DayOfWeek::Thursday, "Thursday"),
        (DayOfWeek::Friday, "Friday"),
        (DayOfWeek::Saturday, "Saturday"),
        (DayOfWeek::Sunday, "Sunday"),
    ];
    for (day, label) in days {
        assert_eq!(format!("{}", day), label);
    }
}

#[test]
fn day_of_week_equality() {
    assert_eq!(DayOfWeek::Monday, DayOfWeek::Monday);
    assert_ne!(DayOfWeek::Monday, DayOfWeek::Friday);
}

// --- ScheduledTask tests ---

#[test]
fn scheduled_task_creation() {
    let task = ScheduledTask::new(
        "daily-sync",
        "Daily Sync",
        ScheduleFrequency::Daily { hour: 2, minute: 0 },
    );
    assert_eq!(task.id, "daily-sync");
    assert_eq!(task.name, "Daily Sync");
    assert!(task.enabled);
    assert!(task.last_run.is_none());
    assert!(task.next_run.is_some());
}

#[test]
fn scheduled_task_is_due_when_disabled() {
    let mut task = ScheduledTask::new("t", "T", ScheduleFrequency::IntervalMinutes(0));
    task.enabled = false;
    assert!(!task.is_due());
}

#[test]
fn scheduled_task_is_due_when_next_run_none() {
    // Once task in the past has next_run = None
    let past = Utc::now() - Duration::hours(1);
    let task = ScheduledTask::new("t", "T", ScheduleFrequency::Once(past));
    assert!(!task.is_due());
}

#[test]
fn scheduled_task_mark_run_updates_fields() {
    let mut task = ScheduledTask::new("t", "T", ScheduleFrequency::IntervalMinutes(30));
    assert!(task.last_run.is_none());

    task.mark_run();
    assert!(task.last_run.is_some());
    assert!(task.next_run.is_some());
}

// --- Scheduler tests ---

#[test]
fn scheduler_new_is_empty() {
    let s = Scheduler::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
}

#[test]
fn scheduler_default_is_empty() {
    let s = Scheduler::default();
    assert!(s.is_empty());
}

#[test]
fn scheduler_add_and_get() {
    let mut s = Scheduler::new();
    let task = ScheduledTask::new("t1", "Task 1", ScheduleFrequency::IntervalMinutes(10));
    s.add_task(task);
    assert_eq!(s.len(), 1);
    assert!(s.get_task("t1").is_some());
    assert!(s.get_task("nonexistent").is_none());
}

#[test]
fn scheduler_get_task_mut() {
    let mut s = Scheduler::new();
    s.add_task(ScheduledTask::new(
        "t1",
        "Task",
        ScheduleFrequency::IntervalHours(1),
    ));
    let task = s.get_task_mut("t1").unwrap();
    task.enabled = false;
    assert!(!s.get_task("t1").unwrap().enabled);
}

#[test]
fn scheduler_mark_task_run() {
    let mut s = Scheduler::new();
    s.add_task(ScheduledTask::new(
        "t1",
        "Task",
        ScheduleFrequency::IntervalMinutes(10),
    ));
    assert!(s.mark_task_run("t1"));
    assert!(!s.mark_task_run("nonexistent"));

    let task = s.get_task("t1").unwrap();
    assert!(task.last_run.is_some());
}

#[test]
fn scheduler_get_due_tasks() {
    let mut s = Scheduler::new();
    // Future task shouldn't be due
    s.add_task(ScheduledTask::new(
        "future",
        "Future",
        ScheduleFrequency::Once(Utc::now() + Duration::hours(1)),
    ));
    assert!(s.get_due_tasks().is_empty());
}

#[test]
fn scheduler_multiple_tasks() {
    let mut s = Scheduler::new();
    for i in 0..5 {
        s.add_task(ScheduledTask::new(
            format!("t-{}", i),
            format!("Task {}", i),
            ScheduleFrequency::IntervalHours(1),
        ));
    }
    assert_eq!(s.len(), 5);
    for i in 0..5 {
        assert!(s.get_task(&format!("t-{}", i)).is_some());
    }
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn interval_minutes_is_always_future(mins in 1u32..10000) {
            let freq = ScheduleFrequency::IntervalMinutes(mins);
            let now = Utc::now();
            let next = freq.next_occurrence(&now);
            prop_assert!(next.is_some());
            prop_assert!(next.unwrap() > now);
        }

        #[test]
        fn interval_hours_is_always_future(hrs in 1u32..1000) {
            let freq = ScheduleFrequency::IntervalHours(hrs);
            let now = Utc::now();
            let next = freq.next_occurrence(&now);
            prop_assert!(next.is_some());
            prop_assert!(next.unwrap() > now);
        }

        #[test]
        fn scheduler_n_tasks(n in 0usize..30) {
            let mut s = Scheduler::new();
            for i in 0..n {
                s.add_task(ScheduledTask::new(
                    format!("t-{}", i),
                    "T",
                    ScheduleFrequency::IntervalHours(1),
                ));
            }
            prop_assert_eq!(s.len(), n);
        }
    }
}
