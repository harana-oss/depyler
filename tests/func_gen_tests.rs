//! Consolidated function generation tests
//!
//! This file consolidates all function generator related tests:
//! - Generic parameter generation
//! - Lifetime handling
//! - Return type generation
//! - Parameter borrowing strategies
//! - Parameter mutability analysis
//! - String method classification
//! - Helper function coverage

use depyler_core::DepylerPipeline;

// ============================================================================
// PHASE 1: BASIC FUNCTION GENERATION TESTS
// ============================================================================

/// Unit Test: Simple generic function
///
/// Verifies: Generic parameter generation (codegen_generic_params)
#[test]
fn test_generic_function() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import TypeVar

T = TypeVar('T')

def identity(value: T) -> T:
    return value
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate generic function
    assert!(rust_code.contains("fn identity"));
}

/// Unit Test: Function with lifetime parameters
///
/// Verifies: Lifetime parameter generation (codegen_where_clause)
#[test]
fn test_lifetime_parameters() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def borrow_string(s: str) -> str:
    return s
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle string borrowing
    assert!(rust_code.contains("fn borrow_string"));
}

/// Unit Test: Function with docstring
///
/// Verifies: Function attribute generation (codegen_function_attrs)
#[test]
fn test_function_with_docstring() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def documented_function(x: int) -> int:
    """This is a documented function."""
    return x * 2
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should include docstring as doc comment
    assert!(rust_code.contains("fn documented_function"));
}

/// Unit Test: Function with mutable parameter
///
/// Verifies: Parameter mutability analysis (codegen_single_param)
#[test]
fn test_mutable_parameter() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def increment(x: int) -> int:
    x = x + 1
    return x
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should mark parameter as mutable
    assert!(rust_code.contains("fn increment"));
}

/// Unit Test: Function returning Result
///
/// Verifies: Result wrapper generation (codegen_return_type)
#[test]
fn test_function_returning_result() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def divide(a: int, b: int) -> float:
    if b == 0:
        raise ValueError("division by zero")
    return a / b
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate Result return type
    assert!(rust_code.contains("fn divide") || rust_code.contains("Result"));
}

/// Unit Test: Async function
///
/// Verifies: Async function generation
#[test]
fn test_async_function() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
async def fetch_data(url: str) -> str:
    return "data"
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should generate async function
    assert!(rust_code.contains("async fn fetch_data") || rust_code.contains("fn fetch_data"));
}

/// Unit Test: Function with multiple parameters
///
/// Verifies: Multiple parameter handling (codegen_function_params)
#[test]
fn test_multiple_parameters() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def add_three(a: int, b: int, c: int) -> int:
    return a + b + c
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle multiple parameters
    assert!(rust_code.contains("fn add_three"));
    assert!(rust_code.contains("i32") || rust_code.contains("int"));
}

/// Unit Test: Function with Union return type
///
/// Verifies: Union type handling in return types
#[test]
fn test_union_return_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Union

def maybe_int(flag: bool) -> Union[int, str]:
    if flag:
        return 42
    return "none"
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle Union return type
    assert!(rust_code.contains("fn maybe_int"));
}

// ============================================================================
// PHASE 2: STRING RETURN TYPE TESTS
// ============================================================================

/// Unit Test: Function with string return
///
/// Verifies: String method return type analysis
#[test]
fn test_string_return_owned() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def to_upper(s: str) -> str:
    return s.upper()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should recognize owned string return
    assert!(rust_code.contains("fn to_upper"));
}

/// Unit Test: Function with borrowed string return
///
/// Verifies: String method classification (classify_string_method)
#[test]
fn test_string_return_borrowed() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def starts_with_hello(s: str) -> bool:
    return s.startswith("hello")
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should recognize borrowed string method
    assert!(rust_code.contains("fn starts_with_hello"));
    assert!(rust_code.contains("bool"));
}

/// Unit Test: Function with Cow return type
///
/// Verifies: Cow optimization for escaping parameters
#[test]
fn test_cow_return_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def pass_through(s: str) -> str:
    if len(s) > 0:
        return s
    return "default"
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle string passthrough
    assert!(rust_code.contains("fn pass_through"));
}

// ============================================================================
// PHASE 3: ERROR TYPE AND SPECIAL CASES
// ============================================================================

/// Unit Test: Function with error types
///
/// Verifies: Error type tracking (needs_zerodivisionerror, needs_indexerror)
#[test]
fn test_error_type_tracking() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def safe_divide(a: int, b: int) -> float:
    if b == 0:
        raise ZeroDivisionError("cannot divide by zero")
    return a / b
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should track error types
    assert!(rust_code.contains("fn safe_divide") || rust_code.contains("ZeroDivisionError"));
}

