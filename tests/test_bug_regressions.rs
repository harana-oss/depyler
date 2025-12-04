use crate::test_helpers::transpile;

// ============================================================================
// FINAL TYPE ANNOTATION TESTS - Python typing.Final to Rust const
// ============================================================================

#[test]
fn test_final_int_constant() {
    let python_code = r#"
from typing import Final

def get_field_goal() -> int:
    FIELD_GOAL: Final[int] = 600
    return FIELD_GOAL
"#;

    let rust_code = transpile(python_code);
    println!("Generated Rust code:\n{}", rust_code);

    // Check that 'const' is used instead of 'let'
    assert!(
        rust_code.contains("const FIELD_GOAL"),
        "Should generate 'const FIELD_GOAL', but got:\n{}",
        rust_code
    );

    // Check that the type annotation is i32 (not Final<i32>)
    assert!(
        rust_code.contains(": i32"),
        "Should have type annotation ': i32', but got:\n{}",
        rust_code
    );

    // Check that the value is 600
    assert!(
        rust_code.contains("= 600"),
        "Should have value '= 600', but got:\n{}",
        rust_code
    );

    // Ensure 'let' is not used for this constant
    assert!(
        !rust_code.contains("let FIELD_GOAL"),
        "Should NOT use 'let' for Final annotation, but got:\n{}",
        rust_code
    );
}

// ============================================================================
// ORIGINAL BUG REGRESSION TESTS
// ============================================================================

use std::process::Command;

#[test]
fn test_variable_shadowing_in_loops_bug() {
    // Bug: Variable assignments inside loops create new variables instead of updating existing ones
    let python_code = r#"
def factorial(n: int) -> int:
    result = 1
    for i in range(1, n + 1):
        result = result * i
    return result
"#;

    let rust_code = transpile(python_code);

    // Should contain proper range syntax (1..n + 1), not just "for i in 1" without the range
    assert!(
        rust_code.contains("for i in 1.."),
        "Loop should use proper range syntax with '..' operator"
    );

    // Should contain proper assignment without let keyword
    let lines: Vec<&str> = rust_code.lines().collect();
    let mut found_assignment = false;
    let mut found_shadowing = false;

    for line in lines {
        // Check for assignment like "result = result * i" (with or without parentheses)
        if (line.trim().contains("result = result * i") || line.trim().contains("result = (result * i)"))
            && !line.trim().starts_with("let mut")
        {
            found_assignment = true;
        }
        if line.trim().starts_with("let mut result = result * i")
            || line.trim().starts_with("let mut result = (result * i)")
        {
            found_shadowing = true;
        }
    }

    assert!(found_assignment, "Should have proper assignment: result = result * i");
    assert!(!found_shadowing, "Should not shadow variable with let mut inside loop");
}

#[test]
fn test_binary_search_variable_shadowing_bug() {
    // Bug: Variable assignments in conditionals create new variables
    let python_code = r#"
def binary_search(arr: list, target: int) -> int:
    left = 0
    right = len(arr) - 1
    while left <= right:
        mid = (left + right) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;

    let rust_code = transpile(python_code);

    // Should NOT contain "let mut left =" or "let mut right =" inside conditionals
    let lines: Vec<&str> = rust_code.lines().collect();
    let mut inside_conditional = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.contains("if") || trimmed.contains("else") {
            inside_conditional = true;
        }
        if inside_conditional && (trimmed.starts_with("let mut left =") || trimmed.starts_with("let mut right =")) {
            panic!("Found variable shadowing inside conditional: {}", trimmed);
        }
        if trimmed == "}" {
            inside_conditional = false;
        }
    }
}

