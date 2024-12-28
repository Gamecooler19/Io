# Memory Management in Io

## Ownership Rules
- Each value has exactly one owner
- When owner goes out of scope, value is dropped
- Values can be moved or borrowed

## Examples

### Ownership
```io
fn take_ownership(value: String) {
    println(value);
    // value is dropped here
}

fn main() {
    let text = "Hello";
    take_ownership(text);
    // text is no longer valid here
}
```

### Borrowing
```io
fn borrow_value(value: &String) {
    println(value);
    // value is still valid after this function
}

fn main() {
    let text = "Hello";
    borrow_value(&text);
    println(text); // Still works
}
```