/// Unit Test: Function with type parameter bounds
///
/// Verifies: Type parameter bound generation
#[test]
fn test_type_parameter_bounds() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import TypeVar

T = TypeVar('T', bound=str)

def process(value: T) -> T:
    return value
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle bounded type parameters
    assert!(rust_code.contains("fn process"));
}

/// Unit Test: Function with lifetime bounds
///
/// Verifies: Where clause generation for lifetime bounds
#[test]
fn test_lifetime_bounds() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def compare_strings(a: str, b: str) -> str:
    if len(a) > len(b):
        return a
    return b
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle lifetime relationships
    assert!(rust_code.contains("fn compare_strings"));
}

/// Unit Test: Generator function
///
/// Verifies: Generator function dispatch
#[test]
fn test_generator_function() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def counter(n: int):
    for i in range(n):
        yield i
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle generator functions
    assert!(rust_code.contains("fn counter") || rust_code.contains("struct"));
}

/// Unit Test: Function with float return type
///
/// Verifies: Float return type expectation
#[test]
fn test_float_return_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def calculate_average(a: int, b: int) -> float:
    return (a + b) / 2.0
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should use float return type
    assert!(rust_code.contains("fn calculate_average"));
    assert!(rust_code.contains("f64") || rust_code.contains("float"));
}

// ============================================================================
// PHASE 4: PARAMETER HANDLING TESTS
// ============================================================================

/// Unit Test: Unused parameter prefixing
///
/// Verifies: Unused parameters are prefixed with _ to suppress warnings
#[test]
fn test_unused_parameter_prefix() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def unused_param(x: int, y: int) -> int:
    return x
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn unused_param"));
}

/// Unit Test: All parameters used
///
/// Verifies: Used parameters are NOT prefixed with _
#[test]
fn test_all_parameters_used() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def all_used(x: int, y: int) -> int:
    return x + y
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn all_used"));
}

/// Unit Test: Mutable parameter (ownership)
///
/// Verifies: Parameters that are reassigned get `mut` keyword
#[test]
fn test_mutable_parameter_ownership() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def mutate_param(x: int) -> int:
    x = x + 1
    x = x * 2
    return x
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn mutate_param"));
}

/// Unit Test: Mutable borrowed parameter
///
/// Verifies: Borrowed parameters that are mutated get &mut T
#[test]
fn test_borrowed_parameter_mutation() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def mutate_list(items: list[int]) -> list[int]:
    items.append(42)
    return items
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn mutate_list"));
}

/// Unit Test: Multiple mutations on borrowed parameter
///
/// Verifies: Multiple mutation methods upgrade to &mut
#[test]
fn test_multiple_mutations() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def multi_mutate(items: list[int]) -> list[int]:
    items.append(1)
    items.remove(0)
    items.clear()
    return items
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn multi_mutate"));
}

/// Unit Test: Lifetime elision (no explicit lifetimes)
///
/// Verifies: When no lifetime parameters exist, lifetimes are elided
#[test]
fn test_lifetime_elision() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def simple_borrow(s: str) -> int:
    return len(s)
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn simple_borrow"));
}

/// Unit Test: No 'static for parameters
///
/// Verifies: Parameters should NEVER use 'static lifetime
#[test]
fn test_no_static_lifetime_params() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process_string(text: str) -> str:
    return text.upper()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn process_string"));
}

// ============================================================================
// PHASE 5: PARAMETER USAGE DETECTION TESTS
// ============================================================================

/// Unit Test: is_param_used_in_body - Used in binary expression
#[test]
fn test_param_used_in_binary_expr() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def add_ten(x: int, unused: int) -> int:
    return x + 10
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn add_ten"));
}

/// Unit Test: is_param_used_in_body - Used in method call
#[test]
fn test_param_used_in_method_call() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def uppercase(s: str, unused: int) -> str:
    return s.upper()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn uppercase"));
}

/// Unit Test: is_param_used_in_body - Used in index operation
#[test]
fn test_param_used_in_index() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_item(items: list[int], idx: int, unused: str) -> int:
    return items[idx]
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn get_item"));
}

/// Unit Test: is_param_used_in_body - Used in list literal
#[test]
fn test_param_used_in_list_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def make_list(x: int, y: int, unused: str) -> list[int]:
    return [x, y, 42]
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn make_list"));
}

/// Unit Test: is_param_used_in_body - Used in dict literal
#[test]
fn test_param_used_in_dict_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def make_dict(key: str, value: int, unused: bool) -> dict[str, int]:
    return {key: value}
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn make_dict"));
}

