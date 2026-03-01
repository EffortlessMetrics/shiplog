use shiplog_ingest_gitlab::MrState;

#[test]
fn mr_state_as_str() {
    assert_eq!(MrState::Opened.as_str(), "opened");
    assert_eq!(MrState::Merged.as_str(), "merged");
    assert_eq!(MrState::Closed.as_str(), "closed");
    assert_eq!(MrState::All.as_str(), "all");
}

#[test]
fn mr_state_from_str() {
    assert_eq!("opened".parse::<MrState>().unwrap(), MrState::Opened);
    assert_eq!("merged".parse::<MrState>().unwrap(), MrState::Merged);
    assert_eq!("closed".parse::<MrState>().unwrap(), MrState::Closed);
    assert_eq!("all".parse::<MrState>().unwrap(), MrState::All);
}

#[test]
fn mr_state_from_str_case_insensitive() {
    assert_eq!("MERGED".parse::<MrState>().unwrap(), MrState::Merged);
    assert_eq!("Opened".parse::<MrState>().unwrap(), MrState::Opened);
}

#[test]
fn mr_state_from_str_invalid() {
    assert!("invalid".parse::<MrState>().is_err());
    assert!("".parse::<MrState>().is_err());
}

#[test]
fn mr_state_equality() {
    assert_eq!(MrState::Opened, MrState::Opened);
    assert_ne!(MrState::Opened, MrState::Closed);
}
