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

    let result = transpile_snippet(py_code);
    match result {
        Ok(rust_code) => {
            println!("Generated Rust code:\n{rust_code}");
            // Verify the code contains the swap pattern
            assert!(rust_code.contains("vec!"), "Should contain vec! for array init");
            assert!(rust_code.contains("_swap_tmp"), "Should use temp variables for swap");
            assert!(rust_code.contains("a[0"), "Should have index assignment for a[0]");
            assert!(rust_code.contains("a[2"), "Should have index assignment for a[2]");
        }
        Err(e) => {
            println!("Transpilation failed with error:\n{e}");
            panic!("Transpilation failed: {e}");
        }
    }
}