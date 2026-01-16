# Creating a Service Assembly

The `#[assembly]` attribute macro simplifies implementing the `ServiceAssembly` trait by automatically generating the
`name()`, `provides()`, and `requires()` methods.

## Basic Usage

### Simple Assembly (No Dependencies)

```rust
use assembly_macros::assembly;

#[assembly(
    name = "CustomAssembly",
    provides = [Foo])]
struct CustomAssembly {}

impl ServiceAssembly for CustomAssembly {
    fn init(&self, context: &AssemblyContext) -> Result<()> {
        // Your custom init logic here
        let service = Arc::new(MyService::new());
        context.registry.register(service);
        Ok(())
    }
}
```

This assembly provides a single service, `Foo`. The `name` attribute is optional. Assemblies may have 0..N provided
services and 0..N required services. The `ServiceAssembly` trait must be implemented with the `init()` method.

The macros generates:

```rust
impl ServiceAssembly for SimpleAssembly {
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

### Assembly with Dependencies

```rust
#[assembly(
    name = "CustomAssembly",
    provides = [Box<dyn Database>],
    requires = [Box<dyn ConnectionPool>])]
struct CustomAssembly {}
```

### Multiple Provides and Requires

```rust
#[assembly(
    name = "CustomAssembly",
    provides = [ServiceA, ServiceB, ServiceC],
    requires = [ServiceD, ServiceE, ServiceF])]
struct CustomAssembly {}
```

### Trait Objects

Trait objects are handled as follows:

```rust
#[assembly(
    name = "CustomAssembly",
    provides = [Box<dyn HttpServer>, Box<dyn Router>],
    requires = [Box<dyn Database>, Box<dyn Cache>])]
struct CustomAssembly {}
```

## Implementing Additional Lifecycle Methods

`ServiceAssembly` lifecycle callbacks are available for `init()`, `prepare()`, `start()`, `finalize()` and
`shutdown`. Default implementations are provided for all methods except `init()`:

```rust
struct CustomAssembly {}

impl ServiceAssembly for CustomAssembly {
    fn init(&self, context: &AssemblyContext) -> Result<()> {
        // Register provided services
        let service = Arc::new(MyService::new());
        context.registry.register(service);
        Ok(())
    }
}
```
