//! Task scheduling utilities for shiplog.
//!
//! Provides scheduling primitives for recurring tasks like
//! nightly sync jobs, weekly reports, and periodic backups.

use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Schedule frequency for recurring tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleFrequency {
    /// Run once at a specific time
    Once(DateTime<Utc>),
    /// Run daily at a specific time
    Daily { hour: u32, minute: u32 },
    /// Run weekly on a specific day
    Weekly {
        day: DayOfWeek,
        hour: u32,
        minute: u32,
    },
    /// Run every N minutes
    IntervalMinutes(u32),
    /// Run every N hours
    IntervalHours(u32),
}

/// Days of the week.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl fmt::Display for DayOfWeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DayOfWeek::Monday => write!(f, "Monday"),
            DayOfWeek::Tuesday => write!(f, "Tuesday"),
            DayOfWeek::Wednesday => write!(f, "Wednesday"),
            DayOfWeek::Thursday => write!(f, "Thursday"),
            DayOfWeek::Friday => write!(f, "Friday"),
            DayOfWeek::Saturday => write!(f, "Saturday"),
            DayOfWeek::Sunday => write!(f, "Sunday"),
        }
    }
}

/// A scheduled task with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub frequency: ScheduleFrequency,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

impl ScheduledTask {
    /// Create a new scheduled task.
    pub fn new(id: impl Into<String>, name: impl Into<String>, frequency: ScheduleFrequency) -> Self {
        let next_run = frequency.next_occurrence(&Utc::now());
        Self {
            id: id.into(),
            name: name.into(),
            frequency,
            enabled: true,
            last_run: None,
            next_run,
        }
    }

    /// Check if the task is due to run.
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }
        match self.next_run {
            Some(next) => Utc::now() >= next,
            None => false,
        }
    }

    /// Mark the task as having run.
    pub fn mark_run(&mut self) {
        self.last_run = Some(Utc::now());
        self.next_run = self.frequency.next_occurrence(&Utc::now());
    }
}

impl ScheduleFrequency {
    /// Calculate the next occurrence after the given time.
    pub fn next_occurrence(&self, from: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            ScheduleFrequency::Once(time) => {
                if *time > *from {
                    Some(*time)
                } else {
                    None
                }
            }
            ScheduleFrequency::Daily { hour, minute } => {
                let mut next = from.date_naive().and_hms_opt(*hour, *minute, 0)?;
                let next = DateTime::<Utc>::from_naive_utc_and_offset(next, Utc);
                if next <= *from {
                    Some(next + Duration::days(1))
                } else {
                    Some(next)
                }
            }
            ScheduleFrequency::Weekly { day, hour, minute } => {
                let day_num = match day {
                    DayOfWeek::Monday => 0,
                    DayOfWeek::Tuesday => 1,
                    DayOfWeek::Wednesday => 2,
                    DayOfWeek::Thursday => 3,
                    DayOfWeek::Friday => 4,
                    DayOfWeek::Saturday => 5,
                    DayOfWeek::Sunday => 6,
                };
                let current_day = from.weekday().num_days_from_monday();
                let days_until = if day_num >= current_day {
                    day_num - current_day
                } else {
                    7 - (current_day - day_num)
                };
                let mut next = (from.date_naive() + Duration::days(days_until as i64))
                    .and_hms_opt(*hour, *minute, 0)?;
                let next = DateTime::<Utc>::from_naive_utc_and_offset(next, Utc);
                if next <= *from {
                    Some(next + Duration::days(7))
                } else {
                    Some(next)
                }
            }
            ScheduleFrequency::IntervalMinutes(interval) => {
                Some(*from + Duration::minutes(*interval as i64))
            }
            ScheduleFrequency::IntervalHours(interval) => {
                Some(*from + Duration::hours(*interval as i64))
            }
        }
    }
}

/// Scheduler for managing multiple tasks.
pub struct Scheduler {
    tasks: Vec<ScheduledTask>,
}

impl Scheduler {
    /// Create a new scheduler.
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Add a task to the scheduler.
    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.push(task);
    }

    /// Get all due tasks.
    pub fn get_due_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.iter().filter(|t| t.is_due()).collect()
    }

    /// Get task by ID.
    pub fn get_task(&self, id: &str) -> Option<&ScheduledTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get mutable task by ID.
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut ScheduledTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Mark a task as having run.
    pub fn mark_task_run(&mut self, id: &str) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            task.mark_run();
            true
        } else {
            false
        }
    }

    /// Get the number of tasks.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if there are no tasks.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_daily_schedule_next_occurrence() {
        let freq = ScheduleFrequency::Daily { hour: 10, minute: 0 };
        let now = Utc::now();
        
        // Should return a time today or tomorrow at 10:00
        let next = freq.next_occurrence(&now);
        assert!(next.is_some());
        
        let next = next.unwrap();
        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 0);
    }

    #[test]
    fn test_interval_minutes() {
        let freq = ScheduleFrequency::IntervalMinutes(30);
        let now = Utc::now();
        
        let next = freq.next_occurrence(&now);
        assert!(next.is_some());
        
        let diff = next.unwrap() - now;
        assert!(diff.num_minutes() >= 29 && diff.num_minutes() <= 31);
    }

    #[test]
    fn test_scheduled_task_creation() {
        let task = ScheduledTask::new(
            "test-task",
            "Test Task",
            ScheduleFrequency::Daily { hour: 9, minute: 0 },
        );
        
        assert_eq!(task.id, "test-task");
        assert_eq!(task.name, "Test Task");
        assert!(task.enabled);
        assert!(task.next_run.is_some());
    }

    #[test]
    fn test_scheduler_add_and_get() {
        let mut scheduler = Scheduler::new();
        
        let task = ScheduledTask::new(
            "task-1",
            "Task 1",
            ScheduleFrequency::Daily { hour: 8, minute: 0 },
        );
        scheduler.add_task(task);
        
        assert_eq!(scheduler.len(), 1);
        assert!(scheduler.get_task("task-1").is_some());
        assert!(scheduler.get_task("nonexistent").is_none());
    }

    #[test]
    fn test_mark_task_run() {
        let mut scheduler = Scheduler::new();
        
        let task = ScheduledTask::new(
            "task-1",
            "Task 1",
            ScheduleFrequency::Daily { hour: 8, minute: 0 },
        );
        scheduler.add_task(task);
        
        // Mark as run
        scheduler.mark_task_run("task-1");
        
        let after_run = scheduler.get_task("task-1").unwrap();
        // Verify last_run was set
        assert!(after_run.last_run.is_some());
        // Verify next_run exists
        assert!(after_run.next_run.is_some());
    }

    #[test]
    fn test_day_of_week_display() {
        assert_eq!(format!("{}", DayOfWeek::Monday), "Monday");
        assert_eq!(format!("{}", DayOfWeek::Sunday), "Sunday");
    }
}