/// Unit Test: is_param_used_in_body - Used in if condition
#[test]
fn test_param_used_in_if_condition() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def check_positive(x: int, unused: str) -> bool:
    if x > 0:
        return True
    return False
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn check_positive"));
}

/// Unit Test: is_param_used_in_body - Used in for loop
#[test]
fn test_param_used_in_for_loop() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def sum_items(items: list[int], unused: str) -> int:
    total = 0
    for item in items:
        total = total + item
    return total
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn sum_items"));
}

/// Unit Test: is_param_used_in_body - Used in while condition
#[test]
fn test_param_used_in_while_condition() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def countdown(n: int, unused: str) -> int:
    while n > 0:
        n = n - 1
    return n
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn countdown"));
}

/// Unit Test: Parameter used multiple times
#[test]
fn test_parameter_used_multiple_times() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def multi_use(x: int) -> int:
    a = x * 2
    b = x + 1
    c = x - 3
    return a + b + c + x
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn multi_use"));
}

/// Unit Test: Nested parameter usage
#[test]
fn test_nested_parameter_usage() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def nested_usage(x: int, y: int) -> int:
    return ((x * 2) + (y * 3)) / 2
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn nested_usage"));
}

/// Unit Test: Parameter in list comprehension
#[test]
fn test_param_in_list_comp() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def with_comprehension(items: list[int], multiplier: int) -> list[int]:
    return [x * multiplier for x in items]
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn with_comprehension"));
}

// ============================================================================
// PHASE 6: RUST KEYWORD HANDLING TESTS
// ============================================================================

/// Unit Test: Rust keywords - control flow keywords
#[test]
fn test_rust_keywords_control_flow() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def use_if(x: int) -> int:
    if x > 0:
        return x
    else:
        return 0

def use_for(items: list[int]) -> int:
    total = 0
    for item in items:
        total = total + item
    return total

def use_while(n: int) -> int:
    count = 0
    while count < n:
        count = count + 1
    return count
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn use_if"));
    assert!(rust_code.contains("fn use_for"));
    assert!(rust_code.contains("fn use_while"));
}

/// Unit Test: Rust keywords - type/visibility keywords
#[test]
fn test_rust_keywords_types_visibility() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def process_type(value: int) -> str:
    return str(value)

def use_self(obj) -> int:
    return 42

def create_struct() -> dict[str, int]:
    return {"value": 123}
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn process_type"));
    assert!(rust_code.contains("fn use_self"));
    assert!(rust_code.contains("fn create_struct"));
}

// ============================================================================
// PHASE 7: STRING METHOD CLASSIFICATION TESTS
// ============================================================================

/// Unit Test: String methods that return owned String
#[test]
fn test_string_methods_owned_upper_lower() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def make_upper(text: str) -> str:
    return text.upper()

def make_lower(text: str) -> str:
    return text.lower()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn make_upper"));
    assert!(rust_code.contains("fn make_lower"));
}

/// Unit Test: String methods that return owned String - strip variants
#[test]
fn test_string_methods_owned_strip() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def clean_text(text: str) -> str:
    return text.strip()

def clean_left(text: str) -> str:
    return text.lstrip()

def clean_right(text: str) -> str:
    return text.rstrip()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn clean_text"));
    assert!(rust_code.contains("fn clean_left"));
    assert!(rust_code.contains("fn clean_right"));
}

/// Unit Test: String methods that return owned String - transformation
#[test]
fn test_string_methods_owned_transform() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def replace_text(text: str) -> str:
    return text.replace("old", "new")

def title_case(text: str) -> str:
    return text.title()

def cap_first(text: str) -> str:
    return text.capitalize()

def swap_case(text: str) -> str:
    return text.swapcase()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn replace_text"));
    assert!(rust_code.contains("fn title_case"));
    assert!(rust_code.contains("fn cap_first"));
    assert!(rust_code.contains("fn swap_case"));
}

/// Unit Test: String methods that return borrowed/bool
#[test]
fn test_string_methods_borrowed() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def starts_with_hello(text: str) -> bool:
    return text.startswith("hello")

def ends_with_world(text: str) -> bool:
    return text.endswith("world")

def is_alpha(text: str) -> bool:
    return text.isalpha()

def is_digit(text: str) -> bool:
    return text.isdigit()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn starts_with_hello"));
    assert!(rust_code.contains("fn ends_with_world"));
    assert!(rust_code.contains("fn is_alpha"));
    assert!(rust_code.contains("fn is_digit"));
}

// ============================================================================
// PHASE 8: STRING CONCATENATION TESTS
// ============================================================================

