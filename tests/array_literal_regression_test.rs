/// DEPYLER-0161: Array Literal Transpilation Regression Tests
///
/// BUG: Array literal assignments are being dropped during code generation.
/// ALL variable assignments are missing from generated code, leaving only
/// return statements with undefined variables.
///
/// EXTREME TDD: These tests MUST fail first, then we fix the transpiler.
mod test_helpers;
use test_helpers::transpile;

#[test]
fn test_simple_array_literal_assignment() {
    let python_code = r#"
def test_array():
    arr = [1, 2, 3]
    return arr
"#;

    let rust_code = transpile(python_code);

    assert!(
        rust_code.contains("arr = ") || rust_code.contains("let arr"),
        "Generated code MUST contain array assignment. Got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("[1") || rust_code.contains("vec!"),
        "Generated code MUST contain array literal initialization. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_multiple_array_assignments() {
    let python_code = r#"
def test_arrays():
    arr1 = [1, 2, 3]
    arr2 = [4, 5, 6]
    arr3 = [7, 8, 9]
    return arr1, arr2, arr3
"#;

    let rust_code = transpile(python_code);

    assert!(
        rust_code.contains("arr1") && rust_code.contains("arr2") && rust_code.contains("arr3"),
        "All array variables must be present. Got:\n{}",
        rust_code
    );

    let assignment_count = rust_code.matches("arr1 =").count() + rust_code.matches("let arr1").count();
    assert!(
        assignment_count > 0,
        "arr1 must have an assignment statement. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_array_with_booleans() {
    let python_code = r#"
def test_bool_array():
    flags = [True, False, True]
    return flags
"#;

    let rust_code = transpile(python_code);

    assert!(
        rust_code.contains("flags = ") || rust_code.contains("let flags"),
        "Generated code MUST contain flags assignment. Got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("true") || rust_code.contains("false"),
        "Generated code MUST contain boolean literals. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_generated_code_compiles() {
    let python_code = r#"
def simple_array():
    nums = [1, 2, 3]
    return nums
"#;

    let rust_code = transpile(python_code);

    let parsed = syn::parse_file(&rust_code);

    assert!(
        parsed.is_ok(),
        "Generated Rust code must be syntactically valid. Parse error: {:?}\nGenerated code:\n{}",
        parsed.err(),
        rust_code
    );
}

#[cfg(test)]
mod property_tests {
    use depyler_core::DepylerPipeline;
    use quickcheck::{TestResult, quickcheck};

    fn prop_array_assignment_generates_valid_rust(size: usize) -> TestResult {
        if size > 20 {
            return TestResult::discard();
        }

        let elements: Vec<String> = (0..size).map(|i| i.to_string()).collect();
        let python_code = format!("def test_array():\n    arr = [{}]\n    return arr", elements.join(", "));

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(&python_code);

        if result.is_err() {
            return TestResult::error("Transpilation failed");
        }

        let rust_code = result.unwrap();

        let has_assignment = rust_code.contains("arr = ") || rust_code.contains("let arr");

        TestResult::from_bool(has_assignment)
    }

    #[test]
    fn test_property_array_assignments() {
        quickcheck(prop_array_assignment_generates_valid_rust as fn(usize) -> TestResult);
    }
}
