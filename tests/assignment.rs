use depyler_core::DepylerPipeline;

fn transpile_snippet(python_code: &str) -> Result<String, String> {
    let pipeline = DepylerPipeline::new();
    pipeline
        .transpile(python_code)
        .map_err(|e| format!("Transpilation error: {e}"))
}

#[test]
fn test_tuple_swap_array_indices() {
    let py_code = r#"
def swap_elements():
    a = [1, 2, 3]
    a[0], a[2] = a[2], a[0]
    return a
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");
    println!("Generated Rust code:\n{rust_code}");

    assert!(rust_code.contains("vec!"));
}
