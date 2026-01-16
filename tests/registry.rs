// Copyright (c) 2025 Metaform Systems, Inc
//
// This program and the accompanying materials are made available under the
// terms of the Apache License, Version 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
// SPDX-License-Identifier: Apache-2.0
//
// Contributors:
//      Metaform Systems, Inc. - initial API and implementation

use assemblr::registry::{RegistryWriteHandle, ServiceRegistry};
use assemblr::{register, register_trait, resolve_trait};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Test trait and implementations
trait DatabaseService: Send + Sync {
    fn query(&self, sql: &str) -> String;
}

struct PostgresDb;

impl DatabaseService for PostgresDb {
    fn query(&self, sql: &str) -> String {
        format!("Executing: {}", sql)
    }
}

struct MySqlDb;

impl DatabaseService for MySqlDb {
    fn query(&self, sql: &str) -> String {
        format!("MySQL: {}", sql)
    }
}

// Test structs
struct CacheService {
    name: String,
}

#[allow(dead_code)]
struct ConfigService {
    port: u16,
    host: String,
}

struct Counter {
    value: Mutex<i32>,
}

impl Counter {
    fn new() -> Self {
        Counter {
            value: Mutex::new(0),
        }
    }

    fn increment(&self) -> i32 {
        let mut val = self.value.lock().unwrap();
        *val += 1;
        *val
    }

    fn get(&self) -> i32 {
        *self.value.lock().unwrap()
    }
}

// Generic struct
struct Container<T> {
    data: T,
}

// Complex trait
trait ComplexService: Send + Sync {
    fn method_one(&self) -> String;
    fn method_two(&self, input: i32) -> i32;
    fn method_three(&self) -> Vec<String>;
}

struct ComplexServiceImpl {
    prefix: String,
}

impl ComplexService for ComplexServiceImpl {
    fn method_one(&self) -> String {
        format!("{}_one", self.prefix)
    }

    fn method_two(&self, input: i32) -> i32 {
        input * 2
    }

    fn method_three(&self) -> Vec<String> {
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    }
}

// ============================================================================
// Basic Original Tests
// ============================================================================

#[test]
fn test_register_and_get_struct() {
    let registry = ServiceRegistry::new();
    let cache_service = CacheService {
        name: "redis".to_string(),
    };

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, cache_service);
    }

    let cache = registry.resolve::<CacheService>();
    assert_eq!(cache.name, "redis");
}

#[test]
fn test_register_and_get_trait() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register_trait!(&handle, dyn DatabaseService, PostgresDb);
    }

    let db = registry.resolve::<Box<dyn DatabaseService>>();
    assert_eq!(db.query("SELECT 1"), "Executing: SELECT 1");
}

#[test]
#[should_panic(expected = "Service 'registry::CacheService' not found in registry")]
fn test_get_nonexistent_service() {
    let registry = ServiceRegistry::new();
    registry.resolve::<CacheService>();
}

#[test]
fn test_multiple_services() {
    let registry = ServiceRegistry::new();
    let cache_service = CacheService {
        name: "redis".to_string(),
    };

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, cache_service);
        register_trait!(&handle, dyn DatabaseService, PostgresDb);
        // Test resolve_trait with both ServiceRegistry and RegistryWriteHandle
        resolve_trait!(&registry, dyn DatabaseService);
        resolve_trait!(&handle, dyn DatabaseService);
    }

    assert!(registry.contains::<CacheService>());
    assert!(registry.contains::<Box<dyn DatabaseService>>());
    assert!(!registry.contains::<String>());
}

// ============================================================================
// Empty Registry Tests
// ============================================================================

#[test]
fn test_empty_registry() {
    let registry = ServiceRegistry::new();
    assert!(!registry.contains::<CacheService>());
    assert!(!registry.contains::<String>());
}

#[test]
fn test_contains_empty_registry() {
    let registry = ServiceRegistry::new();
    assert!(!registry.contains::<CacheService>());
    assert!(!registry.contains::<Box<dyn DatabaseService>>());
    assert!(!registry.contains::<i32>());
}

