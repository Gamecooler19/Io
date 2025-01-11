# Memory Management

## Ownership Model
```rust
fn example_ownership() {
    let s1 = String::from("hello"); // s1 owns the string
    let s2 = s1;                    // ownership moves to s2
    // println!("{}", s1);          // Error: s1 no longer valid
}
```

## Reference Management
### Borrowing Rules
1. One mutable reference OR many immutable references
2. References must not outlive their referent
3. No null references possible

```rust
fn example_borrowing() {
    let mut data = vec![1, 2, 3];
    let ref1 = &data;    // Immutable borrow
    let ref2 = &data;    // Multiple immutable borrows okay
    println!("{} {}", ref1[0], ref2[0]);
    
    let ref3 = &mut data;  // Now only mutable borrow allowed
    ref3[0] = 10;
}
```

## Memory Safety Features
- Stack vs Heap allocation decisions
- RAII (Resource Acquisition Is Initialization)
- Compile-time memory checks
- Zero-cost abstractions

## Advanced Memory Patterns
### Custom Allocators
```rust
#[global_allocator]
static ALLOCATOR: CustomAllocator = CustomAllocator::new();
```

### Memory Pools
```rust
struct MemoryPool<T> {
    chunks: Vec<Box<[T]>>,
    free_list: Vec<*mut T>,
}
```

## Performance Optimization
- Stack allocation preferences
- Avoiding unnecessary heap allocations
- Memory alignment considerations

## Debugging Tools
- Memory leak detection
- Heap profiling
- Address sanitizer integration
