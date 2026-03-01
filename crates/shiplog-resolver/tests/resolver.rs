//! Integration tests for shiplog-resolver.

use shiplog_resolver::*;
use std::any::Any;

// ── Custom services for testing ─────────────────────────────────────────────

#[derive(Debug)]
struct DatabaseService {
    connection_string: String,
}

impl DatabaseService {
    fn new(conn: &str) -> Self {
        Self {
            connection_string: conn.to_string(),
        }
    }
}

impl Resolvable for DatabaseService {
    fn get_id(&self) -> &str {
        "database"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct CacheService {
    ttl_seconds: u64,
}

impl CacheService {
    fn new(ttl: u64) -> Self {
        Self { ttl_seconds: ttl }
    }
}

impl Resolvable for CacheService {
    fn get_id(&self) -> &str {
        "cache"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct LogService;

impl LogService {
    fn new() -> Self {
        Self
    }
}

impl Resolvable for LogService {
    fn get_id(&self) -> &str {
        "logger"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_resolver_error_display() {
    let err = ResolverError::new("Service 'database' not registered");
    insta::assert_snapshot!("resolver_error_display", format!("{err}"));
}

#[test]
fn snapshot_service_descriptor() {
    let desc = ServiceDescriptor {
        id: "my-service".to_string(),
        lifetime: ServiceLifetime::Singleton,
    };
    insta::assert_snapshot!("service_descriptor_debug", format!("{desc:?}"));
}

// ── Integration tests: service chains ───────────────────────────────────────

#[test]
fn register_and_resolve_multiple_service_types() {
    let mut container = ServiceContainer::new();
    container.register(SimpleService::new("svc1", "val1"));
    container.register(DatabaseService::new("postgres://localhost"));
    container.register(CacheService::new(3600));
    container.register(LogService::new());

    assert_eq!(container.service_count(), 4);

    let db = container.resolve::<DatabaseService>().unwrap();
    assert_eq!(db.connection_string, "postgres://localhost");

    let cache = container.resolve::<CacheService>().unwrap();
    assert_eq!(cache.ttl_seconds, 3600);

    let svc = container.resolve::<SimpleService>().unwrap();
    assert_eq!(svc.value(), "val1");

    let log = container.resolve::<LogService>();
    assert!(log.is_some());
}

#[test]
fn resolver_resolves_registered_services() {
    let mut container = ServiceContainer::new();
    container.register(DatabaseService::new("sqlite://test.db"));
    container.register(CacheService::new(60));

    let resolver = Resolver::new(container);

    let db: &DatabaseService = resolver.resolve().unwrap();
    assert_eq!(db.connection_string, "sqlite://test.db");

    let cache: &CacheService = resolver.resolve().unwrap();
    assert_eq!(cache.ttl_seconds, 60);
}

#[test]
fn resolver_try_resolve_returns_none_for_unregistered() {
    let container = ServiceContainer::new();
    let resolver = Resolver::new(container);
    assert!(resolver.try_resolve::<DatabaseService>().is_none());
}

#[test]
fn resolver_resolve_returns_error_for_unregistered() {
    let container = ServiceContainer::new();
    let resolver = Resolver::new(container);
    let result: Result<&DatabaseService, _> = resolver.resolve();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ── Builder pattern tests ───────────────────────────────────────────────────

#[test]
fn basic_service_provider_builds_container() {
    let container = BasicServiceProvider::new()
        .with_service(SimpleService::new("s1", "v1"))
        .with_service(DatabaseService::new("test"))
        .with_service(CacheService::new(120))
        .build();

    assert_eq!(container.service_count(), 3);
    assert!(container.has_service("s1"));
    assert!(container.has_service("database"));
    assert!(container.has_service("cache"));
}

#[test]
fn service_provider_trait_provides_by_id() {
    let mut container = ServiceContainer::new();
    container.register(SimpleService::new("finder", "found-it"));

    let provided = container.provide("finder");
    assert!(provided.is_some());
    assert_eq!(provided.unwrap().get_id(), "finder");
}

#[test]
fn service_provider_returns_none_for_missing() {
    let container = ServiceContainer::new();
    assert!(container.provide("nonexistent").is_none());
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_container() {
    let container = ServiceContainer::new();
    assert_eq!(container.service_count(), 0);
    assert!(!container.has_service("anything"));
    assert!(container.resolve::<SimpleService>().is_none());
    assert!(container.get_descriptor("anything").is_none());
}

#[test]
fn duplicate_registration_overwrites() {
    let mut container = ServiceContainer::new();
    container.register(SimpleService::new("svc", "first"));
    container.register(SimpleService::new("svc", "second"));

    // Should have 1 service (overwritten)
    assert_eq!(container.service_count(), 1);
    let resolved = container.resolve::<SimpleService>().unwrap();
    assert_eq!(resolved.value(), "second");
}

#[test]
fn register_with_different_lifetimes() {
    let mut container = ServiceContainer::new();
    container.register_with_lifetime(
        SimpleService::new("transient", "t"),
        ServiceLifetime::Transient,
    );
    container.register_with_lifetime(
        DatabaseService::new("singleton"),
        ServiceLifetime::Singleton,
    );
    container.register_with_lifetime(CacheService::new(0), ServiceLifetime::Scoped);

    let t_desc = container.get_descriptor("transient").unwrap();
    assert!(t_desc.lifetime.is_transient());

    let s_desc = container.get_descriptor("database").unwrap();
    assert!(s_desc.lifetime.is_singleton());

    let sc_desc = container.get_descriptor("cache").unwrap();
    assert!(sc_desc.lifetime.is_scoped());
}

#[test]
fn service_lifetime_exhaustive() {
    let transient = ServiceLifetime::Transient;
    assert!(transient.is_transient());
    assert!(!transient.is_singleton());
    assert!(!transient.is_scoped());

    let singleton = ServiceLifetime::Singleton;
    assert!(!singleton.is_transient());
    assert!(singleton.is_singleton());
    assert!(!singleton.is_scoped());

    let scoped = ServiceLifetime::Scoped;
    assert!(!scoped.is_transient());
    assert!(!scoped.is_singleton());
    assert!(scoped.is_scoped());
}

#[test]
fn resolver_error_is_std_error() {
    let err = ResolverError::new("test");
    let _: &dyn std::error::Error = &err;
}

#[test]
fn default_container_is_empty() {
    let container = ServiceContainer::default();
    assert_eq!(container.service_count(), 0);
}

#[test]
fn default_provider_is_empty() {
    let provider = BasicServiceProvider::default();
    let container = provider.build();
    assert_eq!(container.service_count(), 0);
}

#[test]
fn resolve_wrong_type_returns_none() {
    let mut container = ServiceContainer::new();
    container.register(SimpleService::new("svc", "val"));
    // Try to resolve as DatabaseService when only SimpleService is registered
    assert!(container.resolve::<DatabaseService>().is_none());
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn registered_service_is_resolvable(id in "[a-z]{1,10}", val in "[a-z]{1,20}") {
            let mut container = ServiceContainer::new();
            container.register(SimpleService::new(&id, &val));
            let resolved = container.resolve::<SimpleService>();
            prop_assert!(resolved.is_some());
            prop_assert_eq!(resolved.unwrap().value(), &val);
        }

        #[test]
        fn has_service_after_register(id in "[a-z]{1,10}") {
            let mut container = ServiceContainer::new();
            container.register(SimpleService::new(&id, "v"));
            prop_assert!(container.has_service(&id));
        }

        #[test]
        fn service_count_matches_registrations(count in 1_usize..10) {
            let mut container = ServiceContainer::new();
            for i in 0..count {
                container.register(SimpleService::new(&format!("svc-{i}"), "v"));
            }
            prop_assert_eq!(container.service_count(), count);
        }
    }
}
