//! Authentication and authorization utilities for shiplog.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Permission levels for shiplog operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    /// Read-only access
    #[default]
    Read,
    /// Read and write access
    Write,
    /// Full administrative access
    Admin,
}

/// Represents an authenticated user or service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl Identity {
    /// Create a new identity with the given id, name and permissions
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            permissions: HashSet::new(),
        }
    }

    /// Add a permission to this identity
    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self
    }

    /// Check if this identity has the given permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Check if this identity has at least the given permission level
    pub fn has_min_permission(&self, required: Permission) -> bool {
        let level = |p: &Permission| match p {
            Permission::Read => 0,
            Permission::Write => 1,
            Permission::Admin => 2,
        };

        self.permissions
            .iter()
            .any(|p| level(p) >= level(&required))
    }
}

/// Authorization context for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub identity: Identity,
    pub resource: Option<String>,
}

impl AuthContext {
    /// Create a new authorization context
    pub fn new(identity: Identity) -> Self {
        Self {
            identity,
            resource: None,
        }
    }

    /// Create a new authorization context with a resource
    pub fn with_resource(identity: Identity, resource: impl Into<String>) -> Self {
        Self {
            identity,
            resource: Some(resource.into()),
        }
    }

    /// Check if the current identity can perform an action with the given permission
    pub fn can(&self, permission: Permission) -> bool {
        self.identity.has_min_permission(permission)
    }
}

/// API token for programmatic access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub token: String,
    pub identity: Identity,
    pub expires_at: Option<i64>,
}

impl ApiToken {
    /// Create a new API token
    pub fn new(token: impl Into<String>, identity: Identity) -> Self {
        Self {
            token: token.into(),
            identity,
            expires_at: None,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_permissions() {
        let identity = Identity::new("user1", "Test User")
            .with_permission(Permission::Read)
            .with_permission(Permission::Write);

        assert!(identity.has_permission(Permission::Read));
        assert!(identity.has_permission(Permission::Write));
        assert!(!identity.has_permission(Permission::Admin));
    }

    #[test]
    fn identity_min_permission() {
        let read_only = Identity::new("user1", "Reader").with_permission(Permission::Read);

        let writer = Identity::new("user2", "Writer").with_permission(Permission::Write);

        assert!(read_only.has_min_permission(Permission::Read));
        assert!(!read_only.has_min_permission(Permission::Write));
        assert!(writer.has_min_permission(Permission::Read));
        assert!(writer.has_min_permission(Permission::Write));
    }

    #[test]
    fn auth_context_can() {
        let identity = Identity::new("user1", "Test User").with_permission(Permission::Write);

        let ctx = AuthContext::new(identity);

        assert!(ctx.can(Permission::Read));
        assert!(ctx.can(Permission::Write));
        assert!(!ctx.can(Permission::Admin));
    }

    #[test]
    fn api_token_expiry() {
        let token = ApiToken::new("test_token", Identity::new("user1", "User"));

        // No expiry set, should not be expired
        assert!(!token.is_expired(1000));

        // With expiry in the future
        let future_token = ApiToken {
            token: "future".to_string(),
            identity: Identity::new("user1", "User"),
            expires_at: Some(2000),
        };

        assert!(!future_token.is_expired(1000));
        assert!(future_token.is_expired(3000));
    }
}
