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

#![allow(dead_code)]

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::dag::Graph;
use crate::registry::{RegistryWriteHandle, ServiceRegistry};

#[derive(Error, Debug)]
pub enum AssemblyError {
    #[error("Invalid runtime mode: {0}")]
    InvalidRuntimeMode(String),

    #[error("Assembly '{assembly}' error: {message}")]
    MissingDependency { assembly: String, message: String },

    #[error("Cyclic dependency detected in assembly graph ({0})")]
    CyclicDependency(String),

    #[error("{0}")]
    GeneralError(String),
}

pub type Result<T> = std::result::Result<T, AssemblyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Debug,
    Development,
    Production,
}

impl RuntimeMode {
    pub fn is_valid(&self) -> bool {
        true
    }

    pub fn parse(mode: &str) -> Result<Self> {
        match mode.to_lowercase().as_str() {
            "production" | "prod" => Ok(RuntimeMode::Production),
            "development" | "dev" => Ok(RuntimeMode::Development),
            "debug" => Ok(RuntimeMode::Debug),
            _ => Err(AssemblyError::InvalidRuntimeMode(mode.to_string())),
        }
    }
}

impl fmt::Display for RuntimeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeMode::Debug => write!(f, "debug"),
            RuntimeMode::Development => write!(f, "development"),
            RuntimeMode::Production => write!(f, "production"),
        }
    }
}

pub trait LogMonitor: Send + Sync {
    fn debug(&self, message: &str);
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
}

pub struct NoopMonitor;

impl LogMonitor for NoopMonitor {
    fn debug(&self, _: &str) {}
    fn info(&self, _: &str) {}
    fn warn(&self, _: &str) {}
    fn error(&self, _: &str) {}
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct TypeKey(TypeId, String);

impl TypeKey {
    pub fn new<T: 'static>() -> Self {
        TypeKey(TypeId::of::<T>(), String::from(std::any::type_name::<T>()))
    }
}

impl fmt::Display for TypeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.1)
    }
}

/// Context provided during the init() and prepare() phases with write access to the registry
pub struct MutableAssemblyContext {
    pub registry: RegistryWriteHandle,
    pub log_monitor: Arc<dyn LogMonitor>,
    pub mode: RuntimeMode,
}

/// Context provided during the start() phase with read-only registry access
#[derive(Clone)]
pub struct AssemblyContext {
    pub registry: Arc<ServiceRegistry>,
    pub log_monitor: Arc<dyn LogMonitor>,
    pub mode: RuntimeMode,
}

/// Base trait for service assembly metadata
pub trait ServiceAssemblyBase: Send + Sync {
    fn name(&self) -> &str;

    fn provides(&self) -> Vec<TypeKey> {
        Vec::new()
    }

    fn requires(&self) -> Vec<TypeKey> {
        Vec::new()
    }
}

/// A subsystem that contributes services to a runtime
pub trait ServiceAssembly: ServiceAssemblyBase {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()>;

    fn prepare(&self, _context: &MutableAssemblyContext) -> Result<()> {
        Ok(())
    }

    fn start(&self, _context: &AssemblyContext) -> Result<()> {
        Ok(())
    }

    fn finalize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub struct Assembler {
    assemblies: RwLock<Vec<Arc<dyn ServiceAssembly>>>,
    registry: Arc<ServiceRegistry>,
    log_monitor: Arc<dyn LogMonitor>,
    mode: RuntimeMode,
}

impl Assembler {
    pub fn new(log_monitor: Arc<dyn LogMonitor>, mode: RuntimeMode) -> Self {
        Assembler {
            assemblies: RwLock::new(Vec::new()),
            registry: Arc::new(ServiceRegistry::new()),
            log_monitor,
            mode,
        }
    }

    /// Registers a service assembly
    pub fn register(&self, assembly: Arc<dyn ServiceAssembly>) {
        self.assemblies.write().unwrap().push(assembly);
    }

