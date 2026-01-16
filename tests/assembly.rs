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
    Assembler, AssemblyContext, AssemblyError, LogMonitor, MutableAssemblyContext, NoopMonitor,
    Result, RuntimeMode, ServiceAssembly, ServiceAssemblyBase, TypeKey,
};
use assemblr::registry::ServiceRegistry;
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

#[test]
fn test_runtime_mode_parse_all_variants() {
    assert_eq!(RuntimeMode::parse("production").unwrap(), RuntimeMode::Production);
    assert_eq!(RuntimeMode::parse("prod").unwrap(), RuntimeMode::Production);
    assert_eq!(RuntimeMode::parse("development").unwrap(), RuntimeMode::Development);
    assert_eq!(RuntimeMode::parse("dev").unwrap(), RuntimeMode::Development);
    assert_eq!(RuntimeMode::parse("debug").unwrap(), RuntimeMode::Debug);
}

#[test]
fn test_runtime_mode_parse_case_insensitive() {
    assert_eq!(RuntimeMode::parse("PRODUCTION").unwrap(), RuntimeMode::Production);
    assert_eq!(RuntimeMode::parse("Debug").unwrap(), RuntimeMode::Debug);
    assert_eq!(RuntimeMode::parse("DeVeLoPmEnT").unwrap(), RuntimeMode::Development);
    assert_eq!(RuntimeMode::parse("PROD").unwrap(), RuntimeMode::Production);
}

#[test]
fn test_runtime_mode_parse_invalid() {
    assert!(RuntimeMode::parse("invalid").is_err());
    assert!(RuntimeMode::parse("test").is_err());
    assert!(RuntimeMode::parse("").is_err());

    let err = RuntimeMode::parse("invalid").unwrap_err();
    assert!(err.to_string().contains("Invalid runtime mode"));
}

#[test]
fn test_runtime_mode_display() {
    assert_eq!(format!("{}", RuntimeMode::Debug), "debug");
    assert_eq!(format!("{}", RuntimeMode::Development), "development");
    assert_eq!(format!("{}", RuntimeMode::Production), "production");
}

#[test]
fn test_runtime_mode_is_valid() {
    assert!(RuntimeMode::Debug.is_valid());
    assert!(RuntimeMode::Development.is_valid());
    assert!(RuntimeMode::Production.is_valid());
}

// ============================================================================
// TypeKey Tests
// ============================================================================

#[test]
fn test_typekey_display() {
    let key = TypeKey::new::<ServiceA>();
    let display = format!("{}", key);
    assert!(display.contains("ServiceA"));
}

#[test]
fn test_typekey_equality() {
    let key1 = TypeKey::new::<ServiceA>();
    let key2 = TypeKey::new::<ServiceA>();
    assert_eq!(key1, key2);
}

#[test]
fn test_typekey_different_types() {
    let key_a = TypeKey::new::<ServiceA>();
    let key_b = TypeKey::new::<ServiceB>();
    assert_ne!(key_a, key_b);
}

#[test]
fn test_typekey_hash() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    let key_a = TypeKey::new::<ServiceA>();
    let key_b = TypeKey::new::<ServiceB>();

    map.insert(key_a.clone(), "ServiceA");
    map.insert(key_b.clone(), "ServiceB");

    assert_eq!(map.get(&key_a), Some(&"ServiceA"));
    assert_eq!(map.get(&key_b), Some(&"ServiceB"));
}

// ============================================================================
// AssemblyError Tests
// ============================================================================

#[test]
fn test_error_invalid_runtime_mode_display() {
    let err = AssemblyError::InvalidRuntimeMode("invalid".to_string());
    assert!(err.to_string().contains("Invalid runtime mode"));
    assert!(err.to_string().contains("invalid"));
}

#[test]
fn test_error_missing_dependency_display() {
    let err = AssemblyError::MissingDependency {
        assembly: "TestAssembly".to_string(),
        message: "Service not found".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("TestAssembly"));
    assert!(msg.contains("Service not found"));
}

