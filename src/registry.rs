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

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Register a trait object: `register_trait!(registry_handle, dyn MyTrait, MyImpl)`
/// Creates Arc<Box<dyn Trait>> automatically
///
/// # Parameters
/// * `registry_handle` - A mutable reference to `RegistryWriteHandle`
#[macro_export]
macro_rules! register_trait {
    ($registry:expr, $trait_type:ty, $instance:expr) => {{
        let __registry: &RegistryWriteHandle = $registry;
        __registry.register::<Box<$trait_type>>(std::sync::Arc::new(
            Box::new($instance) as Box<$trait_type>
        ))
    }};
}

/// Resolve a trait object: `resolve_trait!(registry, dyn MyTrait)`
///
/// # Parameters
/// * `registry` - A reference to `ServiceRegistry` or `RegistryWriteHandle`
#[macro_export]
macro_rules! resolve_trait {
    ($registry:expr, $trait_type:ty) => {{
        ($registry).resolve::<Box<$trait_type>>()
    }};
}

/// Register a concrete type: `register!(registry_handle, instance)`
/// Automatically wraps the instance in Arc
///
/// # Parameters
/// * `registry_handle` - A mutable reference to `RegistryWriteHandle`
#[macro_export]
macro_rules! register {
    ($registry:expr, $instance:expr) => {{
        let __registry: &RegistryWriteHandle = $registry;
        __registry.register(std::sync::Arc::new($instance))
    }};
}

/// A registry that maps service types to their instances
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
    pub(crate) fn register<T: Any + Send + Sync + 'static>(&self, service: Arc<T>) {
        let mut services = self.services.write().unwrap();
        services.insert(TypeId::of::<T>(), service as Arc<dyn Any + Send + Sync>);
    }

    /// Get a registered service
    ///
    /// # Panics
    /// Panics if the service is not registered
    pub fn resolve<T: Any + Send + Sync + 'static>(&self) -> Arc<T> {
        let services = self.services.read().unwrap();
        services
            .get(&TypeId::of::<T>())
            .and_then(|service| service.clone().downcast::<T>().ok())
            .unwrap_or_else(|| panic!("Service '{}' not found in registry", std::any::type_name::<T>()))
    }

    /// Check if a service is registered
    pub fn contains<T: Any + 'static>(&self) -> bool {
        self.services
            .read()
            .unwrap()
            .contains_key(&TypeId::of::<T>())
    }
}

pub struct RegistryWriteHandle {
    services: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl RegistryWriteHandle {
    pub fn new(registry: &ServiceRegistry) -> Self {
        RegistryWriteHandle {
            services: Arc::clone(&registry.services),
        }
    }

    pub fn register<T: Any + Send + Sync + 'static>(&self, service: Arc<T>) {
        let mut services = self.services.write().unwrap();
        services.insert(TypeId::of::<T>(), service as Arc<dyn Any + Send + Sync>);
    }

    pub fn resolve<T: Any + Send + Sync + 'static>(&self) -> Arc<T> {
        let services = self.services.read().unwrap();
        services
            .get(&TypeId::of::<T>())
            .and_then(|service| service.clone().downcast::<T>().ok())
            .unwrap_or_else(|| panic!("Service '{}' not found in registry", std::any::type_name::<T>()))
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