// ============================================================================
// Registration & Replacement
// ============================================================================

#[test]
fn test_service_replacement() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
    }

    let first = registry.resolve::<CacheService>();
    assert_eq!(first.name, "redis");

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "memcached".to_string() });
    }

    let second = registry.resolve::<CacheService>();
    assert_eq!(second.name, "memcached");
}

#[test]
fn test_register_different_values_same_type() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "first".to_string() });
        register!(&handle, CacheService { name: "second".to_string() });
        register!(&handle, CacheService { name: "third".to_string() });
    }

    let service = registry.resolve::<CacheService>();
    assert_eq!(service.name, "third");
}

#[test]
fn test_multiple_different_types() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
        register!(&handle, ConfigService { port: 8080, host: "localhost".to_string() });
        register!(&handle, Counter::new());
    }

    assert!(registry.contains::<CacheService>());
    assert!(registry.contains::<ConfigService>());
    assert!(registry.contains::<Counter>());

    let cache = registry.resolve::<CacheService>();
    let config = registry.resolve::<ConfigService>();
    let counter = registry.resolve::<Counter>();

    assert_eq!(cache.name, "redis");
    assert_eq!(config.port, 8080);
    assert_eq!(counter.get(), 0);
}

// ============================================================================
// RegistryWriteHandle Behavior
// ============================================================================

#[test]
fn test_handle_shares_storage() {
    let registry = ServiceRegistry::new();
    let handle = RegistryWriteHandle::new(&registry);

    register!(&handle, CacheService { name: "redis".to_string() });

    // Should be visible in both handle and registry
    let from_registry = registry.resolve::<CacheService>();
    let from_handle = handle.resolve::<CacheService>();

    assert_eq!(from_registry.name, "redis");
    assert_eq!(from_handle.name, "redis");
}

#[test]
fn test_multiple_handles() {
    let registry = ServiceRegistry::new();
    let handle1 = RegistryWriteHandle::new(&registry);
    let handle2 = RegistryWriteHandle::new(&registry);

    register!(&handle1, CacheService { name: "redis".to_string() });
    register!(&handle2, ConfigService { port: 8080, host: "localhost".to_string() });

    // Both services should be accessible from both handles
    assert!(registry.contains::<CacheService>());
    assert!(registry.contains::<ConfigService>());

    let cache_from_h1 = handle1.resolve::<CacheService>();
    let cache_from_h2 = handle2.resolve::<CacheService>();
    assert_eq!(cache_from_h1.name, cache_from_h2.name);
}

#[test]
fn test_handle_resolve_vs_registry_resolve() {
    let registry = ServiceRegistry::new();
    let handle = RegistryWriteHandle::new(&registry);

    register!(&handle, Counter::new());

    let from_registry = registry.resolve::<Counter>();
    let from_handle = handle.resolve::<Counter>();

    // Both should point to the same Arc instance
    from_registry.increment();
    assert_eq!(from_handle.get(), 1);

    from_handle.increment();
    assert_eq!(from_registry.get(), 2);
}

#[test]
fn test_handle_register_visible_in_registry() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "memcached".to_string() });
    }

    assert!(registry.contains::<CacheService>());
    let service = registry.resolve::<CacheService>();
    assert_eq!(service.name, "memcached");
}

// ============================================================================
// Arc Reference Sharing
// ============================================================================

#[test]
fn test_resolve_returns_same_arc() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, Counter::new());
    }

    let counter1 = registry.resolve::<Counter>();
    let counter2 = registry.resolve::<Counter>();

    counter1.increment();
    counter1.increment();

    // counter2 should see the same shared state
    assert_eq!(counter2.get(), 2);

    counter2.increment();
    assert_eq!(counter1.get(), 3);
}