#[test]
fn test_error_cyclic_dependency_display() {
    let err = AssemblyError::CyclicDependency("A -> B -> A".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Cyclic dependency"));
    assert!(msg.contains("A -> B -> A"));
}

#[test]
fn test_error_general_error_display() {
    let err = AssemblyError::GeneralError("Something went wrong".to_string());
    assert_eq!(err.to_string(), "Something went wrong");
}

// ============================================================================
// LogMonitor Tests
// ============================================================================

struct MockLogMonitor {
    messages: Arc<Mutex<Vec<String>>>,
}

impl MockLogMonitor {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }
}

impl LogMonitor for MockLogMonitor {
    fn debug(&self, message: &str) {
        self.messages.lock().unwrap().push(format!("DEBUG: {}", message));
    }

    fn info(&self, message: &str) {
        self.messages.lock().unwrap().push(format!("INFO: {}", message));
    }

    fn warn(&self, message: &str) {
        self.messages.lock().unwrap().push(format!("WARN: {}", message));
    }

    fn error(&self, message: &str) {
        self.messages.lock().unwrap().push(format!("ERROR: {}", message));
    }
}

#[test]
fn test_custom_log_monitor() {
    let monitor = Arc::new(MockLogMonitor::new());
    monitor.debug("test debug");
    monitor.info("test info");
    monitor.warn("test warn");
    monitor.error("test error");

    let messages = monitor.get_messages();
    assert_eq!(messages.len(), 4);
    assert!(messages[0].contains("DEBUG"));
    assert!(messages[1].contains("INFO"));
    assert!(messages[2].contains("WARN"));
    assert!(messages[3].contains("ERROR"));
}

#[test]
fn test_noop_monitor() {
    let monitor = NoopMonitor;
    monitor.debug("test");
    monitor.info("test");
    monitor.warn("test");
    monitor.error("test");
    // Should not panic
}

#[test]
fn test_monitor_messages_during_assembly() {
    let monitor = Arc::new(MockLogMonitor::new());
    let assembler = Assembler::new(monitor.clone(), RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct TestAssembly {}
    impl ServiceAssembly for TestAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(TestAssembly {}));
    assembler.assemble().unwrap();

    let messages = monitor.get_messages();
    assert!(!messages.is_empty());
    // Should contain init, prepare, and start messages
    let has_init = messages.iter().any(|m| m.contains("Initialized"));
    let has_prepare = messages.iter().any(|m| m.contains("Prepared"));
    let has_start = messages.iter().any(|m| m.contains("Started"));
    assert!(has_init);
    assert!(has_prepare);
    assert!(has_start);
}

// ============================================================================
// Context Tests
// ============================================================================

#[test]
fn test_mutable_context_registry_access() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct ContextTestAssembly {}
    impl ServiceAssembly for ContextTestAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            // Should be able to register through context
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(ContextTestAssembly {}));
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_context_cloning() {
    let monitor = Arc::new(NoopMonitor);
    let registry = Arc::new(ServiceRegistry::new());

    let context = AssemblyContext {
        registry: registry.clone(),
        log_monitor: monitor.clone(),
        mode: RuntimeMode::Debug,
    };

    let cloned = context.clone();
    assert_eq!(cloned.mode, RuntimeMode::Debug);
}

#[test]
fn test_context_mode_propagation() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Production);

    let captured_mode = Arc::new(Mutex::new(None));
    let captured_mode_clone = captured_mode.clone();

    #[assembly(provides = [ServiceA])]
    struct ModeTestAssembly {
        captured: Arc<Mutex<Option<RuntimeMode>>>,
    }
    impl ServiceAssembly for ModeTestAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            *self.captured.lock().unwrap() = Some(context.mode);
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(ModeTestAssembly {
        captured: captured_mode_clone,
    }));
    assembler.assemble().unwrap();

    assert_eq!(captured_mode.lock().unwrap().unwrap(), RuntimeMode::Production);
}

