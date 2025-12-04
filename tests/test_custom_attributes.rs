//! Tests for custom Rust attributes via @depyler: custom_attribute

use crate::test_helpers;
use crate::test_helpers::{parse_to_hir, transpile_and_check};

#[test]
fn test_single_custom_attribute() {
    let python_code = r#"
# @depyler: custom_attribute = "inline"
def add(a: int, b: int) -> int:
    return a + b
"#;
    transpile_and_check(python_code, &["#[inline]", "pub fn add"]);
}

#[test]
fn test_multiple_custom_attributes() {
    let python_code = r#"
# @depyler: custom_attribute = "inline"
# @depyler: custom_attribute = "must_use"
def calculate(x: int) -> int:
    return x * 2
"#;
    transpile_and_check(python_code, &["#[inline]", "#[must_use]", "pub fn calculate"]);
}

#[test]
fn test_custom_attribute_with_args() {
    let python_code = r#"
# @depyler: custom_attribute = "inline(always)"
def fast_function(n: int) -> int:
    return n + 1
"#;
    transpile_and_check(python_code, &["#[inline(always)]", "pub fn fast_function"]);
}

#[test]
fn test_custom_attributes_with_other_annotations() {
    let python_code = r#"
# @depyler: optimization_level = "aggressive"
# @depyler: custom_attribute = "inline"
# @depyler: performance_critical = "true"
def hot_path(items: list[int]) -> int:
    total = 0
    for item in items:
        total += item
    return total
"#;
    transpile_and_check(python_code, &["#[inline]", "pub fn hot_path"]);
}

#[test]
fn test_no_custom_attributes() {
    let python_code = r#"
def normal_function(x: int) -> int:
    return x * 2
"#;
    transpile_and_check(python_code, &["pub fn normal_function"]);
}

#[test]
fn test_custom_attribute_cold() {
    let python_code = r#"
# @depyler: custom_attribute = "cold"
def error_handler(msg: str) -> None:
    print(msg)
"#;
    transpile_and_check(python_code, &["#[cold]", "pub fn error_handler"]);
}

#[test]
fn test_custom_attribute_repr() {
    let python_code = r#"
# @depyler: custom_attribute = "repr(C)"
def get_layout() -> int:
    return 42
"#;
    transpile_and_check(python_code, &["#[repr(C)]", "pub fn get_layout"]);
}

#[test]
fn test_multiple_functions_different_attributes() {
    let python_code = r#"
# @depyler: custom_attribute = "inline"
def fast_func(x: int) -> int:
    return x + 1

# @depyler: custom_attribute = "cold"
def slow_func(x: int) -> int:
    return x - 1
"#;
    transpile_and_check(python_code, &["#[inline]", "#[cold]", "pub fn fast_func", "pub fn slow_func"]);
}

#[test]
fn test_custom_attribute_with_docstring() {
    let python_code = r#"
# @depyler: custom_attribute = "inline"
def documented_function(x: int) -> int:
    """This is a documented function."""
    return x * 2
"#;
    transpile_and_check(python_code, &["#[inline]", "pub fn documented_function"]);
}

#[test]
fn test_parse_to_hir_preserves_custom_attributes() {
    let python_code = r#"
# @depyler: custom_attribute = "inline"
# @depyler: custom_attribute = "must_use"
def test_func(x: int) -> int:
    return x
"#;
    let hir = parse_to_hir(python_code);
    
    assert_eq!(hir.functions.len(), 1);
    let func = &hir.functions[0];
    assert_eq!(func.annotations.custom_attributes.len(), 2);
    assert_eq!(func.annotations.custom_attributes[0], "inline");
    assert_eq!(func.annotations.custom_attributes[1], "must_use");
}