#[test]
fn test_docstring_as_expression_bug() {
    // Bug: Docstrings are treated as expressions that return strings
    let python_code = r#"
def add(a: int, b: int) -> int:
    "Add two numbers"
    return a + b
"#;

    let rust_code = transpile(python_code);

    // Should NOT contain docstring as an expression like '"Add two numbers" . to_string ();'
    assert!(
        !rust_code.contains(r#""Add two numbers" . to_string ()"#),
        "Docstring should not be generated as a string expression"
    );

    // Should contain proper Rust doc comment
    assert!(
        rust_code.contains("/// Add two numbers") || rust_code.contains("#[doc"),
        "Should generate proper Rust documentation"
    );
}

#[test]
fn test_array_length_underflow_bug() {
    // Bug: len(arr) - 1 can underflow if array is empty
    let python_code = r#"
def process_empty_list(arr: list) -> int:
    if len(arr) == 0:
        return -1
    right = len(arr) - 1
    return right
"#;

    let rust_code = transpile(python_code);

    // Should handle empty array case properly
    assert!(
        rust_code.contains("len()") || rust_code.contains(".is_empty()"),
        "Should use proper length checking"
    );

    // Should not cause underflow
    assert!(
        !rust_code.contains("arr . len () - 1") || rust_code.contains("saturating_sub"),
        "Should prevent integer underflow in array length calculation"
    );
}

#[test]
fn test_integer_division_bug() {
    // Bug: Python // operator should map to proper integer division
    let python_code = r#"
def divide(a: int, b: int) -> int:
    return a // b
"#;

    let rust_code = transpile(python_code);

    // Should use proper integer division, not float division
    assert!(
        rust_code.contains("/") && !rust_code.contains("/ 2.0"),
        "Should generate integer division for // operator"
    );
}

#[test]
fn test_list_indexing_bounds_checking_bug() {
    // Bug: Array indexing should have proper bounds checking
    let python_code = r#"
def get_item(arr: list, index: int) -> int:
    return arr[index]
"#;

    let rust_code = transpile(python_code);

    // Should use safe indexing methods
    assert!(
        rust_code.contains(".get(") || rust_code.contains("bounds_check"),
        "Should use safe array indexing"
    );
}

#[test]
fn test_string_concatenation_efficiency_bug() {
    // Bug: String operations should be efficient
    let python_code = r#"
def concat_strings(a: str, b: str) -> str:
    return a + b
"#;

    let rust_code = transpile(python_code);

    // Should use efficient string operations
    assert!(
        rust_code.contains("format!") || rust_code.contains("String::from") || rust_code.contains("&str"),
        "Should generate efficient string operations"
    );
}

#[test]
fn test_type_inference_ownership_bug() {
    // Bug: Type inference should generate correct ownership patterns
    let python_code = r#"
def process_data(data: list) -> list:
    result = []
    for item in data:
        result.append(item * 2)
    return result
"#;

    let rust_code = transpile(python_code);

    // Should have proper ownership and borrowing
    assert!(
        rust_code.contains("Vec<") && rust_code.contains("push"),
        "Should generate proper Vec operations"
    );

    // Should not have unnecessary clones or copies
    assert!(
        !rust_code.contains(".clone().clone()"),
        "Should not generate redundant clones"
    );
}

#[test]
fn test_docstring_generation_bug() {
    // Bug: Docstrings are converted to expressions instead of comments
    let python_code = r#"
def example_function(x: int) -> int:
    """This is a docstring that should become a comment"""
    return x * 2
"#;

    let rust_code = transpile(python_code);

    // Should NOT contain docstring as executable expression
    assert!(
        !rust_code.contains(r#""This is a docstring that should become a comment" . to_string ()"#),
        "Docstring should not be an executable expression"
    );

    // Should contain proper Rust documentation comment
    assert!(
        rust_code.contains("/// This is a docstring") || rust_code.contains("#[doc = \"This is a docstring"),
        "Should generate proper Rust documentation"
    );
}

#[test]
fn test_integer_division_operator_bug() {
    // Bug: Python // operator should map to integer division, not float division
    let python_code = r#"
def floor_divide(a: int, b: int) -> int:
    return a // b
"#;

    let rust_code = transpile(python_code);

    // Should use integer division, not float division
    // In Rust, integer division with / already truncates toward zero for integers
    // So (a / b) is correct for positive integers, but we should be explicit
    assert!(rust_code.contains("/ b"), "Should contain division operation");

    // Should not generate float division
    assert!(
        !rust_code.contains("/ b as f"),
        "Should not cast to float for integer division"
    );
}

#[test]
fn test_multiplication_optimization_bug() {
    // Bug: x * 2 should not become bit shift unless safe
    let python_code = r#"
def multiply_by_two(x: int) -> int:
    return x * 2
"#;

    let rust_code = transpile(python_code);

    // Should either use multiplication or clearly documented bit shift
    // Bit shift is an optimization but can behave differently for negative numbers
    if rust_code.contains("<<") {
        // If using bit shift, should handle negative numbers correctly
        // or document the optimization
        assert!(
            rust_code.contains("// Optimized") || rust_code.contains("wrapping_shl"),
            "Bit shift optimization should be documented or handle edge cases"
        );
    } else {
        assert!(
            rust_code.contains("* 2") || rust_code.contains("*2"),
            "Should use multiplication if not optimizing"
        );
    }
}

#[test]
fn test_plain_list_type_mapping_bug() {
    // Bug: Plain 'list' type should map to Vec<T> not literal 'list'
    let python_code = r#"
def process_items(items: list) -> list:
    return items
"#;

    let rust_code = transpile(python_code);

    // Should NOT contain literal 'list' type
    assert!(
        !rust_code.contains("items : list"),
        "Should not generate literal 'list' type"
    );

    // Should contain proper Vec type
    assert!(
        rust_code.contains("Vec<") || rust_code.contains("&["),
        "Should generate Vec<T> or slice type for list"
    );
}

#[test]
fn test_integer_underflow_array_length_bug() {
    // Bug: arr.len() - 1 can underflow if array is empty
    let python_code = r#"
def get_last_index(arr: list) -> int:
    return len(arr) - 1
"#;

    let rust_code = transpile(python_code);

    // Should use saturating arithmetic or proper bounds checking
    assert!(
        rust_code.contains("saturating_sub")
            || rust_code.contains("checked_sub")
            || rust_code.contains("if") && rust_code.contains("is_empty"),
        "Should handle potential underflow in array length operations"
    );
}

#[test]
fn test_method_call_spacing_bug() {
    // Bug: Method calls have unnecessary spaces
    let python_code = r#"
def get_length(arr: list) -> int:
    return len(arr)
"#;

    let rust_code = transpile(python_code);

    // Should NOT have spaces in method calls
    assert!(
        !rust_code.contains("arr . len ()"),
        "Method calls should not have spaces"
    );

    // Should have proper method call syntax
    assert!(
        rust_code.contains("arr.len()") || rust_code.contains("len(arr)"),
        "Should generate proper method call syntax"
    );
}

#[test]
fn test_bounds_checking_array_indexing_bug() {
    // Bug: Array indexing should have bounds checking
    let python_code = r#"
def get_item(arr: list, i: int) -> int:
    return arr[i]
"#;

    let rust_code = transpile(python_code);

    // Should use safe indexing methods
    assert!(
        rust_code.contains(".get(") || rust_code.contains("bounds_check") || rust_code.contains("unwrap_or"),
        "Should use safe array indexing"
    );
}

#[test]
fn test_consistent_type_mapping() {
    // Bug: Type mapping should be consistent across contexts
    let python_code = r#"
def create_list() -> list:
    items = []
    return items
"#;

    let rust_code = transpile(python_code);

    // Return type and variable type should be consistent
    let vec_count = rust_code.matches("Vec<").count();
    let list_count = rust_code.matches(" list").count();

    // Should prefer Vec over literal 'list'
    assert!(
        vec_count >= list_count,
        "Should consistently use Vec<T> over literal 'list' type"
    );
}

#[test]
fn test_rust_code_formatting_consistency() {
    // Bug: Generated Rust code has inconsistent spacing and formatting issues
    let python_code = r#"
from typing import List

def calculate_sum(numbers: List[int]) -> int:
    """Calculate the sum of a list of integers."""
    total: int = 0
    for n in numbers:
        total += n
    return total
"#;

    let rust_code = transpile(python_code);

    // Check proper generic spacing
    assert!(
        rust_code.contains("Vec<i32>"),
        "Expected proper generic spacing 'Vec<i32>', got: {}",
        rust_code
    );

    // Check proper assignment spacing - should NOT have "=("
    assert!(
        !rust_code.contains("=("),
        "Found improper assignment spacing '=(', should be ' = (': {}",
        rust_code
    );

    // Check proper operator spacing
    assert!(
        rust_code.contains(" = "),
        "Expected proper assignment operator spacing ' = ': {}",
        rust_code
    );

    // Check method call spacing - should NOT have spaces in method calls
    assert!(
        !rust_code.contains(". "),
        "Found spaces in method calls, should be '.method()': {}",
        rust_code
    );
}

// ============================================================================
//
// ============================================================================
// BUG: Array literal assignments are being DROPPED during code generation.
// ALL variable assignments are missing, leaving only return statements with
// undefined variables. This is a SHOWSTOPPER bug.
//
// EXTREME TDD: These tests MUST fail first, proving the bug exists.
// ============================================================================

#[test]
fn test_simple_array_literal_missing_assignment() {
    //
    let python_code = r#"
def test_array():
    arr = [1, 2, 3]
    return arr
"#;

    let rust_code = transpile(python_code);

    // CRITICAL ASSERTION: Generated code MUST include the assignment
    assert!(
        rust_code.contains("arr =") || rust_code.contains("let arr"),
        "BUG CONFIRMED: Array assignment is missing!\nGenerated code:\n{}",
        rust_code
    );

    // CRITICAL: Must contain array initialization, not just undefined variable
    assert!(
        rust_code.contains("[1") || rust_code.contains("vec!"),
        "BUG CONFIRMED: Array literal is missing!\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_multiple_array_assignments_dropped() {
    //
    let python_code = r#"
def test_arrays():
    arr1 = [1, 2, 3]
    arr2 = [4, 5, 6]
    arr3 = [7, 8, 9]
    return arr1, arr2, arr3
"#;

    let rust_code = transpile(python_code);

    // Check for assignments (not just variable names in return statement)
    let has_arr1_assign = rust_code.contains("arr1 =") || rust_code.contains("let arr1");
    let has_arr2_assign = rust_code.contains("arr2 =") || rust_code.contains("let arr2");
    let has_arr3_assign = rust_code.contains("arr3 =") || rust_code.contains("let arr3");

    assert!(
        has_arr1_assign,
        "BUG CONFIRMED: arr1 assignment is missing!\nGenerated code:\n{}",
        rust_code
    );
    assert!(
        has_arr2_assign,
        "BUG CONFIRMED: arr2 assignment is missing!\nGenerated code:\n{}",
        rust_code
    );
    assert!(
        has_arr3_assign,
        "BUG CONFIRMED: arr3 assignment is missing!\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_boolean_array_assignment_dropped() {
    //
    let python_code = r#"
def test_bool_array():
    flags = [True, False, True]
    return flags
"#;

    let rust_code = transpile(python_code);

    assert!(
        rust_code.contains("flags =") || rust_code.contains("let flags"),
        "BUG CONFIRMED: Boolean array assignment is missing!\nGenerated code:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("true") || rust_code.contains("false"),
        "BUG CONFIRMED: Boolean literals are missing!\nGenerated code:\n{}",
        rust_code
    );
}

// ============================================================================
//
// ============================================================================
// BUG: copy.copy() for lists generates `.copy()` method call which doesn't
// exist in Rust. Should generate `.clone()` or proper copy semantics.
//
// DISCOVERED: TDD Book validation (copy module)
// SEVERITY: P1 MAJOR - affects fundamental stdlib function
// ============================================================================

#[test]
fn test_copy_copy_list_invalid_codegen() {
    //
    let python_code = r#"
import copy

def test_shallow_copy() -> int:
    original = [1, 2, 3]
    copied = copy.copy(original)
    copied.append(4)
    return len(original)
"#;

    let rust_code = transpile(python_code);

    // CRITICAL: Should NOT generate `.copy()` method (doesn't exist in Rust)
    assert!(
        !rust_code.contains(".copy()"),
        "BUG CONFIRMED: Generated invalid `.copy()` method!\nGenerated code:\n{}",
        rust_code
    );

    // Should generate valid Rust code with .clone() or proper copy semantics
    assert!(
        rust_code.contains(".clone()") || rust_code.contains("copy::copy"),
        "Should generate valid Rust copy operation (.clone() or copy::copy)\nGenerated code:\n{}",
        rust_code
    );

    // Verify the copy semantics are correct (shallow copy behavior)
    assert!(
        rust_code.contains("copied") || rust_code.contains("let"),
        "Should have proper variable assignment for copied list\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_copy_copy_dict_works() {
    //
    let python_code = r#"
import copy

def test_dict_copy() -> int:
    original = {"a": 1, "b": 2}
    copied = copy.copy(original)
    copied["c"] = 3
    return len(original)
"#;

    let rust_code = transpile(python_code);

    // Should generate valid Rust code for dict copy
    assert!(
        rust_code.contains(".clone()") || rust_code.contains("copy"),
        "Should generate valid dict copy operation\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_copy_deepcopy_list_works() {
    //
    let python_code = r#"
import copy

def test_deep_copy() -> int:
    original = [[1, 2], [3, 4]]
    copied = copy.deepcopy(original)
    copied[0].append(5)
    return len(original[0])
"#;

    let rust_code = transpile(python_code);

    // Should generate valid deep copy operation
    assert!(
        rust_code.contains("clone") || rust_code.contains("deep"),
        "Should generate valid deep copy operation\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_untyped_list_parameter_compiles() {
    let python_code = r#"
def sum_list(numbers: list) -> int:
    total = 0
    for num in numbers:
        total = total + num
    return total
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("fn sum_list"));
    assert!(!rust_code.contains("DynamicType"));
}

#[test]
fn test_untyped_dict_parameter_compiles() {
    let python_code = r#"
def get_value(data: dict, key: str) -> int:
    return data.get(key, 0)
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("fn get_value"));
    assert!(!rust_code.contains("DynamicType"));
}
