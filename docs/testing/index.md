# Testing in Io

## Unit Tests
```io
#[test]
fn test_addition() {
    assert_eq(2 + 2, 4);
}

#[test]
fn test_string_concat() {
    let result = "Hello" + " " + "World";
    assert_eq(result, "Hello World");
}
```

## Integration Tests
```io
// tests/integration_test.io
import { Database } from "../src/db";

#[test]
async fn test_database_connection() {
    let db = await Database::connect("localhost:5432");
    assert(db.is_connected());
}
```

## Test Fixtures
```io
#[fixture]
fn setup_database() -> Database {
    let db = Database::new();
    db.migrate();
    return db;
}
