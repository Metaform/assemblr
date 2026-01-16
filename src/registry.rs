
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A service registry that maps types to their instances
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl ServiceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        ServiceRegistry {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a service
    pub fn register<T: Any + Send + Sync + 'static>(&self, service: Arc<T>) {
        let mut services = self.services.write().unwrap();
        services.insert(TypeId::of::<T>(), service as Arc<dyn Any + Send + Sync>);
    }

    /// Get a registered service
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let services = self.services.read().unwrap();
        services
            .get(&TypeId::of::<T>())
            .and_then(|service| {
                service.clone().downcast::<T>().ok()
            })
    }

    /// Check if a service is registered
    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.services.read().unwrap().contains_key(&TypeId::of::<T>())
    }

    /// Clear all registered services
    pub fn clear(&self) {
        self.services.write().unwrap().clear();
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

    trait DatabaseService: Send + Sync {
        fn query(&self, sql: &str) -> String;
    }

    struct PostgresDb;

    impl DatabaseService for PostgresDb {
        fn query(&self, sql: &str) -> String {
            format!("Executing: {}", sql)
        }
    }

    struct CacheService {
        name: String,
    }

    #[test]
    fn test_register_and_get_struct() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(CacheService {
            name: "redis".to_string(),
        }));

        let cache = registry.get::<CacheService>().unwrap();
        assert_eq!(cache.name, "redis");
    }

    struct Foo {
        ds: Arc<Box<dyn DatabaseService>>
    }

    #[test]
    fn test_foo() {
        let registry = ServiceRegistry::new();

        registry.register(Arc::new(Box::new(PostgresDb) as Box<dyn DatabaseService>));

        let db = registry.get::<Box<dyn DatabaseService>>().unwrap();
        let f = Foo { ds: db.clone() };

        assert_eq!(db.query("SELECT 1"), "Executing: SELECT 1");
    }

    #[test]
    fn test_register_and_get_trait() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(Box::new(PostgresDb) as Box<dyn DatabaseService>));

        let db = registry.get::<Box<dyn DatabaseService>>().unwrap();
        assert_eq!(db.query("SELECT 1"), "Executing: SELECT 1");
    }

    #[test]
    fn test_get_nonexistent_service() {
        let registry = ServiceRegistry::new();
        let result = registry.get::<CacheService>();
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_services() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(CacheService {
            name: "redis".to_string(),
        }));

        registry.register(Arc::new(Box::new(PostgresDb) as Box<dyn DatabaseService>));

        assert!(registry.contains::<CacheService>());
        assert!(registry.contains::<Box<dyn DatabaseService>>());
        assert!(!registry.contains::<String>());
    }

    #[test]
    fn test_clear() {
        let registry = ServiceRegistry::new();
        registry.register(Arc::new(CacheService {
            name: "redis".to_string(),
        }));

        assert!(registry.contains::<CacheService>());
        registry.clear();
        assert!(!registry.contains::<CacheService>());
    }
}

fn main() {
    let registry = ServiceRegistry::new();

    registry.register(Arc::new(CacheService {
        name: "redis".to_string(),
    }));

    registry.register(Arc::new(Box::new(PostgresDb) as Box<dyn DatabaseService>));

    if let Some(cache) = registry.get::<CacheService>() {
        println!("Got cache service: {}", cache.name);
    }

    if let Some(db) = registry.get::<Box<dyn DatabaseService>>() {
        println!("{}", db.query("SELECT * FROM users"));
    }
}

trait DatabaseService: Send + Sync {
    fn query(&self, sql: &str) -> String;
}

struct PostgresDb;

impl DatabaseService for PostgresDb {
    fn query(&self, sql: &str) -> String {
        format!("Executing: {}", sql)
    }
}

struct CacheService {
    name: String,
}