# Types in Io

Io provides powerful and flexible type definitions to help you write safe, efficient code. Below are key type categories along with brief examples illustrating their use in common scenarios.

## Basic Types

### Numbers
```io
let integer: int = 42;
let float: float = 3.14;
```

### Strings
```io
let greeting: string = "Hello";
let multiline: string = """
    Multiple
    lines
    text
""";
```

### Booleans
```io
let is_valid: bool = true;
let has_error: bool = false;
```

## Compound Types

### Arrays
```io
let numbers: [int] = [1, 2, 3, 4, 5];
let mixed: [any] = [1, "two", true];
```

### Maps
```io
let person: map = {
    name: "John",
    age: 30
};
```

## Type Aliases
```io
type ID = int;
type Text = string;
```
Type aliases can improve code readability and enforce conceptual clarity.

## Type Inference
Io supports smart type inference:

```io
let inferred = 42;  // Type int
let auto_array = [1, 2, 3];  // Type [int]
```

Type safety is a core principle in Io, preventing common runtime errors.
