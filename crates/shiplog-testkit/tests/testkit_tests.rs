use shiplog_schema::event::{EventKind, PullRequestState};
use shiplog_testkit::*;

#[test]
fn pr_event_creation() {
    let event = pr_event("owner/repo", 42, "Fix bug");
    assert_eq!(event.kind, EventKind::PullRequest);
    assert_eq!(event.repo.full_name, "owner/repo");
    if let shiplog_schema::event::EventPayload::PullRequest(pr) = &event.payload {
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Fix bug");
        assert_eq!(pr.state, PullRequestState::Merged);
    } else {
        panic!("expected PullRequest payload");
    }
}

#[test]
fn pr_event_different_repos() {
    let e1 = pr_event("a/b", 1, "one");
    let e2 = pr_event("c/d", 2, "two");
    assert_ne!(e1.id, e2.id);
    assert_ne!(e1.repo.full_name, e2.repo.full_name);
}

#[test]
fn pr_event_has_stable_id() {
    let e1 = pr_event("owner/repo", 1, "title");
    let e2 = pr_event("owner/repo", 1, "title");
    assert_eq!(e1.id, e2.id);
}
