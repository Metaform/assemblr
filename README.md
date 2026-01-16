![Assemblr](/assets/assemblr.logo.svg)

Assemblr is a Rust library for composing runtimes from modular service assemblies.

## The Service Assembly Abstraction

A **ServiceAssembly** is a subsystem abstraction that encapsulates services and their lifecycle within a runtime. Each
assembly declares:

- **Provides**: The services (types) it registers and makes available
- **Requires**: The services it depends on from other assemblies
- **Lifecycle**: Initialization, preparation, startup, finalization, and shutdown phases

Assemblies enable modular, composable service architectures with clear dependency boundaries. `ServiceAssembly`
instances are composed into a runtime using an `Assembler`.

## Using the `Assembler`

The **Assembler** orchestrates assembly initialization through a three-phase process:

1. **Registration**: Assemblies are registered via `register()`
2. **Assembly**: resolves dependencies, detects cycles, orders assemblies, and executes lifecycle phases (init →
   prepare → start) `assemble()`
3. **Shutdown**: gracefully tears down assemblies in reverse order (finalize → shutdown) `shutdown()`

``` rust
use assemblr::assembly::{Assembler, RuntimeMode, NoopMonitor};

let monitor = Arc::new(NoopMonitor);
let assembler = Assembler::new(monitor, RuntimeMode::Production);
assembler.register(Arc::new(assembly_a));
assembler.register(Arc::new(assembly_b));
assembler.assemble()?;  // Resolves and initializes

assembler.shutdown()?;  // Cleans up
```

### Creating Service Assemblies

The `#[assembly]` macro simplifies implementing the `ServiceAssembly` trait by automatically generating the
`name()`, `provides()`, and `requires()` methods.

#### Simple Assembly

```rust
use assembly_macros::assembly;
use assemblr::assembly::{MutableAssemblyContext, ServiceAssembly, Result};
use std::sync::Arc;

#[assembly(
    name = "DatabaseAssembly",
    provides = [Database])]
struct DatabaseAssembly {}

impl ServiceAssembly for DatabaseAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Register services using the RegistryWriteHandle
        let db = Arc::new(Database::new());
        context.registry.register(db);
        Ok(())
    }
}
```

This assembly provides a single service, `Database`. The `name` attribute is optional. Assemblies may have 0..N provided
services and 0..N required services. The `ServiceAssembly` trait must be implemented with the `init()` method.

The macro generates:

```rust
impl ServiceAssemblyBase for SimpleAssembly {
    fn name(&self) -> &str {
        "SimpleAssembly"
    }

    fn provides(&self) -> Vec<TypeKey> {
        vec![TypeKey::new::<Foo>()]
    }

    fn requires(&self) -> Vec<TypeKey> {
        Vec::new()
    }
}
```

#### Assembly with Dependencies

```rust
#[assembly(
    provides = [Database],
    requires = [ConnectionPool])]
struct DatabaseAssembly {}
```

#### Multiple Provides and Requires

```rust
#[assembly(
    provides = [ServiceA, ServiceB, ServiceC],
    requires = [ServiceD, ServiceE, ServiceF])]
struct CustomAssembly {}
```

#### Trait Objects

Trait objects are handled as follows:

```rust
#[assembly(
    provides = [Box<dyn HttpServer>, Box<dyn Router>],
    requires = [Box<dyn Database>, Box<dyn Cache>])]
struct CustomAssembly {}
```

### Registration Helper Macros

The library provides convenience macros for registering services:

#### `register!` - Register Concrete Types

```rust
use assemblr::register;

impl ServiceAssembly for CustomAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Automatically wraps instance in Arc
        register!(&context.registry, MyService::new());
        Ok(())
    }
}
```

#### `register_trait!` - Register Trait Objects

```rust
use assemblr::register_trait;

impl ServiceAssembly for DatabaseAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Automatically wraps in Arc<Box<dyn Trait>>
        register_trait!(&context.registry, dyn Database, PostgresDatabase);
        Ok(())
    }
}
```

#### `resolve_trait!` - Resolve Trait Objects

Works with both `ServiceRegistry` and `RegistryWriteHandle`.

**Note**: Panics if the service is not registered. Use this when you expect the service to always be available.

```rust
use assemblr::resolve_trait;

impl ServiceAssembly for CacheAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Resolve a dependency registered by another assembly
        // Works with RegistryWriteHandle from MutableAssemblyContext
        let db = resolve_trait!(&context.registry, dyn Database);

        // Use the dependency and register your service
        register_trait!(&context.registry, dyn Cache, RedisCache::new(db));
        Ok(())
    }
}
```

### Implementing Additional Lifecycle Methods

`ServiceAssembly` lifecycle callbacks are available for `init()`, `prepare()`, `start()`, `finalize()` and
`shutdown`. Default implementations are provided for all methods except `init()`:

```rust
impl ServiceAssembly for CustomAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Register provided services
        register!(&context.registry, MyService::new());
        Ok(())
    }

    fn prepare(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Prepare resources after all assemblies are initialized
        Ok(())
    }

    fn start(&self, context: &AssemblyContext) -> Result<()> {
        // Start background tasks, servers, etc.
        Ok(())
    }

    fn finalize(&self) -> Result<()> {
        // Pre-shutdown cleanup
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        // Final shutdown and resource release
        Ok(())
    }
}
```

## Complete Example

Here's a complete example showing how to build a runtime with dependent assemblies:

```rust
use assemblr::assembly::{Assembler, AssemblyContext, MutableAssemblyContext, 
                         RuntimeMode, NoopMonitor, ServiceAssembly, Result};
use assemblr::{register_trait, resolve_trait};
use assembly_macros::assembly;
use std::sync::Arc;

// Define your service traits
trait Database: Send + Sync {
    fn query(&self) -> String;
}

trait Cache: Send + Sync {
    fn get(&self) -> String;
}

// Implement concrete services
struct PostgresDb;
impl Database for PostgresDb {
    fn query(&self) -> String {
        "postgres_data".to_string()
    }
}

struct RedisCache {
    db: Arc<Box<dyn Database>>,
}

impl Cache for RedisCache {
    fn get(&self) -> String {
        format!("cached: {}", self.db.query())
    }
}

// Create assemblies
#[assembly(provides = [Box<dyn Database>])]
struct DatabaseAssembly {}

impl ServiceAssembly for DatabaseAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        register_trait!(&context.registry, dyn Database, PostgresDb);
        Ok(())
    }
}

#[assembly(provides = [Box<dyn Cache>], requires = [Box<dyn Database>])]
struct CacheAssembly {}

impl ServiceAssembly for CacheAssembly {
    fn init(&self, context: &MutableAssemblyContext) -> Result<()> {
        // Resolve the database dependency
        let db = resolve_trait!(&context.registry, dyn Database);

        // Register the cache service
        register_trait!(&context.registry, dyn Cache, RedisCache { db });
        Ok(())
    }
}

// Compose and run
fn main() -> Result<()> {
    let monitor = Arc::new(NoopMonitor);
    let assembler = Assembler::new(monitor, RuntimeMode::Production);

    // Register assemblies (order doesn't matter - dependencies are resolved automatically)
    assembler.register(Arc::new(CacheAssembly {}));
    assembler.register(Arc::new(DatabaseAssembly {}));

    // Assemble initializes in the correct dependency order
    assembler.assemble()?;

    // Services are now available through the registry
    // ... application runs ...

    // Clean shutdown
    assembler.shutdown()?;
    Ok(())
}
```
