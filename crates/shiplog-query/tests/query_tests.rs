use shiplog_query::*;

#[test]
fn parse_empty_query() {
    let q = Query::parse("").unwrap();
    // Empty query should succeed
    let _ = q;
}

#[test]
fn parse_source_query() {
    let q = Query::parse("source:github");
    assert!(q.is_ok());
}

#[test]
fn parse_kind_query() {
    let q = Query::parse("kind:pr");
    assert!(q.is_ok());
}

#[test]
fn parse_actor_query() {
    let q = Query::parse("actor:octocat");
    assert!(q.is_ok());
}

#[test]
fn parse_tag_query() {
    let q = Query::parse("tag:feature");
    assert!(q.is_ok());
}

#[test]
fn parse_repo_query() {
    let q = Query::parse("repo:owner/name");
    assert!(q.is_ok());
}

#[test]
fn parse_date_queries() {
    assert!(Query::parse("since:2025-01-01").is_ok());
    assert!(Query::parse("until:2025-12-31").is_ok());
}

#[test]
fn parse_combined_query() {
    let q = Query::parse("source:github kind:pr actor:octocat");
    assert!(q.is_ok());
}

#[test]
fn parse_unknown_operator_fails() {
    let q = Query::parse("unknown:value");
    assert!(q.is_err());
}

#[test]
fn parse_invalid_syntax() {
    let q = Query::parse("nocolon");
    assert!(q.is_err());
}

#[test]
fn query_error_display() {
    let err = QueryError::SyntaxError("bad".to_string());
    assert!(err.to_string().contains("bad"));
    let err = QueryError::UnknownOperator("foo".to_string());
    assert!(err.to_string().contains("foo"));
    let err = QueryError::InvalidValue("bar".to_string());
    assert!(err.to_string().contains("bar"));
}
