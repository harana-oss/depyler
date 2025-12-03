mod test_helpers;

use test_helpers::{transpile, transpile_and_check};

#[test]
fn test_simple_constants() {
    let python_code = r#"
A = 1
B = 2
C = 3
"#;

    let rust_code = transpile_and_check(python_code, &["A", "B", "C", "1", "2", "3"]);

    let has_const_declarations = rust_code.contains("const A")
        || rust_code.contains("pub const A")
        || rust_code.contains("static A")
        || rust_code.contains("pub static A");

    assert!(
        has_const_declarations,
        "Constants should be declared with const or static keyword. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_constants_with_types() {
    let python_code = r#"
A: int = 1
B: int = 2
C: int = 3
"#;

    let rust_code = transpile_and_check(python_code, &["A", "B", "C"]);

    let has_type_annotations = rust_code.contains("i32") || rust_code.contains("i64");

    assert!(
        has_type_annotations,
        "Constants should have integer type annotations. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_string_constants() {
    let python_code = r#"
NAME = "Alice"
GREETING = "Hello"
MESSAGE = "World"
"#;

    let rust_code = transpile(python_code);

    assert!(rust_code.contains("NAME"), "Should contain NAME");
    assert!(rust_code.contains("GREETING"), "Should contain GREETING");
    assert!(rust_code.contains("MESSAGE"), "Should contain MESSAGE");
    assert!(rust_code.contains("Alice"), "Should contain Alice");
    assert!(rust_code.contains("Hello"), "Should contain Hello");
    assert!(rust_code.contains("World"), "Should contain World");
}

#[test]
fn test_mixed_type_constants() {
    let python_code = r#"
INT_VALUE = 42
FLOAT_VALUE = 3.14
STRING_VALUE = "test"
BOOL_VALUE = True
"#;

    let rust_code = transpile(python_code);

    assert!(rust_code.contains("INT_VALUE"), "Should contain INT_VALUE");
    assert!(rust_code.contains("FLOAT_VALUE"), "Should contain FLOAT_VALUE");
    assert!(rust_code.contains("STRING_VALUE"), "Should contain STRING_VALUE");
    assert!(rust_code.contains("BOOL_VALUE"), "Should contain BOOL_VALUE");
    assert!(rust_code.contains("42"), "Should contain 42");
    assert!(rust_code.contains("3.14"), "Should contain 3.14");
    assert!(rust_code.contains("test"), "Should contain test");
    assert!(
        rust_code.contains("true") || rust_code.contains("True"),
        "Generated code must contain boolean value. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_generated_constants_code_compiles() {
    let python_code = r#"
A = 1
B = 2
C = 3
"#;

    transpile_and_check(python_code, &["A"]);
}

#[test]
fn test_constants_used_in_function() {
    let python_code = r#"
A = 1
B = 2
C = 3

def sum_constants() -> int:
    return A + B + C
"#;

    transpile_and_check(python_code, &["A", "B", "C", "sum_constants"]);
}

#[test]
fn test_integer_list_constant() {
    let python_code = r#"
VEC = [1, 2, 3, 4]
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("VEC"), "Should contain VEC");
}

#[test]
fn test_string_list_constant() {
    let python_code = r#"
VEC = ["1", "2", "3", "4"]
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("VEC"), "Should contain VEC");
}

#[test]
fn test_float_list_constant() {
    let python_code = r#"
VEC = [1.1, 2.2, 3.3, 4.4]
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("VEC"), "Should contain VEC");
}

#[test]
fn test_bool_list_constant() {
    let python_code = r#"
VEC = [True, False, True, False]
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("VEC"), "Should contain VEC");
}

#[test]
fn test_nested_integer_list_constant() {
    let python_code = r#"
X_VALUES = [[0, 100], [100, 200]]
"#;

    let rust_code = transpile(python_code);

    assert!(rust_code.contains("lazy_static!"), "Should use lazy_static!");
    assert!(
        rust_code.contains("pub static ref X_VALUES: Vec<Vec<i32>> = vec![vec![0, 100], vec![100, 200]]"),
        "Nested list must be declared as static ref with correct type and value. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_string_list_constant_multiple() {
    let python_code = r#"
STRINGS = ["one", "two", "three"]
"#;

    let rust_code = transpile(python_code);

    assert!(rust_code.contains("lazy_static!"), "Should use lazy_static!");
    assert!(
        rust_code.contains("pub static ref STRINGS: Vec<String> = vec![\"one\".to_string(), \"two\".to_string(), \"three\".to_string()]"),
        "String list must be declared as static ref with correct type and value. Got:\n{}",
        rust_code
    );
}
