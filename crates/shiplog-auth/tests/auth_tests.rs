use shiplog_auth::*;

#[test]
fn identity_creation() {
    let id = Identity::new("u1", "User One");
    assert_eq!(id.id, "u1");
    assert_eq!(id.name, "User One");
    assert!(id.permissions.is_empty());
}

#[test]
fn identity_with_permissions() {
    let id = Identity::new("u1", "User")
        .with_permission(Permission::Read)
        .with_permission(Permission::Write);
    assert!(id.has_permission(Permission::Read));
    assert!(id.has_permission(Permission::Write));
    assert!(!id.has_permission(Permission::Admin));
}

#[test]
fn identity_min_permission() {
    let reader = Identity::new("u1", "R").with_permission(Permission::Read);
    assert!(reader.has_min_permission(Permission::Read));
    assert!(!reader.has_min_permission(Permission::Write));

    let writer = Identity::new("u2", "W").with_permission(Permission::Write);
    assert!(writer.has_min_permission(Permission::Read));
    assert!(writer.has_min_permission(Permission::Write));
    assert!(!writer.has_min_permission(Permission::Admin));

    let admin = Identity::new("u3", "A").with_permission(Permission::Admin);
    assert!(admin.has_min_permission(Permission::Read));
    assert!(admin.has_min_permission(Permission::Admin));
}

#[test]
fn auth_context_can() {
    let id = Identity::new("u1", "U").with_permission(Permission::Write);
    let ctx = AuthContext::new(id);
    assert!(ctx.can(Permission::Read));
    assert!(ctx.can(Permission::Write));
    assert!(!ctx.can(Permission::Admin));
}

#[test]
fn auth_context_with_resource() {
    let id = Identity::new("u1", "U");
    let ctx = AuthContext::with_resource(id, "repo/shiplog");
    assert_eq!(ctx.resource, Some("repo/shiplog".to_string()));
}

#[test]
fn api_token_not_expired_without_expiry() {
    let token = ApiToken::new("tok", Identity::new("u1", "U"));
    assert!(!token.is_expired(999999));
}

#[test]
fn api_token_expiry() {
    let token = ApiToken {
        token: "tok".to_string(),
        identity: Identity::new("u1", "U"),
        expires_at: Some(1000),
    };
    assert!(!token.is_expired(500));
    assert!(!token.is_expired(1000));
    assert!(token.is_expired(1001));
}

#[test]
fn permission_default() {
    assert_eq!(Permission::default(), Permission::Read);
}

#[test]
fn identity_serde_roundtrip() {
    let id = Identity::new("u1", "User").with_permission(Permission::Read);
    let json = serde_json::to_string(&id).unwrap();
    let de: Identity = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, "u1");
    assert!(de.permissions.contains(&Permission::Read));
}