#[test]
fn test_arc_strong_count() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
    }

    let ref1 = registry.resolve::<CacheService>();
    let ref2 = registry.resolve::<CacheService>();
    let ref3 = registry.resolve::<CacheService>();

    // All three should point to the same Arc
    assert_eq!(ref1.name, ref2.name);
    assert_eq!(ref2.name, ref3.name);

    // Verify they're actually the same Arc by checking pointer equality
    assert!(Arc::ptr_eq(&ref1, &ref2));
    assert!(Arc::ptr_eq(&ref2, &ref3));
}

// ============================================================================
// Complex Trait Scenarios
// ============================================================================

#[test]
fn test_multiple_trait_implementations() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        // Note: We can only register one implementation per trait type
        // This tests that we can register a trait service
        register_trait!(&handle, dyn DatabaseService, PostgresDb);
    }

    let db = resolve_trait!(&registry, dyn DatabaseService);
    assert_eq!(db.query("SELECT *"), "Executing: SELECT *");

    // Replace with different implementation
    {
        let handle = RegistryWriteHandle::new(&registry);
        register_trait!(&handle, dyn DatabaseService, MySqlDb);
    }

    let db2 = resolve_trait!(&registry, dyn DatabaseService);
    assert_eq!(db2.query("SELECT *"), "MySQL: SELECT *");
}

#[test]
fn test_trait_with_mutable_state() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, Counter::new());
    }

    let counter1 = registry.resolve::<Counter>();
    let counter2 = registry.resolve::<Counter>();

    assert_eq!(counter1.increment(), 1);
    assert_eq!(counter2.increment(), 2);
    assert_eq!(counter1.get(), 2);
}

#[test]
fn test_trait_with_multiple_methods() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register_trait!(&handle, dyn ComplexService, ComplexServiceImpl {
            prefix: "test".to_string()
        });
    }

    let service = resolve_trait!(&registry, dyn ComplexService);
    assert_eq!(service.method_one(), "test_one");
    assert_eq!(service.method_two(5), 10);
    assert_eq!(service.method_three(), vec!["a", "b", "c"]);
}

// ============================================================================
// Different Data Types
// ============================================================================

#[test]
fn test_register_primitive_wrapper() {
    struct IntWrapper(i32);
    struct BoolWrapper(bool);

    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, IntWrapper(42));
        register!(&handle, BoolWrapper(true));
    }

    let int_val = registry.resolve::<IntWrapper>();
    let bool_val = registry.resolve::<BoolWrapper>();

    assert_eq!(int_val.0, 42);
    assert_eq!(bool_val.0, true);
}

#[test]
fn test_register_string_and_vec() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, "hello world".to_string());
        register!(&handle, vec![1, 2, 3, 4, 5]);
    }

    let string_val = registry.resolve::<String>();
    let vec_val = registry.resolve::<Vec<i32>>();

    assert_eq!(*string_val, "hello world");
    assert_eq!(*vec_val, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_register_complex_nested_types() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        let mut map = HashMap::new();
        map.insert("key1".to_string(), vec![1, 2, 3]);
        map.insert("key2".to_string(), vec![4, 5, 6]);
        register!(&handle, map);

        register!(&handle, ConfigService {
            port: 9000,
            host: "0.0.0.0".to_string()
        });
    }

    let map_val = registry.resolve::<HashMap<String, Vec<i32>>>();
    assert_eq!(map_val.get("key1"), Some(&vec![1, 2, 3]));

    let config = registry.resolve::<ConfigService>();
    assert_eq!(config.port, 9000);
}

#[test]
fn test_register_generic_types() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, Container { data: 42 });
        register!(&handle, Container { data: "text".to_string() });
        register!(&handle, Container { data: vec![1, 2, 3] });
    }

    let int_container = registry.resolve::<Container<i32>>();
    let string_container = registry.resolve::<Container<String>>();
    let vec_container = registry.resolve::<Container<Vec<i32>>>();

    assert_eq!(int_container.data, 42);
    assert_eq!(string_container.data, "text");
    assert_eq!(vec_container.data, vec![1, 2, 3]);
}

// ============================================================================
// Macro Edge Cases
// ============================================================================