    /// Initializes and prepares registered assemblies in dependency order
    pub fn assemble(&self) -> Result<()> {
        // Acquire write lock once at the start
        let mut assemblies = self.assemblies.write().unwrap();

        // Build dependency graph
        let mut assembly_graph: Graph<String> = Graph::new();
        let mut mapped_assemblies: HashMap<TypeKey, String> = HashMap::new();

        // Add vertices for each assembly
        for assembly in assemblies.iter() {
            let name = assembly.name().to_string();
            assembly_graph.add_vertex(name.clone(), name.clone());

            for provided in assembly.provides() {
                mapped_assemblies.insert(provided, name.clone());
            }
        }

        // Add edges for dependencies
        for assembly in assemblies.iter() {
            let assembly_name = assembly.name().to_string();
            for required in assembly.requires() {
                if let Some(required_assembly) = mapped_assemblies.get(&required) {
                    assembly_graph.add_edge(&assembly_name, required_assembly);
                } else {
                    let error_msg =
                        format!("Required assembly not found for service: {}", required);
                    self.log_monitor.error(&format!(
                        "Failed to resolve dependency in {}: {}",
                        assembly_name, error_msg
                    ));
                    return Err(AssemblyError::MissingDependency {
                        assembly: assembly_name,
                        message: error_msg,
                    });
                }
            }
        }

        // Perform topological sort
        let sort_result = assembly_graph.topological_sort();
        if sort_result.has_cycle {
            let cycle_info = if sort_result.cycle_path.is_empty() {
                "unknown cycle".to_string()
            } else {
                format!("Cycle path: {:?}", sort_result.cycle_path)
            };
            let error_msg = format!(
                "Cyclic dependency detected in assembly graph ({})",
                cycle_info
            );
            self.log_monitor.error(&error_msg);
            return Err(AssemblyError::CyclicDependency(cycle_info));
        }

        // Reverse the sorted order (dependencies first)
        let ordered_assemblies = sort_result
            .sorted_order
            .iter()
            .rev()
            .filter_map(|name| assemblies.iter().find(|a| a.name() == name).cloned())
            .collect::<Vec<_>>();

        // Create read-only context for the start phase
        let context = AssemblyContext {
            registry: self.registry.clone(),
            log_monitor: self.log_monitor.clone(),
            mode: self.mode,
        };

        // Create mutable context for the init phase
        let registry_handle = RegistryWriteHandle::new(&self.registry);
        let init_context = MutableAssemblyContext {
            registry: registry_handle,
            log_monitor: self.log_monitor.clone(),
            mode: self.mode,
        };

        // Initialize assemblies with mutable context
        for assembly in &ordered_assemblies {
            assembly.init(&init_context)?;
            self.log_monitor
                .debug(&format!("Initialized: {}", assembly.name()));
        }

        // Create mutable context for prepare phase
        let prepare_registry_handle = RegistryWriteHandle::new(&self.registry);
        let prepare_context = MutableAssemblyContext {
            registry: prepare_registry_handle,
            log_monitor: self.log_monitor.clone(),
            mode: self.mode,
        };

        // Prepare assemblies with mutable context
        for assembly in &ordered_assemblies {
            assembly.prepare(&prepare_context)?;
            self.log_monitor
                .debug(&format!("Prepared: {}", assembly.name()));
        }

        // Start assemblies with read-only context
        for assembly in &ordered_assemblies {
            assembly.start(&context)?;
            self.log_monitor
                .debug(&format!("Started: {}", assembly.name()));
        }

        // Replace assemblies vec with ordered version
        *assemblies = ordered_assemblies;

        Ok(())
    }

    /// Finalizes and shuts down assemblies in reverse order
    /// Attempts to gracefully degrade on errors, collecting all failures
    pub fn shutdown(&self) -> Result<()> {
        let mut errors: Vec<String> = Vec::new();

        // Acquire read lock for iteration
        let assemblies = self.assemblies.read().unwrap();

        // Finalize assemblies
        for assembly in assemblies.iter().rev() {
            match assembly.finalize() {
                Ok(_) => {
                    self.log_monitor
                        .debug(&format!("Finalized: {}", assembly.name()));
                }
                Err(e) => {
                    let error_msg = format!("Finalize: '{}': {}", assembly.name(), e);
                    errors.push(error_msg);
                }
            }
        }

        // Shutdown assemblies
        for assembly in assemblies.iter().rev() {
            match assembly.shutdown() {
                Ok(_) => {
                    self.log_monitor
                        .debug(&format!("Shutdown: {}", assembly.name()));
                }
                Err(e) => {
                    let error_msg = format!("Shutdown: {}: {}", assembly.name(), e);
                    errors.push(error_msg);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(AssemblyError::GeneralError(format!(
                "Errors shutting down:\n {}",
                errors.join("\n")
            )))
        }
    }
}
