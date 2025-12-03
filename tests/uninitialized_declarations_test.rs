//! Integration tests for annotated declarations without initial values
//!
//! Tests the full pipeline from Python code with annotated declarations
//! (e.g., `field: Type`) to Rust code generation (e.g., `let mut field: Type;`).
//!
//! Test Strategy:
//! - Annotated declarations without values for various types
//! - Immutable vs mutable variable detection
//! - Type annotations (int, str, bool, custom types, Optional)
//! - Integration with rest of function body
//! - Edge cases and error handling

use depyler_core::DepylerPipeline;

/// Test basic int type annotation without initial value
#[test]
fn test_uninitialized_int_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    count: int
    count = 5
    return count
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut count: i32;
    assert!(
        rust_code.contains("let mut count: i32;"),
        "Expected 'let mut count: i32;' in generated code:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("count = 5"),
        "Expected assignment 'count = 5' in generated code:\n{}",
        rust_code
    );
}

/// Test string type annotation without initial value
#[test]
fn test_uninitialized_string_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    name: str
    name = "Alice"
    return name
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut name: String;
    assert!(
        rust_code.contains("let mut name: String;") || rust_code.contains("let mut name : String ;"),
        "Expected 'let mut name: String;' in generated code:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("name = \"Alice\"") || rust_code.contains("name = String :: from(\"Alice\")"),
        "Expected name assignment in generated code:\n{}",
        rust_code
    );
}

/// Test bool type annotation without initial value
#[test]
fn test_uninitialized_bool_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def check():
    valid: bool
    valid = True
    return valid
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut valid: bool;
    assert!(
        rust_code.contains("let mut valid: bool;") || rust_code.contains("let mut valid : bool ;"),
        "Expected 'let mut valid: bool;' in generated code:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("valid = true"),
        "Expected assignment 'valid = true' in generated code:\n{}",
        rust_code
    );
}

/// Test custom type annotation without initial value
#[test]
fn test_uninitialized_custom_type_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    position: FieldPosition
    position = FieldPosition()
    return position
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut position: FieldPosition;
    assert!(
        rust_code.contains("position: FieldPosition;") || rust_code.contains("position : FieldPosition ;"),
        "Expected 'position: FieldPosition;' in generated code:\n{}",
        rust_code
    );
}

/// Test Optional type annotation without initial value
#[test]
fn test_uninitialized_optional_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    value: Optional[int]
    value = None
    return value
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut value: Option<i32>;
    assert!(
        rust_code.contains("value: Option") || rust_code.contains("value : Option"),
        "Expected 'value: Option<i32>;' in generated code:\n{}",
        rust_code
    );
}

/// Test multiple uninitialized declarations
#[test]
fn test_multiple_uninitialized_declarations() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    field_position: FieldPosition
    new_team: str
    valid: bool
    
    field_position = FieldPosition()
    new_team = "Team A"
    valid = True
    
    return valid
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate declarations for all three
    assert!(
        rust_code.contains("field_position") && rust_code.contains("FieldPosition"),
        "Expected field_position declaration in:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("new_team") && rust_code.contains("String"),
        "Expected new_team declaration in:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("valid") && rust_code.contains("bool"),
        "Expected valid declaration in:\n{}",
        rust_code
    );
}

/// Test uninitialized declaration followed by conditional assignment
#[test]
fn test_uninitialized_with_conditional_assignment() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process(flag: bool):
    result: int
    if flag:
        result = 10
    else:
        result = 20
    return result
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut result: i32;
    assert!(
        rust_code.contains("let mut result: i32;") || rust_code.contains("let mut result : i32 ;"),
        "Expected 'let mut result: i32;' in generated code:\n{}",
        rust_code
    );

    // Should have both assignments in if/else branches
    assert!(
        rust_code.contains("result = 10") && rust_code.contains("result = 20"),
        "Expected conditional assignments in generated code:\n{}",
        rust_code
    );
}

/// Test string variable assigned in both branches of if/else (deferred initialization)
#[test]
fn test_string_conditional_assignment() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    if True:
        val = "one"
    else:
        val = "two"
    return val
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();
    println!("Generated Rust code:\n{}", rust_code);

    // Should be immutable since val is only assigned once (in whichever branch executes)
    assert!(
        rust_code.contains("let val;"),
        "Expected immutable 'let val;' (not 'let mut val;') in generated code:\n{}",
        rust_code
    );

    // Should have both string assignments
    assert!(
        rust_code.contains("\"one\"") && rust_code.contains("\"two\""),
        "Expected both string assignments in generated code:\n{}",
        rust_code
    );
}

/// Test that annotated assignment WITH value still works correctly
#[test]
fn test_annotated_with_value_unchanged() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    count: int = 42
    return count
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let count: i32 = 42; (or let mut if mutated)
    assert!(
        rust_code.contains("count") && rust_code.contains("42"),
        "Expected count = 42 in generated code:\n{}",
        rust_code
    );

    // Should NOT have separate declaration line
    let count_lines: Vec<&str> = rust_code.lines().filter(|line| line.contains("count")).collect();

    // Should be one or two lines (declaration+assignment or combined), not three
    assert!(
        count_lines.len() <= 2,
        "Expected at most 2 lines with 'count', found {} in:\n{}",
        count_lines.len(),
        rust_code
    );
}

/// Test uninitialized float declaration
#[test]
fn test_uninitialized_float_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def calculate():
    result: float
    result = 3.14
    return result
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut result: f64;
    assert!(
        rust_code.contains("result: f64;") || rust_code.contains("result : f64 ;"),
        "Expected 'result: f64;' in generated code:\n{}",
        rust_code
    );
}

/// Test uninitialized list type declaration
#[test]
fn test_uninitialized_list_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    items: list[int]
    items = [1, 2, 3]
    return items
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate: let mut items: Vec<i32>;
    assert!(
        rust_code.contains("items") && rust_code.contains("Vec"),
        "Expected items: Vec declaration in generated code:\n{}",
        rust_code
    );
}

/// Test that immutable uninitialized declaration works (if never reassigned after first assignment)
#[test]
fn test_uninitialized_immutable_declaration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process():
    value: int
    value = 100
    # value is never reassigned, so could be immutable
    return value * 2
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate declaration (may be let or let mut depending on mutability analysis)
    assert!(
        rust_code.contains("value") && rust_code.contains("i32"),
        "Expected value: i32 declaration in generated code:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("100"),
        "Expected value = 100 in generated code:\n{}",
        rust_code
    );
}

/// Test error handling: uninitialized used in expression (should fail at compile time in Rust)
#[test]
fn test_uninitialized_usage_compiles_to_rust() {
    // This test verifies we generate syntactically valid Rust code
    // even if the logic is incorrect (using uninitialized variable)
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def bad_usage():
    count: int
    return count  # Used without assignment - invalid in both Python and Rust
"#;

    // Should transpile without error (semantic checking is Rust compiler's job)
    let result = pipeline.transpile(python_code);

    // We generate Rust code; it will fail at Rust compile time with proper error
    assert!(
        result.is_ok(),
        "Should generate Rust code (even if semantically invalid)"
    );

    if let Ok(rust_code) = result {
        assert!(
            rust_code.contains("count: i32;") || rust_code.contains("count : i32 ;"),
            "Expected declaration in generated code:\n{}",
            rust_code
        );
    }
}
