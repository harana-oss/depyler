//! Integration tests for v3.17.0 Phase 3 - Test Coverage Improvements

mod test_helpers;
use test_helpers::transpile_and_check;

// =============================================================================
// String Method Tests (exercises rust_gen.rs string operations)
// =============================================================================

#[test]
fn test_string_upper_method() {
    let python = r#"
def make_loud(text: str) -> str:
    return text.upper()
"#;
    transpile_and_check(python, &["to_uppercase", "fn make_loud", "String"]);
}

#[test]
fn test_string_lower_method() {
    let python = r#"
def make_quiet(text: str) -> str:
    return text.lower()
"#;
    transpile_and_check(python, &["to_lowercase", "fn make_quiet"]);
}

#[test]
fn test_string_strip_method() {
    let python = r#"
def clean_text(text: str) -> str:
    return text.strip()
"#;
    transpile_and_check(python, &["trim", "fn clean_text"]);
}

#[test]
fn test_string_replace_method() {
    let python = r#"
def replace_spaces(text: str) -> str:
    return text.replace(" ", "_")
"#;
    transpile_and_check(python, &["replace", "fn replace_spaces"]);
}

// =============================================================================
// Division Operator Tests (exercises rust_gen.rs arithmetic operations)
// =============================================================================

#[test]
fn test_true_division_with_floats() {
    let python = r#"
def divide_floats(a: float, b: float) -> float:
    return a / b
"#;
    transpile_and_check(python, &["f64", "/", "fn divide_floats"]);
}

#[test]
fn test_floor_division_with_ints() {
    let python = r#"
def floor_divide(a: int, b: int) -> int:
    return a // b
"#;
    transpile_and_check(python, &["/", "fn floor_divide"]);
}

// =============================================================================
// Type Conversion Tests (exercises direct_rules.rs and type_mapper.rs)
// =============================================================================

#[test]
fn test_int_to_float_conversion() {
    let python = r#"
def convert_number(n: int) -> float:
    return float(n)
"#;
    transpile_and_check(python, &["as f64", "fn convert_number"]);
}

#[test]
fn test_float_to_int_conversion() {
    let python = r#"
def truncate_number(n: float) -> int:
    return int(n)
"#;
    transpile_and_check(python, &["fn truncate_number"]);
}

// =============================================================================
// List Operation Tests (exercises codegen.rs and rust_gen.rs)
// =============================================================================

#[test]
fn test_list_append() {
    let python = r#"
def add_item(items: list[int], item: int) -> list[int]:
    items.append(item)
    return items
"#;
    transpile_and_check(python, &["push", "Vec"]);
}

#[test]
fn test_list_length() {
    let python = r#"
def count_items(items: list[int]) -> int:
    return len(items)
"#;
    transpile_and_check(python, &[".len()", "fn count_items"]);
}

// =============================================================================
// Conditional Logic Tests (exercises ast_bridge.rs control flow)
// =============================================================================

#[test]
fn test_if_else_chain() {
    let python = r#"
def classify(n: int) -> str:
    if n > 0:
        return "positive"
    elif n < 0:
        return "negative"
    else:
        return "zero"
"#;
    transpile_and_check(python, &["if", "else", "positive", "negative", "zero"]);
}

// =============================================================================
// Loop Tests (exercises ast_bridge.rs loop conversions)
// =============================================================================

#[test]
fn test_for_loop_range() {
    let python = r#"
def sum_range(n: int) -> int:
    total: int = 0
    for i in range(n):
        total = total + i
    return total
"#;
    transpile_and_check(python, &["for", "fn sum_range"]);
}

#[test]
fn test_while_loop() {
    let python = r#"
def countdown(n: int) -> int:
    while n > 0:
        n = n - 1
    return n
"#;
    transpile_and_check(python, &["while", "fn countdown"]);
}

// =============================================================================
// Comparison Operator Tests (exercises operator conversion)
// =============================================================================

#[test]
fn test_comparison_operators() {
    let python = r#"
def compare_numbers(a: int, b: int) -> bool:
    return a < b and a <= b and a == b and a != b and a > b and a >= b
"#;
    transpile_and_check(python, &["<", "<=", "==", "!=", ">", ">="]);
}

// =============================================================================
// Boolean Logic Tests (exercises logical operator conversion)
// =============================================================================

#[test]
fn test_boolean_and_or() {
    let python = r#"
def check_condition(a: bool, b: bool, c: bool) -> bool:
    return (a and b) or c
"#;
    transpile_and_check(python, &["&&", "fn check_condition"]);
}

#[test]
fn test_boolean_not() {
    let python = r#"
def negate(value: bool) -> bool:
    return not value
"#;
    transpile_and_check(python, &["!", "fn negate"]);
}
