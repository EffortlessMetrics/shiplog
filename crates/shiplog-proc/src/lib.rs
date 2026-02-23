//! Process utilities for shiplog.
//!
//! This crate provides process utilities for the shiplog ecosystem.

use std::process;

/// Returns the process ID of the current process.
pub fn pid() -> u32 {
    process::id()
}

/// Returns the parent process ID of the current process.
pub fn parent_pid() -> Option<u32> {
    // Note: This is not available on all platforms
    #[cfg(unix)]
    {
        process::Command::new("sh")
            .arg("-c")
            .arg("echo $PPID")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Returns the current process's executable path.
pub fn exe_path() -> Result<std::path::PathBuf, ProcError> {
    std::env::current_exe().map_err(|e| ProcError::IoError(e.to_string()))
}

/// Returns the current process's arguments.
pub fn args() -> Vec<String> {
    std::env::args().collect()
}

/// Returns the number of CPU cores available.
pub fn num_cpus() -> usize {
    // Use thread::available_parallelism if available
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Returns the process name.
pub fn process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Returns the user ID of the current process.
pub fn uid() -> Option<u32> {
    #[cfg(unix)]
    {
        Some(
            process::Command::new("id")
                .arg("-u")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0),
        )
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Returns the user name of the current process.
pub fn username() -> Option<String> {
    #[cfg(unix)]
    {
        process::Command::new("whoami")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Returns the hostname.
pub fn hostname() -> Option<String> {
    std::env::var("HOSTNAME").ok().or({
        #[cfg(unix)]
        {
            process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
        #[cfg(not(unix))]
        {
            None
        }
    })
}

/// Exits the process with the specified exit code.
pub fn exit(code: i32) -> ! {
    process::exit(code)
}

/// Returns the environment variable that contains the process ID (if on Windows).
pub fn pid_env_var() -> Option<&'static str> {
    #[cfg(windows)]
    {
        Some("PID")
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Checks if running in a container.
pub fn in_container() -> bool {
    // Check for common container environment indicators
    std::env::var("DOCKER_CONTAINER").is_ok()
        || std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
        || std::path::Path::new("/.dockerenv").exists()
}

/// Returns the total memory in bytes (if determinable).
pub fn total_memory() -> Option<u64> {
    #[cfg(unix)]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
                    .map(|kb| kb * 1024) // Convert from KB to bytes
            })
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Error type for process operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcError {
    IoError(String),
}

impl std::fmt::Display for ProcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcError::IoError(msg) => write!(f, "Process error: {}", msg),
        }
    }
}

impl std::error::Error for ProcError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid() {
        let pid = pid();
        assert!(pid > 0);
    }

    #[test]
    fn test_args() {
        let args = args();
        // At minimum should have one argument (the executable)
        assert!(!args.is_empty());
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus >= 1);
    }

    #[test]
    fn test_process_name() {
        let name = process_name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_hostname() {
        let hostname = hostname();
        // Hostname might not be set in some test environments
        if let Some(h) = hostname {
            assert!(!h.is_empty());
        }
    }

    #[test]
    fn test_in_container() {
        // Should return a boolean
        let _ = in_container();
    }

    #[test]
    fn test_exe_path() {
        let path = exe_path();
        assert!(path.is_ok());
    }

    #[test]
    fn test_total_memory() {
        // This might not be available on all systems
        let mem = total_memory();
        if let Some(m) = mem {
            assert!(m > 0);
        }
    }

    #[test]
    fn test_uid() {
        #[cfg(unix)]
        {
            let uid = uid();
            assert!(uid.is_some());
        }
    }

    #[test]
    fn test_username() {
        #[cfg(unix)]
        {
            let user = username();
            assert!(user.is_some());
        }
    }

    #[test]
    fn test_parent_pid() {
        #[cfg(unix)]
        {
            let ppid = parent_pid();
            // May not be available in all test environments
            if let Some(p) = ppid {
                assert!(p > 0);
            }
        }
    }
}
