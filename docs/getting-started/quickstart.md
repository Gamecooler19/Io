# Quick Start Guide

This guide will help you get started with Io quickly.

## Hello World

Let's start with a simple "Hello World" program in Io:

```rust
fn main() {
    println!("Hello, world!");
}
```

## Project Structure
Io projects follow a standardized layout:
```
my_project/
├── src/
│   ├── main.cb         # Entry point
│   ├── lib.cb          # Library code
│   └── modules/        # Additional modules
├── tests/              # Test files
├── docs/              # Documentation
├── examples/          # Example code
└── build.toml         # Build configuration
```

### Key Files
- `main.cb`: The entry point for executables
- `lib.cb`: Contains shared library code
- `build.toml`: Configures compilation options, dependencies, and metadata

### Best Practices
- Keep modules small and focused
- Use clear naming conventions
- Separate business logic from infrastructure code

## Multi-File Projects
- Demonstrate splitting code into modules.
- Show how to compile with “cargo build” when there are multiple sources.

## Development Workflow
1. **Project Setup**
   ```bash
   Io new my_project
   cd my_project
   ```

2. **Development Cycle**
   - Write tests first (TDD approach)
   - Implement features
   - Run automated checks:
     ```bash
     Io check        # Static analysis
     Io test        # Run tests
     Io bench       # Performance tests
     ```

3. **Code Organization**
   ```rust
   // Example of well-organized module structure
   mod core {
       mod types;
       mod validation;
       pub mod api;
   }

   mod features {
       pub mod auth;
       pub mod processing;
   }
   ```

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_basic_arithmetic() {
    assert_eq!(add(2, 2), 4);
    assert_eq!(multiply(3, 3), 9);
}
```

### Integration Tests
```rust
#[test]
fn test_full_workflow() {
    let app = Application::new();
    let result = app.process_transaction(Transaction::new());
    assert!(result.is_ok());
}
```

### Performance Testing
```rust
#[bench]
fn bench_large_dataset() {
    // Performance test implementation
}
```

## Performance Tuning
- Compiler optimization flags and usage of “release” builds.
- Profiling techniques to find bottlenecks in your code.

## Advanced Configuration

### Custom Build Scripts
```toml
[build]
optimization = "high"
target = "x86_64-unknown-linux-gnu"
features = ["parallel", "simd"]

[dependencies]
core = "1.0"
async = { version = "2.0", features = ["tokio"] }
```

### Environment-Specific Settings
```toml
[environments.production]
logging = "warn"
workers = 8

[environments.development]
logging = "debug"
workers = 2
```

## Next Steps

Now that you have a basic understanding of how to write a simple program in Io, you can explore the following resources to learn more:

- [Installation Guide](getting-started/installation.md)
- [Type System](core/type-system.md)
- [Functions](core/functions.md)