//! Service Registry - Dependency Injection Container
//! 
//! Inspired by Theia's InversifyJS but implemented in Rust.
//! Provides lazy service instantiation and lifetime management.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Service factory function type
type ServiceFactory = Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// Service registry for dependency injection
pub struct ServiceRegistry {
    /// Registered service factories
    factories: RwLock<HashMap<TypeId, ServiceFactory>>,
    /// Instantiated singleton services
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service factory
    pub fn register<S, F>(&self, factory: F)
    where
        S: 'static + Send + Sync,
        F: Fn() -> S + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<S>();
        let boxed_factory: ServiceFactory = Box::new(move || {
            Arc::new(factory()) as Arc<dyn Any + Send + Sync>
        });
        
        self.factories.write().insert(type_id, boxed_factory);
    }

    /// Register an already-instantiated singleton
    pub fn register_singleton<S: 'static + Send + Sync>(&self, service: S) {
        let type_id = TypeId::of::<S>();
        self.singletons.write().insert(type_id, Arc::new(service));
    }

    /// Get a service (creates if not exists)
    pub fn get<S: 'static + Send + Sync>(&self) -> Option<Arc<S>> {
        let type_id = TypeId::of::<S>();
        
        // Check singletons first
        if let Some(service) = self.singletons.read().get(&type_id) {
            return service.clone().downcast::<S>().ok();
        }

        // Try to create from factory
        let factory = self.factories.read().get(&type_id)?.clone();
        let service = factory();
        
        // Cache as singleton
        self.singletons.write().insert(type_id, Arc::clone(&service));
        
        service.downcast::<S>().ok()
    }

    /// Check if a service is registered
    pub fn has<S: 'static>(&self) -> bool {
        let type_id = TypeId::of::<S>();
        self.factories.read().contains_key(&type_id) 
            || self.singletons.read().contains_key(&type_id)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        value: i32,
    }

    #[test]
    fn test_register_and_get() {
        let registry = ServiceRegistry::new();
        
        registry.register(|| TestService { value: 42 });
        
        let service = registry.get::<TestService>().unwrap();
        assert_eq!(service.value, 42);
    }

    #[test]
    fn test_singleton_behavior() {
        let registry = ServiceRegistry::new();
        
        registry.register(|| TestService { value: 42 });
        
        let service1 = registry.get::<TestService>().unwrap();
        let service2 = registry.get::<TestService>().unwrap();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&service1, &service2));
    }
}
