use shiplog_env::*;

#[test]
fn get_var_nonexistent_returns_none() {
    assert!(get_var("SHIPLOG_INTEG_TEST_MISSING_12345").is_none());
}

#[test]
fn get_var_or_returns_default() {
    assert_eq!(
        get_var_or("SHIPLOG_INTEG_TEST_MISSING_12345", "fallback"),
        "fallback"
    );
}

#[test]
fn get_var_as_nonexistent() {
    let result: Option<u32> = get_var_as("SHIPLOG_INTEG_TEST_MISSING_12345");
    assert!(result.is_none());
}

#[test]
fn is_set_path_exists() {
    assert!(is_set("PATH"));
    assert!(!is_set("SHIPLOG_INTEG_TEST_MISSING_12345"));
}

#[test]
fn truthy_falsy_missing_var() {
    assert!(!is_truthy("SHIPLOG_INTEG_TEST_MISSING_12345"));
    assert!(!is_falsy("SHIPLOG_INTEG_TEST_MISSING_12345"));
}

#[test]
fn vars_returns_some() {
    let vs: Vec<_> = vars().take(3).collect();
    assert!(!vs.is_empty());
}

#[test]
fn vars_with_prefix_matches() {
    let path_vars: Vec<_> = vars_with_prefix("PATH").collect();
    assert!(!path_vars.is_empty());
}

#[test]
fn current_dir_succeeds() {
    assert!(current_dir().is_ok());
}

#[test]
fn path_separator_platform() {
    let sep = path_separator();
    assert!(sep == ';' || sep == ':');
}

#[test]
fn env_error_display() {
    let err = EnvError::IoError("test".to_string());
    assert!(err.to_string().contains("test"));
}
