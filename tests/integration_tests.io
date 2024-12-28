#[test]
fn test_compiler_pipeline() {
    let result = compile_file("examples/hello_world.io");
    assert(result.is_ok());
}

#[test]
fn test_error_handling() {
    let result = compile_file("non_existent.io");
    assert(result.is_err());
    assert_eq(result.unwrap_err(), "File not found");
}

#[test]
fn test_standard_library() {
    let test_cases = [
        ("println", test_println),
        ("read_line", test_input),
        ("to_string", test_conversion)
    ];
    
    for (name, test_fn) in test_cases {
        assert(test_fn(), "Standard library test failed: " + name);
    }
}