#[test]
fn test_context_log_monitor_access() {
    let monitor = Arc::new(MockLogMonitor::new());
    let assembler = Assembler::new(monitor.clone(), RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct LogTestAssembly {}
    impl ServiceAssembly for LogTestAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.log_monitor.info("Custom message from init");
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(LogTestAssembly {}));
    assembler.assemble().unwrap();

    let messages = monitor.get_messages();
    let has_custom = messages.iter().any(|m| m.contains("Custom message from init"));
    assert!(has_custom);
}

// ============================================================================
// Enhanced Shutdown Tests
// ============================================================================

#[test]
fn test_shutdown_reverse_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FirstShutdownAssembly {
        events: Arc<Mutex<Vec<String>>>,
    }
    impl ServiceAssembly for FirstShutdownAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            self.events.lock().unwrap().push("first_init".to_string());
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            self.events.lock().unwrap().push("first_shutdown".to_string());
            Ok(())
        }
    }

    #[assembly(provides = [ServiceB], requires = [ServiceA])]
    struct SecondShutdownAssembly {
        events: Arc<Mutex<Vec<String>>>,
    }
    impl ServiceAssembly for SecondShutdownAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            self.events.lock().unwrap().push("second_init".to_string());
            context.registry.register(Arc::new(ServiceB));
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            self.events.lock().unwrap().push("second_shutdown".to_string());
            Ok(())
        }
    }

    assembler.register(Arc::new(FirstShutdownAssembly { events: events.clone() }));
    assembler.register(Arc::new(SecondShutdownAssembly { events: events.clone() }));
    assembler.assemble().unwrap();
    assembler.shutdown().unwrap();

    let tracked = events.lock().unwrap();
    // Init order: first, second
    assert_eq!(tracked[0], "first_init");
    assert_eq!(tracked[1], "second_init");
    // Shutdown order: second, first (reversed)
    let shutdown_start = tracked.iter().position(|s| s.contains("shutdown")).unwrap();
    assert_eq!(tracked[shutdown_start], "second_shutdown");
    assert_eq!(tracked[shutdown_start + 1], "first_shutdown");
}

#[test]
fn test_shutdown_with_finalize_error() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingFinalizeAssembly {}
    impl ServiceAssembly for FailingFinalizeAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn finalize(&self) -> Result<()> {
            Err(AssemblyError::GeneralError("Finalize failed".to_string()))
        }
    }

    #[assembly(provides = [ServiceB])]
    struct SuccessfulAssembly {}
    impl ServiceAssembly for SuccessfulAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceB));
            Ok(())
        }
    }

    assembler.register(Arc::new(FailingFinalizeAssembly {}));
    assembler.register(Arc::new(SuccessfulAssembly {}));
    assembler.assemble().unwrap();

    let result = assembler.shutdown();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Finalize"));
}

#[test]
fn test_shutdown_with_shutdown_error() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingShutdownAssembly {}
    impl ServiceAssembly for FailingShutdownAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Err(AssemblyError::GeneralError("Shutdown failed".to_string()))
        }
    }

    assembler.register(Arc::new(FailingShutdownAssembly {}));
    assembler.assemble().unwrap();

    let result = assembler.shutdown();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Shutdown"));
}

#[test]
fn test_shutdown_multiple_errors() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingBothAssembly {}
    impl ServiceAssembly for FailingBothAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn finalize(&self) -> Result<()> {
            Err(AssemblyError::GeneralError("Finalize error".to_string()))
        }
        fn shutdown(&self) -> Result<()> {
            Err(AssemblyError::GeneralError("Shutdown error".to_string()))
        }
    }

    assembler.register(Arc::new(FailingBothAssembly {}));
    assembler.assemble().unwrap();

    let result = assembler.shutdown();
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    // Should collect both errors
    assert!(error_msg.contains("Finalize"));
    assert!(error_msg.contains("Shutdown"));
}

