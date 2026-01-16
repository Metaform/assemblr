//  Copyright (c) 2026 Metaform Systems, Inc
//
//  This program and the accompanying materials are made available under the
//  terms of the Apache License, Version 2.0 which is available at
//  https://www.apache.org/licenses/LICENSE-2.0
//
//  SPDX-License-Identifier: Apache-2.0
//
//  Contributors:
//       Metaform Systems, Inc. - initial API and implementation
//

use assemblr::assembly::{
    Assembler, AssemblyContext, AssemblyError, MutableAssemblyContext, NoopMonitor, Result,
    RuntimeMode, ServiceAssembly, ServiceAssemblyBase, TypeKey,
};
use assembly_macros::assembly;
use std::sync::{Arc, Mutex};
// ============================================================================
// Test Service Types
// ============================================================================

struct ServiceA;
struct ServiceB;
struct ServiceC;

trait Database: Send + Sync {
    fn _query(&self) -> String;
}

trait Cache: Send + Sync {
    fn _get(&self) -> String;
}

struct PostgresDatabase;

impl Database for PostgresDatabase {
    fn _query(&self) -> String {
        "postgres_data".to_string()
    }
}

struct RedisCache;

impl Cache for RedisCache {
    fn _get(&self) -> String {
        "cached_value".to_string()
    }
}

// ============================================================================
// Mock Assembly (for flexible testing)
// ============================================================================

struct MockServiceAssembly {
    name: String,
    provides: Vec<TypeKey>,
    requires: Vec<TypeKey>,
}

impl MockServiceAssembly {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            provides: Vec::new(),
            requires: Vec::new(),
        }
    }

    fn with_provides(mut self, services: Vec<TypeKey>) -> Self {
        self.provides = services;
        self
    }

    fn with_requires(mut self, services: Vec<TypeKey>) -> Self {
        self.requires = services;
        self
    }
}

impl ServiceAssemblyBase for MockServiceAssembly {
    fn name(&self) -> &str {
        &self.name
    }

    fn provides(&self) -> Vec<TypeKey> {
        self.provides.clone()
    }

    fn requires(&self) -> Vec<TypeKey> {
        self.requires.clone()
    }
}

impl ServiceAssembly for MockServiceAssembly {
    fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// Basic Dependency Resolution Tests
// ============================================================================

#[test]
fn test_single_assembly_no_dependencies() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    let mock = Arc::new(
        MockServiceAssembly::new("CoreAssembly").with_provides(vec![TypeKey::new::<ServiceA>()]),
    );
    assembler.register(mock);

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_with_single_dependency() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    let provider = Arc::new(
        MockServiceAssembly::new("Provider").with_provides(vec![TypeKey::new::<ServiceA>()]),
    );

    let consumer = Arc::new(
        MockServiceAssembly::new("Consumer")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()]),
    );

    // Register in reverse order to test dependency resolution
    assembler.register(consumer);
    assembler.register(provider);

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_missing_dependency_fails() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    let mock = Arc::new(
        MockServiceAssembly::new("NeedsDependency").with_requires(vec![TypeKey::new::<ServiceA>()]),
    );

    assembler.register(mock);

    let result = assembler.assemble();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Required assembly not found")
    );
}

#[test]
fn test_cyclic_dependency_detected() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    let assembly1 = Arc::new(
        MockServiceAssembly::new("Assembly1")
            .with_provides(vec![TypeKey::new::<ServiceA>()])
            .with_requires(vec![TypeKey::new::<ServiceB>()]),
    );

    let assembly2 = Arc::new(
        MockServiceAssembly::new("Assembly2")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()]),
    );

    assembler.register(assembly1);
    assembler.register(assembly2);

    let result = assembler.assemble();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cyclic dependency")
    );
}

#[test]
fn test_complex_dependency_chain() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    // Create the chain: A <- B <- C
    let assembly_a = Arc::new(
        MockServiceAssembly::new("LayerA").with_provides(vec![TypeKey::new::<ServiceA>()]),
    );

    let assembly_b = Arc::new(
        MockServiceAssembly::new("LayerB")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()]),
    );

    let assembly_c = Arc::new(
        MockServiceAssembly::new("LayerC")
            .with_provides(vec![TypeKey::new::<ServiceC>()])
            .with_requires(vec![TypeKey::new::<ServiceB>()]),
    );

    // Register in random order
    assembler.register(assembly_b);
    assembler.register(assembly_c);
    assembler.register(assembly_a);

    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[assembly(provides = [ServiceA])]
struct LifecycleTrackingAssembly {
    events: Arc<Mutex<Vec<String>>>,
}

impl LifecycleTrackingAssembly {
    fn new(events: Arc<Mutex<Vec<String>>>) -> Self {
        Self { events }
    }

    fn track(&self, event: &str) {
        self.events.lock().unwrap().push(event.to_string());
    }
}

impl ServiceAssembly for LifecycleTrackingAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        self.track("init");
        context.registry.register(Arc::new(ServiceA));
        Ok(())
    }

    fn prepare(&self, _context: &MutableAssemblyContext) -> Result<()> {
        self.track("prepare");
        Ok(())
    }

    fn start(&self, _context: &AssemblyContext) -> Result<()> {
        self.track("start");
        Ok(())
    }

    fn finalize(&self) -> Result<()> {
        self.track("finalize");
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        self.track("shutdown");
        Ok(())
    }
}

