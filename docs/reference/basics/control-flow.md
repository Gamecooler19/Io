# Control Flow in Io

Control flow in Io consists of conditionals, loops, and pattern matching. These constructs allow you to build clear and concise logic without sacrificing readability.

## Conditionals

### If Expressions
```io
if condition {
    // then branch
} else if other_condition {
    // else if branch
} else {
    // else branch
}
```

## Loops

### For Loops
```io
for item in collection {
    println(item);
}

for i in 0..10 {
    println(i);
}
```

### While Loops
```io
while condition {
    // loop body
}
```

## Pattern Matching
```io
match value {
    0 => println("Zero"),
    1 => println("One"),
    _ => println("Something else"),
}
```

## Best Practices
- Use `match` for expressive branching over multiple cases.
- Keep loops small and modular; extract complex logic to functions.