#[test]
fn test_register_macro_with_complex_expression() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);

        // Register with computed values
        let name = format!("{}-{}", "redis", "cache");
        register!(&handle, CacheService { name });

        // Register with constructor
        register!(&handle, Counter::new());
    }

    let cache = registry.resolve::<CacheService>();
    assert_eq!(cache.name, "redis-cache");

    let counter = registry.resolve::<Counter>();
    assert_eq!(counter.get(), 0);
}

#[test]
fn test_resolve_trait_macro_variations() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register_trait!(&handle, dyn DatabaseService, PostgresDb);
    }

    // Via registry
    let db1 = resolve_trait!(&registry, dyn DatabaseService);
    assert_eq!(db1.query("test"), "Executing: test");

    // Via handle
    let handle = RegistryWriteHandle::new(&registry);
    let db2 = resolve_trait!(&handle, dyn DatabaseService);
    assert_eq!(db2.query("test"), "Executing: test");

    // Direct resolve without macro
    let db3 = registry.resolve::<Box<dyn DatabaseService>>();
    assert_eq!(db3.query("test"), "Executing: test");
}

// ============================================================================
// Contains Method Coverage
// ============================================================================

#[test]
fn test_contains_after_registration() {
    let registry = ServiceRegistry::new();

    assert!(!registry.contains::<CacheService>());

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
    }

    assert!(registry.contains::<CacheService>());
}

#[test]
fn test_contains_different_types() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
        register!(&handle, ConfigService { port: 8080, host: "localhost".to_string() });
    }

    assert!(registry.contains::<CacheService>());
    assert!(registry.contains::<ConfigService>());
    assert!(!registry.contains::<Counter>());
    assert!(!registry.contains::<String>());
}

#[test]
fn test_contains_before_and_after() {
    let registry = ServiceRegistry::new();

    assert!(!registry.contains::<CacheService>());
    assert!(!registry.contains::<ConfigService>());

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "redis".to_string() });
    }

    assert!(registry.contains::<CacheService>());
    assert!(!registry.contains::<ConfigService>());

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, ConfigService { port: 3000, host: "localhost".to_string() });
    }

    assert!(registry.contains::<CacheService>());
    assert!(registry.contains::<ConfigService>());
}

// ============================================================================
// Default Trait
// ============================================================================

#[test]
fn test_default_trait() {
    let registry: ServiceRegistry = Default::default();
    assert!(!registry.contains::<CacheService>());

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, CacheService { name: "test".to_string() });
    }

    assert!(registry.contains::<CacheService>());
}

// ============================================================================
// Panic Scenarios
// ============================================================================

#[test]
#[should_panic(expected = "not found in registry")]
fn test_handle_resolve_panics_on_missing() {
    let registry = ServiceRegistry::new();
    let handle = RegistryWriteHandle::new(&registry);

    handle.resolve::<CacheService>();
}

#[test]
#[should_panic(expected = "Service")]
fn test_panic_message_includes_type_name() {
    let registry = ServiceRegistry::new();
    registry.resolve::<ConfigService>();
}

// ============================================================================
// Concurrent/Shared Access
// ============================================================================

#[test]
fn test_shared_service_modification() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, Counter::new());
    }

    let counter1 = registry.resolve::<Counter>();
    let counter2 = registry.resolve::<Counter>();
    let counter3 = registry.resolve::<Counter>();

    counter1.increment();
    counter2.increment();
    counter3.increment();

    assert_eq!(counter1.get(), 3);
    assert_eq!(counter2.get(), 3);
    assert_eq!(counter3.get(), 3);
}

#[test]
fn test_service_independence() {
    let registry = ServiceRegistry::new();

    {
        let handle = RegistryWriteHandle::new(&registry);
        register!(&handle, Counter::new());
        register!(&handle, CacheService { name: "redis".to_string() });
    }

    let counter = registry.resolve::<Counter>();
    counter.increment();

    // Resolving Counter shouldn't affect CacheService
    let cache = registry.resolve::<CacheService>();
    assert_eq!(cache.name, "redis");

    // Counter should still have its state
    assert_eq!(counter.get(), 1);
}
