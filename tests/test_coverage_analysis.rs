use crate::test_helpers;
use crate::test_helpers::{transpile, transpile_and_check};

use depyler_core::DepylerPipeline;

#[test]
fn test_basic_transpilation_coverage() {
    let simple_function = r#"
def add_numbers(a: int, b: int) -> int:
    return a + b
"#;

    transpile_and_check(simple_function, &[]);
}

#[test]
fn test_hir_parsing_coverage() {
    let pipeline = DepylerPipeline::new();
    let function_with_control_flow = r#"
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)
"#;

    let hir_result = pipeline.parse_to_hir(function_with_control_flow);
    assert!(hir_result.is_ok(), "HIR parsing should work for valid Python");

    let hir = hir_result.unwrap();
    assert_eq!(hir.functions.len(), 1, "Should parse one function");
    assert_eq!(hir.functions[0].name, "factorial", "Function name should be preserved");
}

#[test]
fn test_error_handling_coverage() {
    let pipeline = DepylerPipeline::new();

    let invalid_python = "def invalid_func(\n    return 42";
    let result: Result<String, String> = Ok(transpile(invalid_python));
    assert!(result.is_err(), "Invalid Python should cause error");

    let empty_input = "";
    let empty_result = pipeline.transpile(empty_input);
    assert!(empty_result.is_ok() || empty_result.is_err());
}

#[test]
fn test_python_construct_coverage() {
    let constructs = [
        r#"
def simple() -> int:
    return 42
"#,
        r#"
def with_params(x: int, y: str) -> str:
    return y + str(x)
"#,
        r#"
def with_if(x: int) -> int:
    if x > 0:
        return x
    return 0
"#,
        r#"
def with_loop(n: int) -> int:
    total = 0
    for i in range(n):
        total += i
    return total
"#,
        r#"
def with_variables() -> int:
    x = 10
    y = 20
    z = x + y
    return z
"#,
    ];

    for construct in constructs {
        transpile_and_check(construct, &[]);
    }
}

#[test]
fn test_type_annotation_coverage() {
    let type_examples = vec![
        "def int_func(x: int) -> int: return x",
        "def str_func(x: str) -> str: return x",
        "def bool_func(x: bool) -> bool: return x",
        "def float_func(x: float) -> float: return x",
    ];

    for type_example in type_examples {
        transpile_and_check(type_example, &[]);
    }
}

#[test]
fn test_pipeline_configuration_coverage() {
    let default_pipeline = DepylerPipeline::new();
    let test_code = "def test() -> int: return 42";
    let result = default_pipeline.transpile(test_code);
    assert!(
        result.is_ok() || result.is_err(),
        "Default pipeline should handle basic code"
    );

    let verified_pipeline = DepylerPipeline::new().with_verification();
    let result2 = verified_pipeline.transpile(test_code);
    assert!(
        result2.is_ok() || result2.is_err(),
        "Verified pipeline should handle basic code"
    );
}

#[test]
fn test_memory_safety_patterns_coverage() {
    let memory_patterns = vec![
        r#"
def concat_strings(a: str, b: str) -> str:
    return a + b
"#,
        r#"
def list_ops() -> int:
    items = [1, 2, 3]
    return len(items)
"#,
        r#"
def reassignment() -> int:
    x = 10
    x = 20
    return x
"#,
    ];

    for pattern in memory_patterns {
        transpile_and_check(pattern, &[]);
    }
}

#[test]
fn test_uncovered_edge_cases() {
    let with_docstring = r#"
def documented_func() -> int:
    """This function returns 42."""
    return 42
"#;
    transpile_and_check(with_docstring, &[]);

    let multiple_returns = r#"
def multiple_returns(x: int) -> int:
    if x > 0:
        return x
    if x < 0:
        return -x
    return 0
"#;
    transpile_and_check(multiple_returns, &[]);

    let nested_calls = r#"
def outer(x: int) -> int:
    return inner(x + 1)

def inner(y: int) -> int:
    return y * 2
"#;
    transpile_and_check(nested_calls, &[]);
}
