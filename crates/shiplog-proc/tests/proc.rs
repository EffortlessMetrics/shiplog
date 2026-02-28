use shiplog_proc::*;

// ── pid ───────────────────────────────────────────────────────────────

#[test]
fn pid_is_positive() {
    assert!(pid() > 0);
}

#[test]
fn pid_is_stable_within_process() {
    let p1 = pid();
    let p2 = pid();
    assert_eq!(p1, p2, "pid should be stable within the same process");
}

// ── args ──────────────────────────────────────────────────────────────

#[test]
fn args_is_nonempty() {
    let a = args();
    assert!(!a.is_empty(), "args should contain at least the executable");
}

#[test]
fn args_first_is_executable() {
    let a = args();
    assert!(
        !a[0].is_empty(),
        "first argument should be a non-empty executable path"
    );
}

// ── num_cpus ──────────────────────────────────────────────────────────

#[test]
fn num_cpus_at_least_one() {
    assert!(num_cpus() >= 1);
}

#[test]
fn num_cpus_stable() {
    let c1 = num_cpus();
    let c2 = num_cpus();
    assert_eq!(c1, c2, "CPU count should be stable");
}

// ── process_name ──────────────────────────────────────────────────────

#[test]
fn process_name_not_empty() {
    let name = process_name();
    assert!(!name.is_empty());
}

#[test]
fn process_name_does_not_contain_path_separator() {
    let name = process_name();
    // process_name() should be just the filename, not a full path
    assert!(!name.contains('/') || cfg!(not(unix)));
}

// ── exe_path ──────────────────────────────────────────────────────────

#[test]
fn exe_path_ok() {
    let path = exe_path();
    assert!(path.is_ok());
}

#[test]
fn exe_path_exists() {
    let path = exe_path().unwrap();
    assert!(path.exists(), "exe path should exist on disk");
}

#[test]
fn exe_path_is_absolute() {
    let path = exe_path().unwrap();
    assert!(path.is_absolute(), "exe path should be absolute");
}

// ── hostname ──────────────────────────────────────────────────────────

#[test]
fn hostname_nonempty_if_present() {
    if let Some(h) = hostname() {
        assert!(!h.is_empty());
    }
}

// ── in_container ──────────────────────────────────────────────────────

#[test]
fn in_container_returns_bool() {
    // Just verify it doesn't panic
    let _result = in_container();
}

// ── parent_pid (platform-dependent) ───────────────────────────────────

#[test]
fn parent_pid_non_windows() {
    // On non-unix (e.g., Windows), parent_pid should return None
    #[cfg(not(unix))]
    {
        assert!(parent_pid().is_none());
    }
}

// ── uid / username (platform-dependent) ───────────────────────────────

#[test]
fn uid_none_on_windows() {
    #[cfg(not(unix))]
    {
        assert!(uid().is_none());
    }
}

#[test]
fn username_none_on_windows() {
    #[cfg(not(unix))]
    {
        assert!(username().is_none());
    }
}

// ── total_memory ──────────────────────────────────────────────────────

#[test]
fn total_memory_none_on_windows() {
    #[cfg(not(unix))]
    {
        assert!(total_memory().is_none());
    }
}

// ── pid_env_var ───────────────────────────────────────────────────────

#[test]
fn pid_env_var_platform_specific() {
    #[cfg(windows)]
    {
        assert_eq!(pid_env_var(), Some("PID"));
    }
    #[cfg(not(windows))]
    {
        assert_eq!(pid_env_var(), None);
    }
}

// ── ProcError ─────────────────────────────────────────────────────────

#[test]
fn proc_error_display() {
    let err = ProcError::IoError("test failure".to_string());
    assert_eq!(format!("{}", err), "Process error: test failure");
}

#[test]
fn proc_error_eq() {
    let e1 = ProcError::IoError("a".to_string());
    let e2 = ProcError::IoError("a".to_string());
    let e3 = ProcError::IoError("b".to_string());
    assert_eq!(e1, e2);
    assert_ne!(e1, e3);
}

#[test]
fn proc_error_clone() {
    let err = ProcError::IoError("msg".to_string());
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

#[test]
fn proc_error_debug() {
    let err = ProcError::IoError("debug test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("debug test"));
}

#[test]
fn proc_error_is_std_error() {
    let err = ProcError::IoError("test".to_string());
    let _: &dyn std::error::Error = &err;
}