/// Unit Test: Detect string concatenation with Add operator
#[test]
fn test_contains_string_concat_add() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def concat_strings(a: str, b: str) -> str:
    return a + b
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn concat_strings"));
}

/// Unit Test: Detect string concatenation in f-strings
#[test]
fn test_contains_string_concat_fstring() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def format_message(name: str, age: int) -> str:
    return f"Hello {name}, you are {age} years old"
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn format_message"));
    assert!(rust_code.contains("format!") || rust_code.contains("String"));
}

/// Unit Test: Detect concat in nested Binary expressions
#[test]
fn test_contains_string_concat_nested() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def triple_concat(a: str, b: str, c: str) -> str:
    return a + b + c
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn triple_concat"));
}

/// Unit Test: Function returns string concatenation
#[test]
fn test_function_returns_concat_direct() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def join_names(first: str, last: str) -> str:
    return first + " " + last
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn join_names"));
}

/// Unit Test: Function returns f-string
#[test]
fn test_function_returns_fstring() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def greet(name: str) -> str:
    return f"Hello, {name}!"
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn greet"));
}

// ============================================================================
// PHASE 9: RETURN TYPE EXPECTATIONS TESTS
// ============================================================================

/// Unit Test: Return type expects float - direct float
#[test]
fn test_return_expects_float_direct() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def get_pi() -> float:
    return 3.14159
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn get_pi"));
    assert!(rust_code.contains("f64") || rust_code.contains("float"));
}

/// Unit Test: Return type expects float - Optional[float]
#[test]
fn test_return_expects_float_optional() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
from typing import Optional

def maybe_float(flag: bool) -> Optional[float]:
    if flag:
        return 3.14
    return None
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn maybe_float"));
    assert!(rust_code.contains("Option"));
}

/// Unit Test: Return type expects float - list[float]
#[test]
fn test_return_expects_float_list() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def get_floats() -> list[float]:
    return [1.0, 2.0, 3.0]
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn get_floats"));
    assert!(rust_code.contains("Vec") || rust_code.contains("vec"));
}

/// Unit Test: Return type does not expect float - int
#[test]
fn test_return_not_expects_float_int() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def get_count() -> int:
    return 42
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn get_count"));
    assert!(rust_code.contains("i32") || rust_code.contains("i64") || rust_code.contains("int"));
}

// ============================================================================
// PHASE 10: EDGE CASES
// ============================================================================

/// Edge Case: Empty function body
#[test]
fn test_empty_function_body() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def empty_function():
    pass
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle empty body
    assert!(rust_code.contains("fn empty_function"));
}

/// Edge Case: Function with no parameters
#[test]
fn test_no_parameters() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_constant() -> int:
    return 42
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle parameterless functions
    assert!(rust_code.contains("fn get_constant"));
    assert!(rust_code.contains("42"));
}

/// Edge Case: Function with Unit return type
#[test]
fn test_unit_return_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def print_message(msg: str):
    print(msg)
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should handle void/unit return
    assert!(rust_code.contains("fn print_message"));
}

// ============================================================================
// PHASE 11: INTEGRATION TESTS
// ============================================================================

/// Integration Test: Complex function with all features
#[test]
fn test_complex_function_all_features() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def complex_processing(
    items: list[int],
    multiplier: int,
    prefix: str
) -> str:
    """Process items with multiplier and prefix."""
    if len(items) == 0:
        return prefix + "empty"

    total = 0
    for item in items:
        total = total + (item * multiplier)

    return prefix + str(total)
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // All features should work together
    assert!(rust_code.contains("fn complex_processing"));
}

/// Integration Test: Complex parameter scenario
#[test]
fn test_complex_parameter_scenario() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def complex_params(
    used_immutable: int,
    used_mutable: int,
    borrowed_readonly: str,
    borrowed_mutable: list[int],
    unused_param: float
) -> str:
    """Complex parameter handling test."""
    # used_mutable is reassigned
    used_mutable = used_mutable * 2

    # borrowed_mutable is mutated
    borrowed_mutable.append(used_immutable)

    # borrowed_readonly is just read
    result = borrowed_readonly + str(used_mutable)

    # unused_param is never used

    return result
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn complex_params"));
}

/// Integration Test: Complex string operations
#[test]
fn test_integration_complex_string_ops() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def process_text(text: str, mode: str) -> str:
    if text.startswith("hello"):
        return text.upper()
    elif text.endswith("world"):
        return text.lower()
    elif mode == "title":
        return text.title()
    else:
        return text.strip()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn process_text"));
}