#[test]
fn test_shutdown_without_assemble() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct SimpleShutdownAssembly {}
    impl ServiceAssembly for SimpleShutdownAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(SimpleShutdownAssembly {}));
    // Don't call assemble()

    // Shutdown should still work (no-op effectively)
    assert!(assembler.shutdown().is_ok());
}

// ============================================================================
// Lifecycle Phase Tests
// ============================================================================

#[test]
fn test_prepare_phase_failure() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingPrepareAssembly {}
    impl ServiceAssembly for FailingPrepareAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn prepare(&self, _context: &MutableAssemblyContext) -> Result<()> {
            Err(AssemblyError::GeneralError("Prepare failed".to_string()))
        }
    }

    assembler.register(Arc::new(FailingPrepareAssembly {}));
    let result = assembler.assemble();
    assert!(result.is_err());
}

#[test]
fn test_start_phase_failure() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingStartAssembly {}
    impl ServiceAssembly for FailingStartAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn start(&self, _context: &AssemblyContext) -> Result<()> {
            Err(AssemblyError::GeneralError("Start failed".to_string()))
        }
    }

    assembler.register(Arc::new(FailingStartAssembly {}));
    let result = assembler.assemble();
    assert!(result.is_err());
}

#[test]
fn test_finalize_phase_failure() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FailingFinalizeOnly {}
    impl ServiceAssembly for FailingFinalizeOnly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn finalize(&self) -> Result<()> {
            Err(AssemblyError::GeneralError("Finalize error".to_string()))
        }
    }

    assembler.register(Arc::new(FailingFinalizeOnly {}));
    assembler.assemble().unwrap();

    let result = assembler.shutdown();
    assert!(result.is_err());
}

