# File I/O Examples

This guide demonstrates how to safely read and write files, handle JSON, and display meaningful error messages, ensuring robust data management in Io applications.

## Reading a File
```io
fn read_example() -> Result<string> {
    let content = fs::read_to_string("input.txt")?;
    println("Read: " + content);
    Ok(content)
}
```

## Writing to a File
```io
fn write_example() -> Result<()> {
    let data = "Hello, File I/O!";
    fs::write("output.txt", data)?;
    Ok(())
}
```

## Working with JSON
```io
fn handle_json() -> Result<()> {
    let config = {
        name: "test",
        version: "1.0.0"
    };
    
    fs::write_json("config.json", config)?;
    Ok(())
}
```

Always handle potential I/O errors to avoid crashes:
```io
fn safe_read(path: string) -> Result<string> {
    if fs::exists(path) {
        fs::read_to_string(path)
    } else {
        Err("File does not exist.")
    }
}
```
