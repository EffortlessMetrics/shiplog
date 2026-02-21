//! Dependency resolution utilities for shiplog.
//!
//! This crate provides utilities for dependency injection and resolution.

use std::fmt;
use std::any::Any;
use std::collections::HashMap;

/// Trait for resolvable dependencies
pub trait Resolvable: Send + Sync {
    fn get_id(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

/// Service descriptor for registration
#[derive(Debug, Clone)]
pub struct ServiceDescriptor {
    pub id: String,
    pub lifetime: ServiceLifetime,
}

/// Lifetime of a service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLifetime {
    Transient,
    Singleton,
    Scoped,
}

impl ServiceLifetime {
    pub fn is_transient(&self) -> bool {
        matches!(self, ServiceLifetime::Transient)
    }

    pub fn is_singleton(&self) -> bool {
        matches!(self, ServiceLifetime::Singleton)
    }

    pub fn is_scoped(&self) -> bool {
        matches!(self, ServiceLifetime::Scoped)
    }
}

/// Service container for dependency resolution
pub struct ServiceContainer {
    services: HashMap<String, Box<dyn Resolvable>>,
    descriptors: HashMap<String, ServiceDescriptor>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            descriptors: HashMap::new(),
        }
    }

    pub fn register<T: Resolvable + 'static>(&mut self, service: T) -> &mut Self {
        let id = service.get_id().to_string();
        self.descriptors.insert(id.clone(), ServiceDescriptor {
            id: id.clone(),
            lifetime: ServiceLifetime::Singleton,
        });
        self.services.insert(id, Box::new(service));
        self
    }

    pub fn register_with_lifetime<T: Resolvable + 'static>(&mut self, service: T, lifetime: ServiceLifetime) -> &mut Self {
        let id = service.get_id().to_string();
        self.descriptors.insert(id.clone(), ServiceDescriptor {
            id: id.clone(),
            lifetime,
        });
        self.services.insert(id, Box::new(service));
        self
    }

    pub fn resolve<T: Resolvable + 'static>(&self) -> Option<&T> {
        for service in self.services.values() {
            if service.as_any().is::<T>() {
                return service.as_any().downcast_ref::<T>();
            }
        }
        None
    }

    pub fn get_descriptor(&self, id: &str) -> Option<&ServiceDescriptor> {
        self.descriptors.get(id)
    }

    pub fn has_service(&self, id: &str) -> bool {
        self.services.contains_key(id)
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

impl Default for ServiceContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple service implementation
#[derive(Debug)]
pub struct SimpleService {
    id: String,
    value: String,
}

impl SimpleService {
    pub fn new(id: &str, value: &str) -> Self {
        Self {
            id: id.to_string(),
            value: value.to_string(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl Resolvable for SimpleService {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Service provider trait
pub trait ServiceProvider: Send + Sync {
    fn provide(&self, id: &str) -> Option<&dyn Resolvable>;
}

/// Basic service provider implementation
pub struct BasicServiceProvider {
    container: ServiceContainer,
}

impl BasicServiceProvider {
    pub fn new() -> Self {
        Self {
            container: ServiceContainer::new(),
        }
    }

    pub fn with_service<T: Resolvable + 'static>(mut self, service: T) -> Self {
        self.container.register(service);
        self
    }

    pub fn build(self) -> ServiceContainer {
        self.container
    }
}

impl Default for BasicServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceProvider for ServiceContainer {
    fn provide(&self, id: &str) -> Option<&dyn Resolvable> {
        self.services.get(id).map(|s| s.as_ref() as &dyn Resolvable)
    }
}

/// Resolver for looking up services
pub struct Resolver {
    container: ServiceContainer,
}

impl Resolver {
    pub fn new(container: ServiceContainer) -> Self {
        Self { container }
    }

    pub fn resolve<T: Resolvable + 'static>(&self) -> Result<&T, ResolverError> {
        self.container.resolve().ok_or(ResolverError::new("Service not found"))
    }

    pub fn try_resolve<T: Resolvable + 'static>(&self) -> Option<&T> {
        self.container.resolve()
    }
}

/// Errors that can occur during resolution
#[derive(Debug, Clone)]
pub struct ResolverError {
    message: String,
}

impl ResolverError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResolverError: {}", self.message)
    }
}

impl std::error::Error for ResolverError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_lifetime_transient() {
        let lifetime = ServiceLifetime::Transient;
        assert!(lifetime.is_transient());
        assert!(!lifetime.is_singleton());
        assert!(!lifetime.is_scoped());
    }

    #[test]
    fn test_service_lifetime_singleton() {
        let lifetime = ServiceLifetime::Singleton;
        assert!(!lifetime.is_transient());
        assert!(lifetime.is_singleton());
        assert!(!lifetime.is_scoped());
    }

    #[test]
    fn test_service_lifetime_scoped() {
        let lifetime = ServiceLifetime::Scoped;
        assert!(!lifetime.is_transient());
        assert!(!lifetime.is_singleton());
        assert!(lifetime.is_scoped());
    }

    #[test]
    fn test_service_container_new() {
        let container = ServiceContainer::new();
        assert_eq!(container.service_count(), 0);
    }

    #[test]
    fn test_service_container_register() {
        let mut container = ServiceContainer::new();
        container.register(SimpleService::new("service1", "value1"));
        
        assert_eq!(container.service_count(), 1);
        assert!(container.has_service("service1"));
    }

    #[test]
    fn test_service_container_register_with_lifetime() {
        let mut container = ServiceContainer::new();
        container.register_with_lifetime(
            SimpleService::new("service1", "value1"),
            ServiceLifetime::Transient,
        );
        
        let descriptor = container.get_descriptor("service1").unwrap();
        assert_eq!(descriptor.lifetime, ServiceLifetime::Transient);
    }

    #[test]
    fn test_service_container_resolve() {
        let mut container = ServiceContainer::new();
        container.register(SimpleService::new("service1", "test-value"));
        
        let resolved = container.resolve::<SimpleService>();
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().value(), "test-value");
    }

    #[test]
    fn test_service_container_resolve_not_found() {
        let container = ServiceContainer::new();
        let resolved = container.resolve::<SimpleService>();
        assert!(resolved.is_none());
    }

    #[test]
    fn test_service_descriptor() {
        let descriptor = ServiceDescriptor {
            id: "test".to_string(),
            lifetime: ServiceLifetime::Singleton,
        };
        
        assert_eq!(descriptor.id, "test");
        assert!(descriptor.lifetime.is_singleton());
    }

    #[test]
    fn test_simple_service() {
        let service = SimpleService::new("my-service", "my-value");
        assert_eq!(service.get_id(), "my-service");
        assert_eq!(service.value(), "my-value");
    }

    #[test]
    fn test_basic_service_provider() {
        let provider = BasicServiceProvider::new()
            .with_service(SimpleService::new("s1", "v1"))
            .with_service(SimpleService::new("s2", "v2"))
            .build();
        
        assert_eq!(provider.service_count(), 2);
    }

    #[test]
    fn test_resolver_resolve() {
        let mut container = ServiceContainer::new();
        container.register(SimpleService::new("service", "value"));
        
        let resolver = Resolver::new(container);
        let result: Result<&SimpleService, _> = resolver.resolve();
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), "value");
    }

    #[test]
    fn test_resolver_resolve_error() {
        let container = ServiceContainer::new();
        let resolver = Resolver::new(container);
        let result: Result<&SimpleService, _> = resolver.resolve();
        
        assert!(result.is_err());
    }

    #[test]
    fn test_resolver_try_resolve() {
        let mut container = ServiceContainer::new();
        container.register(SimpleService::new("service", "value"));
        
        let resolver = Resolver::new(container);
        let result = resolver.try_resolve::<SimpleService>();
        
        assert!(result.is_some());
    }

    #[test]
    fn test_resolver_error_display() {
        let error = ResolverError::new("Service not found");
        assert_eq!(error.to_string(), "ResolverError: Service not found");
    }
}