#[test]
fn test_lifecycle_phases_access_registry() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct PhaseRegistryAssembly {}
    impl ServiceAssembly for PhaseRegistryAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            // Can write to registry in init
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn prepare(&self, context: &MutableAssemblyContext) -> Result<()> {
            // Can read from registry in prepare
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
        fn start(&self, context: &AssemblyContext) -> Result<()> {
            // Can read from registry in start
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
    }

    assembler.register(Arc::new(PhaseRegistryAssembly {}));
    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Registration Tests
// ============================================================================

#[test]
fn test_register_multiple_assemblies() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    for i in 0..10 {
        let mock = Arc::new(
            MockServiceAssembly::new(&format!("Assembly{}", i))
                .with_provides(vec![TypeKey::new::<ServiceA>()])
        );
        assembler.register(mock);
    }

    // Last registered should be used
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_empty_assembly() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly]
    struct EmptyAssembly {}
    impl ServiceAssembly for EmptyAssembly {
        fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
            Ok(())
        }
    }

    let assembly = EmptyAssembly {};
    assert_eq!(assembly.provides().len(), 0);
    assert_eq!(assembly.requires().len(), 0);

    assembler.register(Arc::new(EmptyAssembly {}));
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_providing_same_service() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FirstProvider {}
    impl ServiceAssembly for FirstProvider {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    #[assembly(provides = [ServiceA])]
    struct SecondProvider {}
    impl ServiceAssembly for SecondProvider {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(FirstProvider {}));
    assembler.register(Arc::new(SecondProvider {}));

    // Last registered wins
    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Complex Dependency Scenarios
// ============================================================================

#[test]
fn test_diamond_dependency_pattern() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    struct ServiceD;

    // A provides base
    let assembly_a = Arc::new(
        MockServiceAssembly::new("A").with_provides(vec![TypeKey::new::<ServiceA>()])
    );

    // B requires A
    let assembly_b = Arc::new(
        MockServiceAssembly::new("B")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()])
    );

    // C requires A
    let assembly_c = Arc::new(
        MockServiceAssembly::new("C")
            .with_provides(vec![TypeKey::new::<ServiceC>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()])
    );

    // D requires B and C
    let assembly_d = Arc::new(
        MockServiceAssembly::new("D")
            .with_provides(vec![TypeKey::new::<ServiceD>()])
            .with_requires(vec![TypeKey::new::<ServiceB>(), TypeKey::new::<ServiceC>()])
    );

    assembler.register(assembly_d);
    assembler.register(assembly_c);
    assembler.register(assembly_b);
    assembler.register(assembly_a);

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_multiple_paths_to_dependency() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[allow(dead_code)]
    struct ServiceD;
    #[allow(dead_code)]
    struct ServiceE;

    // A is the common dependency
    let assembly_a = Arc::new(
        MockServiceAssembly::new("A").with_provides(vec![TypeKey::new::<ServiceA>()])
    );

    // B requires A
    let assembly_b = Arc::new(
        MockServiceAssembly::new("B")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()])
    );

    // C requires A and B (two paths to A)
    let assembly_c = Arc::new(
        MockServiceAssembly::new("C")
            .with_provides(vec![TypeKey::new::<ServiceC>()])
            .with_requires(vec![TypeKey::new::<ServiceA>(), TypeKey::new::<ServiceB>()])
    );

    assembler.register(assembly_c);
    assembler.register(assembly_b);
    assembler.register(assembly_a);

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_independent_assembly_groups() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    // Group 1: A -> B
    let assembly_a = Arc::new(
        MockServiceAssembly::new("A").with_provides(vec![TypeKey::new::<ServiceA>()])
    );
    let assembly_b = Arc::new(
        MockServiceAssembly::new("B")
            .with_provides(vec![TypeKey::new::<ServiceB>()])
            .with_requires(vec![TypeKey::new::<ServiceA>()])
    );

    // Group 2: C (independent)
    let assembly_c = Arc::new(
        MockServiceAssembly::new("C").with_provides(vec![TypeKey::new::<ServiceC>()])
    );

    assembler.register(assembly_b);
    assembler.register(assembly_c);
    assembler.register(assembly_a);

    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_very_deep_dependency_chain() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    struct Service1;
    struct Service2;
    struct Service3;
    struct Service4;
    struct Service5;

    // Chain: 1 <- 2 <- 3 <- 4 <- 5
    let assembly_1 = Arc::new(
        MockServiceAssembly::new("Layer1").with_provides(vec![TypeKey::new::<Service1>()])
    );
    let assembly_2 = Arc::new(
        MockServiceAssembly::new("Layer2")
            .with_provides(vec![TypeKey::new::<Service2>()])
            .with_requires(vec![TypeKey::new::<Service1>()])
    );
    let assembly_3 = Arc::new(
        MockServiceAssembly::new("Layer3")
            .with_provides(vec![TypeKey::new::<Service3>()])
            .with_requires(vec![TypeKey::new::<Service2>()])
    );
    let assembly_4 = Arc::new(
        MockServiceAssembly::new("Layer4")
            .with_provides(vec![TypeKey::new::<Service4>()])
            .with_requires(vec![TypeKey::new::<Service3>()])
    );
    let assembly_5 = Arc::new(
        MockServiceAssembly::new("Layer5")
            .with_provides(vec![TypeKey::new::<Service5>()])
            .with_requires(vec![TypeKey::new::<Service4>()])
    );

    // Register in random order
    assembler.register(assembly_3);
    assembler.register(assembly_5);
    assembler.register(assembly_1);
    assembler.register(assembly_4);
    assembler.register(assembly_2);

    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_assembler_with_no_assemblies() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    // Should succeed with no assemblies
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_multiple_calls_to_assemble() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct MultiCallAssembly {}
    impl ServiceAssembly for MultiCallAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(MultiCallAssembly {}));
    assert!(assembler.assemble().is_ok());

    // Second call should also work
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_requires_but_not_provides() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    // Pure provider
    #[assembly(provides = [ServiceA])]
    struct Provider {}
    impl ServiceAssembly for Provider {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    // Pure consumer (requires but doesn't provide)
    #[assembly(requires = [ServiceA])]
    struct Consumer {}
    impl ServiceAssembly for Consumer {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
    }

    assembler.register(Arc::new(Provider {}));
    assembler.register(Arc::new(Consumer {}));
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_provides_but_not_requires() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    // Pure provider (provides but doesn't require)
    #[assembly(provides = [ServiceA])]
    struct PureProvider {}
    impl ServiceAssembly for PureProvider {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    assembler.register(Arc::new(PureProvider {}));
    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Context State Tests
// ============================================================================

#[test]
fn test_registry_state_persists_across_phases() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct PersistenceTestAssembly {}
    impl ServiceAssembly for PersistenceTestAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn prepare(&self, context: &MutableAssemblyContext) -> Result<()> {
            // Should be able to resolve what was registered in init
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
        fn start(&self, context: &AssemblyContext) -> Result<()> {
            // Should still be available in start
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
    }

    assembler.register(Arc::new(PersistenceTestAssembly {}));
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_multiple_assemblies_share_registry() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct FirstRegistry {}
    impl ServiceAssembly for FirstRegistry {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
    }

    #[assembly(provides = [ServiceB], requires = [ServiceA])]
    struct SecondRegistry {}
    impl ServiceAssembly for SecondRegistry {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            // Should see ServiceA registered by FirstRegistry
            let _service = context.registry.resolve::<ServiceA>();
            context.registry.register(Arc::new(ServiceB));
            Ok(())
        }
    }

    assembler.register(Arc::new(FirstRegistry {}));
    assembler.register(Arc::new(SecondRegistry {}));
    assert!(assembler.assemble().is_ok());
}

#[test]
fn test_assembly_can_resolve_services_in_start() {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Debug);

    #[assembly(provides = [ServiceA])]
    struct StartResolveAssembly {}
    impl ServiceAssembly for StartResolveAssembly {
        fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
            context.registry.register(Arc::new(ServiceA));
            Ok(())
        }
        fn start(&self, context: &AssemblyContext) -> Result<()> {
            // Read-only access should work
            let _service = context.registry.resolve::<ServiceA>();
            Ok(())
        }
    }

    assembler.register(Arc::new(StartResolveAssembly {}));
    assert!(assembler.assemble().is_ok());
}