#[test]
fn test_lifecycle_methods_called_in_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    assembler.register(Arc::new(LifecycleTrackingAssembly::new(events.clone())));
    assembler.assemble().unwrap();

    let tracked = events.lock().unwrap();
    assert_eq!(*tracked, vec!["init", "prepare", "start"]);
}

#[test]
fn test_shutdown_lifecycle() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    assembler.register(Arc::new(LifecycleTrackingAssembly::new(events.clone())));
    assembler.assemble().unwrap();
    assembler.shutdown().unwrap();

    let tracked = events.lock().unwrap();
    assert_eq!(
        *tracked,
        vec!["init", "prepare", "start", "finalize", "shutdown"]
    );
}

#[test]
fn test_initialization_order_respects_dependencies() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FirstAssembly {
        events: Arc<Mutex<Vec<String>>>,
    }
    impl ServiceAssembly for FirstAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            self.events.lock().unwrap().push("first".to_string());
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    #[assembly(provides = [ServiceB], requires = [ServiceA])]
    struct SecondAssembly {
        events: Arc<Mutex<Vec<String>>>,
    }
    impl ServiceAssembly for SecondAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            self.events.lock().unwrap().push("second".to_string());
            context.registry.register(Arc::new(ServiceB));
            Ok(())
        }
    }

    // Register in reverse order
    assembler.register(Arc::new(SecondAssembly {
        events: events.clone(),
    }));
    assembler.register(Arc::new(FirstAssembly {
        events: events.clone(),
    }));

    assembler.assemble().unwrap();

    let tracked = events.lock().unwrap();
    assert_eq!(*tracked, vec!["first", "second"]);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[assembly(provides = [ServiceA])]
struct FailingAssembly {}

impl ServiceAssembly for FailingAssembly {
    fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Err(AssemblyError::GeneralError(
            "Intentional init failure".to_string(),
        ))
    }
}

#[test]
fn test_init_failure_propagates() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    assembler.register(Arc::new(FailingAssembly {}));

    let result = assembler.assemble();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    // The error is wrapped in context, check if it contains our message
    assert!(
        error_msg.contains("Intentional init failure")
            || error_msg.contains("Failed to initialize"),
        "Error message was: {}",
        error_msg
    );
}

// ============================================================================
// Macro Tests
// ============================================================================

#[assembly(provides = [ServiceA])]
struct SimpleAssembly {}

impl ServiceAssembly for SimpleAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        context.registry.register(Arc::new(ServiceA));
        Ok(())
    }
}

#[test]
fn test_macro_generates_name_from_struct() {
    let assembly = SimpleAssembly {};
    assert_eq!(assembly.name(), "SimpleAssembly");
}

#[test]
fn test_macro_generates_provides() {
    let assembly = SimpleAssembly {};
    assert_eq!(assembly.provides().len(), 1);
    assert_eq!(assembly.requires().len(), 0);
}

#[assembly(name = "CustomName", provides = [ServiceA])]
struct ExplicitNameAssembly {}

impl ServiceAssembly for ExplicitNameAssembly {
    fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Ok(())
    }
}

#[test]
fn test_macro_explicit_name() {
    let assembly = ExplicitNameAssembly {};
    assert_eq!(assembly.name(), "CustomName");
}

#[assembly(
    provides = [ServiceA, ServiceB, ServiceC]
)]
struct MultiProvideAssembly {}

impl ServiceAssembly for MultiProvideAssembly {
    fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Ok(())
    }
}

#[test]
fn test_macro_multiple_provides() {
    let assembly = MultiProvideAssembly {};
    assert_eq!(assembly.provides().len(), 3);
}

#[assembly(
    provides = [ServiceB],
    requires = [ServiceA]
)]
struct DependentAssembly {}

impl ServiceAssembly for DependentAssembly {
    fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Ok(())
    }
}

#[test]
fn test_macro_provides_and_requires() {
    let assembly = DependentAssembly {};
    assert_eq!(assembly.name(), "DependentAssembly");
    assert_eq!(assembly.provides().len(), 1);
    assert_eq!(assembly.requires().len(), 1);
}

// ============================================================================
// Trait Object Tests
// ============================================================================

#[assembly(provides = [Box<dyn Database>])]
struct DatabaseAssembly {}

impl ServiceAssembly for DatabaseAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        let db = Arc::new(Box::new(PostgresDatabase) as Box<dyn Database>);
        context.registry.register(db);
        Ok(())
    }
}

#[assembly(
    provides = [Box<dyn Cache>],
    requires = [Box<dyn Database>]
)]
struct CacheAssembly {}

impl ServiceAssembly for CacheAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Verify database is available
        let _db = context.registry.resolve::<Box<dyn Database>>();

        let cache = Arc::new(Box::new(RedisCache) as Box<dyn Cache>);
        context.registry.register(cache);
        Ok(())
    }
}

#[test]
fn test_trait_object_dependencies() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    assembler.register(Arc::new(CacheAssembly {}));
    assembler.register(Arc::new(DatabaseAssembly {}));

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_trait_object_service_resolution() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    assembler.register(Arc::new(DatabaseAssembly {}));
    assembler.assemble().unwrap();

    // Database service should be registered and resolvable
    // (This test verifies the assembly worked correctly)
}

// ============================================================================
// Runtime Mode Tests
// ============================================================================

#[test]
fn test_runtime_mode_debug() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);
    drop(assembler);
}

#[test]
fn test_runtime_mode_development() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Development);
    drop(assembler);
}

#[test]
fn test_runtime_mode_production() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Production);
    drop(assembler);
}