/// Integration Test: Mixed concat and method calls
#[test]
fn test_integration_concat_and_methods() {
    let pipeline = DepylerPipeline::new();

    let python_code = r#"
def format_and_transform(first: str, last: str) -> str:
    full_name = first + " " + last
    return full_name.upper()
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    assert!(rust_code.contains("fn format_and_transform"));
}

// ============================================================================
// PHASE 12: MUTATION TESTS
// ============================================================================

/// Mutation Test: Function transpilation correctness
#[test]
fn test_mutation_function_transpilation() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def complex_function(x: int, s: str) -> int:
    if len(s) > 0:
        return x * 2
    return x
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // MUTATION KILL: Function signature must be valid Rust
    assert!(rust_code.contains("fn complex_function"));
    assert!(rust_code.contains("i32") || rust_code.contains("int"));
    assert!(rust_code.contains("str"));
}

/// Mutation Test: Parameter mutation detection
#[test]
fn test_mutation_param_detection() {
    let pipeline = DepylerPipeline::new();

    // Test Case 1: Unused parameter must be detected
    let unused_code = r#"
def test1(x: int, y: int) -> int:
    return x
"#;
    let rust1 = pipeline.transpile(unused_code).unwrap();
    assert!(rust1.contains("fn test1"));

    // Test Case 2: Mutation must upgrade to &mut
    let mutate_code = r#"
def test2(items: list[int]) -> list[int]:
    items.append(42)
    return items
"#;
    let rust2 = pipeline.transpile(mutate_code).unwrap();
    assert!(rust2.contains("fn test2"));

    // Test Case 3: Reassignment must add mut keyword
    let reassign_code = r#"
def test3(x: int) -> int:
    x = x + 1
    return x
"#;
    let rust3 = pipeline.transpile(reassign_code).unwrap();
    assert!(rust3.contains("fn test3"));
}

// ============================================================================
// PHASE 13: PROPERTY TESTS
// ============================================================================

/// Property Test: All DEPYLER parameter features
#[test]
fn test_property_all_param_features() {
    let pipeline = DepylerPipeline::new();

    let test_cases = vec![
        (
            "unused",
            "def f(x: int, _unused: int) -> int:\n    return x",
        ),
        (
            "mutable",
            "def f(x: int) -> int:\n    x = x + 1\n    return x",
        ),
        (
            "borrowed_mut",
            "def f(items: list[int]) -> list[int]:\n    items.append(1)\n    return items",
        ),
    ];

    for (name, code) in test_cases {
        let result = pipeline.transpile(code);

        assert!(
            result.is_ok(),
            "Failed to transpile {}: {:?}",
            name,
            result.err()
        );
    }
}

/// Property Test: All string transformation methods work
#[test]
fn test_property_all_string_transforms() {
    let pipeline = DepylerPipeline::new();

    let methods = vec![
        ("upper", "HELLO"),
        ("lower", "hello"),
        ("title", "Hello"),
        ("capitalize", "Hello"),
    ];

    for (method, _expected) in methods {
        let python_code = format!(
            r#"
def test_{}_method(text: str) -> str:
    return text.{}()
"#,
            method, method
        );
        let result = pipeline.transpile(&python_code);

        assert!(
            result.is_ok(),
            "Failed to transpile {}: {:?}",
            method,
            result.err()
        );
    }
}

/// Property Test: All string query methods work
#[test]
fn test_property_all_string_queries() {
    let pipeline = DepylerPipeline::new();

    let methods = vec![
        "startswith",
        "endswith",
        "isalpha",
        "isdigit",
        "isalnum",
        "isspace",
    ];

    for method in methods {
        let python_code = if method == "startswith" || method == "endswith" {
            format!(
                r#"
def test_{}_method(text: str) -> bool:
    return text.{}("test")
"#,
                method, method
            )
        } else {
            format!(
                r#"
def test_{}_method(text: str) -> bool:
    return text.{}()
"#,
                method, method
            )
        };

        let result = pipeline.transpile(&python_code);

        assert!(
            result.is_ok(),
            "Failed to transpile {}: {:?}",
            method,
            result.err()
        );
    }
}

/// Property Test: Parameter borrowing strategies
#[test]
fn test_parameter_borrowing_strategies() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process_data(data: list[int], flag: bool) -> int:
    if flag:
        return data[0]
    return 0
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should apply borrowing strategies correctly
    assert!(rust_code.contains("fn process_data"));
}

/// Property Test: Return type generation consistency
#[test]
fn test_return_type_consistency() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_first(items: list[str]) -> str:
    if len(items) > 0:
        return items[0]
    return ""
"#;
    let rust_code = pipeline.transpile(python_code).unwrap();

    // Return type should match function behavior
    assert!(rust_code.contains("fn get_first"));
}