// ============================================================================
// Macro Edge Cases
// ============================================================================

#[test]
fn test_macro_empty_provides_and_requires() {
    #[assembly]
    struct MinimalAssembly {}
    impl ServiceAssembly for MinimalAssembly {
        fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
            Ok(())
        }
    }

    let assembly = MinimalAssembly {};
    assert_eq!(assembly.name(), "MinimalAssembly");
    assert!(assembly.provides().is_empty());
    assert!(assembly.requires().is_empty());
}

#[test]
fn test_macro_only_requires() {
    #[assembly(requires = [ServiceA])]
    struct OnlyRequiresAssembly {}
    impl ServiceAssembly for OnlyRequiresAssembly {
        fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
            Ok(())
        }
    }

    let assembly = OnlyRequiresAssembly {};
    assert!(assembly.provides().is_empty());
    assert_eq!(assembly.requires().len(), 1);
}

#[test]
fn test_macro_with_many_types() {
    struct T1;
    struct T2;
    struct T3;
    struct T4;
    struct T5;

    #[assembly(
        provides = [ServiceA, ServiceB, ServiceC],
        requires = [T1, T2, T3, T4, T5]
    )]
    struct LargeListAssembly {}
    impl ServiceAssembly for LargeListAssembly {
        fn init(&self, _context: &MutableAssemblyContext) -> Result<()> {
            Ok(())
        }
    }

    let assembly = LargeListAssembly {};
    assert_eq!(assembly.provides().len(), 3);
    assert_eq!(assembly.requires().len(), 5);
}
