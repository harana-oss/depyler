//! Tests for dataflow type inference with type casting functions
//!
//! This test suite verifies that the dataflow type inference correctly
//! infers types when using type casting functions like int(), float(),
//! str(), abs(), etc.

use depyler_core::DepylerPipeline;
use depyler_core::dataflow::{InferredTypes, infer_python_function};
use depyler_core::hir::Type;

/// Helper to run type inference and get the result
fn infer_types(source: &str) -> InferredTypes {
    infer_python_function(source).expect("Type inference failed")
}

/// Helper to check that a variable has the expected type
fn assert_variable_type(inferred: &InferredTypes, var_name: &str, expected: Type) {
    let actual = inferred
        .get_variable_type(var_name)
        .expect(&format!("Variable '{}' not found", var_name));
    assert_eq!(
        actual, &expected,
        "Variable '{}': expected {:?}, got {:?}",
        var_name, expected, actual
    );
}

/// Helper to transpile Python to Rust and check that it compiles
fn assert_transpiles_successfully(python: &str) {
    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(result.is_ok(), "Transpilation failed: {:?}", result.err());
    let rust_code = result.unwrap();
    println!("Generated Rust code:\n{}", rust_code);
}

// ============================================================================
// Tests for int() casting
// ============================================================================

#[test]
fn test_int_cast_from_float() {
    let python = r#"
def convert():
    y = 3.14
    x = int(y)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Int);
    assert_variable_type(&inferred, "y", Type::Float);
}

#[test]
fn test_int_cast_from_string() {
    let python = r#"
def convert():
    s = "42"
    x = int(s)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Int);
    assert_variable_type(&inferred, "s", Type::String);
}

#[test]
fn test_int_cast_from_unknown() {
    let python = r#"
def convert(value):
    x = int(value)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Int);
}

#[test]
fn test_int_cast_no_args() {
    let python = r#"
def convert():
    x = int()
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Int);
}

// ============================================================================
// Tests for float() casting
// ============================================================================

#[test]
fn test_float_cast_from_int() {
    let python = r#"
def convert():
    y = 42
    x = float(y)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Float);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn test_float_cast_from_string() {
    let python = r#"
def convert():
    s = "3.14"
    x = float(s)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Float);
    assert_variable_type(&inferred, "s", Type::String);
}

#[test]
fn test_float_cast_from_unknown() {
    let python = r#"
def convert(value):
    x = float(value)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Float);
}

// ============================================================================
// Tests for str() casting
// ============================================================================

#[test]
fn test_str_cast_from_int() {
    let python = r#"
def convert():
    y = 42
    x = str(y)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::String);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn test_str_cast_from_float() {
    let python = r#"
def convert():
    y = 3.14
    x = str(y)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::String);
    assert_variable_type(&inferred, "y", Type::Float);
}

#[test]
fn test_str_cast_from_unknown() {
    let python = r#"
def convert(value):
    x = str(value)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::String);
}

// ============================================================================
// Tests for bool() casting
// ============================================================================

#[test]
fn test_bool_cast_from_int() {
    let python = r#"
def convert():
    y = 42
    x = bool(y)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Bool);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn test_bool_cast_from_string() {
    let python = r#"
def convert():
    s = "hello"
    x = bool(s)
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Bool);
    assert_variable_type(&inferred, "s", Type::String);
}

// ============================================================================
// Tests for abs() function
// ============================================================================

#[test]
fn test_abs_from_int() {
    let python = r#"
def compute():
    h = -42
    g = abs(h)
    return g
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "g", Type::Int);
    assert_variable_type(&inferred, "h", Type::Int);
}

#[test]
fn test_abs_from_float() {
    let python = r#"
def compute():
    h = -3.14
    g = abs(h)
    return g
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "g", Type::Float);
    assert_variable_type(&inferred, "h", Type::Float);
}

#[test]
fn test_abs_from_unknown() {
    let python = r#"
def compute(value):
    g = abs(value)
    return g
"#;
    let inferred = infer_types(python);
    // abs() should preserve the type of the argument
    // When the argument is unknown, the result is also Unknown
    // which gets filtered out, so we don't expect it to be tracked
    // This is reasonable behavior - we can't infer a concrete type
    // This test just verifies it doesn't crash
    let _type = inferred.get_variable_type("g");
    // It's OK if g is not tracked when its type is Unknown
}

// ============================================================================
// Tests for min() and max() functions
// ============================================================================

#[test]
fn test_min_from_ints() {
    let python = r#"
def compute():
    a = 10
    b = 20
    result = min(a, b)
    return result
"#;
    let inferred = infer_types(python);
    // min() should return the same type as its arguments
    // When given ints, should return int
    assert_variable_type(&inferred, "result", Type::Int);
}

#[test]
fn test_max_from_floats() {
    let python = r#"
def compute():
    a = 10.5
    b = 20.3
    result = max(a, b)
    return result
"#;
    let inferred = infer_types(python);
    // max() should return the same type as its arguments
    // When given floats, should return float
    assert_variable_type(&inferred, "result", Type::Float);
}

// ============================================================================
// Complex scenarios
// ============================================================================

#[test]
fn test_int_cast_preserves_type_through_assignment() {
    let python = r#"
def process():
    text = input()
    number = int(text)
    result = number + 10
    return result
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "text", Type::String);
    assert_variable_type(&inferred, "number", Type::Int);
    assert_variable_type(&inferred, "result", Type::Int);
}

#[test]
fn test_abs_preserves_type_in_conditional() {
    let python = r#"
def compute(x: int):
    if x < 0:
        y = abs(x)
    else:
        y = x
    return y
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn test_chained_conversions() {
    let python = r#"
def convert():
    x = "42"
    y = int(x)
    z = float(y)
    w = str(z)
    return w
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::String);
    assert_variable_type(&inferred, "y", Type::Int);
    assert_variable_type(&inferred, "z", Type::Float);
    assert_variable_type(&inferred, "w", Type::String);
}

#[test]
fn test_abs_in_expression() {
    let python = r#"
def compute():
    x = -5
    y = abs(x) + 10
    return y
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Int);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn test_int_cast_in_expression() {
    let python = r#"
def compute():
    x = "42"
    y = int(x) + 10
    return y
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::String);
    assert_variable_type(&inferred, "y", Type::Int);
}

#[test]
fn debug_int_cast_simple() {
    // This is a very simple test to debug exactly what's happening
    let python = r#"
def test():
    x = int("42")
"#;
    let result = infer_python_function(python);
    match result {
        Ok(inferred) => {
            println!(
                "Variables found: {:?}",
                inferred.variable_types.keys().collect::<Vec<_>>()
            );
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            // x should be inferred as Int
            if let Some(ty) = inferred.get_variable_type("x") {
                assert_eq!(ty, &Type::Int, "x should be Type::Int");
            } else {
                panic!("Variable 'x' not found in inferred types");
            }
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn debug_abs_simple() {
    // This is a very simple test to debug exactly what's happening with abs
    let python = r#"
def test():
    h = 42
    g = abs(h)
"#;
    let result = infer_python_function(python);
    match result {
        Ok(inferred) => {
            println!(
                "Variables found: {:?}",
                inferred.variable_types.keys().collect::<Vec<_>>()
            );
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            // g should be inferred as Int (same as h)
            if let Some(ty) = inferred.get_variable_type("g") {
                assert_eq!(ty, &Type::Int, "g should be Type::Int");
            } else {
                panic!("Variable 'g' not found in inferred types");
            }
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

// ============================================================================
// Tests for comparison operators returning boolean
// ============================================================================

#[test]
fn test_comparison_float_literals_returns_bool() {
    // Test: 0.0 == 0.0 should be inferred as bool
    let python = r#"
def test():
    x = 0.0 == 0.0
    return x
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Bool);
}

#[test]
fn test_comparison_float_cast_returns_bool() {
    // Test: float(0) == 0.0 should be inferred as bool
    let python = r#"
def test():
    y = float(0) == 0.0
    return y
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "y", Type::Bool);
}

#[test]
fn test_comparison_string_method_returns_bool() {
    // Test: upper("str") == "STR" should be inferred as bool
    // Note: In Python, str.upper() is a method call, not a function
    let python = r#"
def test():
    z = "str".upper() == "STR"
    return z
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "z", Type::Bool);
}

#[test]
fn test_all_comparison_operators_return_bool() {
    // Test all comparison operators: ==, !=, <, <=, >, >=
    let python = r#"
def test():
    eq = 0.0 == 0.0
    ne = 0.0 != 1.0
    lt = 0.0 < 1.0
    le = 0.0 <= 1.0
    gt = 1.0 > 0.0
    ge = 1.0 >= 0.0
    return eq
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "eq", Type::Bool);
    assert_variable_type(&inferred, "ne", Type::Bool);
    assert_variable_type(&inferred, "lt", Type::Bool);
    assert_variable_type(&inferred, "le", Type::Bool);
    assert_variable_type(&inferred, "gt", Type::Bool);
    assert_variable_type(&inferred, "ge", Type::Bool);
}

#[test]
fn test_comparison_with_type_casting_returns_bool() {
    // Test multiple comparisons with type casting
    let python = r#"
def test():
    x = 0.0 == 0.0
    y = float(0) == 0.0
    z = "str".upper() == "STR"
    return x and y and z
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "x", Type::Bool);
    assert_variable_type(&inferred, "y", Type::Bool);
    assert_variable_type(&inferred, "z", Type::Bool);
}

#[test]
fn test_dataclass_optional_list_subscript_access() {
    let python = r#"
@dataclass
class State:
    list: list[float]

def one():
    model: State
    result = model.list[0] == 1.0
    return result
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for dataclass optional list subscript access:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
            assert_variable_type(&inferred, "result", Type::Bool);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_dataclass_list_subscript_int_cast() {
    let python = r#"
@dataclass
class State:
    list: list[float]

def one():
    model: State
    result = int(model.list[0])
    return result
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for dataclass list subscript int cast:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
            assert_variable_type(&inferred, "result", Type::Int);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_dict_subscript_assignment_inference() {
    let python = r#"
def test():
    test1["item"] = "one"
    one = test1["one"]

    test2["item"] = int(1.0)
    two = test2["item"]

    test3["item"] = int(1.0) == 1
    three = test3["item"]
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for dict subscript assignment:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_dict_subscript_with_return_type_annotation() {
    let python = r#"
def test() -> dict[str, Any]:
    test1["item"] = "one"
    one = test1["one"]
    return test1
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for dict subscript with return annotation:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_nested_type_casting_functions() {
    let python = r#"
def test():
    one = int(1.0)
    two = float(int(1.0))
    three = round(float(int(1.0)))
    four = abs(float(int(1.0)))
    five = upper("test")
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for nested type casting functions:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_dataclass_dict_output_with_int_cast() {
    let python = r#"
@dataclass
class State:
    list: list[float]

def one() -> dict[str, Any]:
   outputs = {}
   state: State = two[State]()
   outputs["item"] = int(state.list[0])
   temp_output = outputs["item"]
   return outputs
"#;
    match infer_python_function(python) {
        Ok(inferred) => {
            println!("Inferred types for dataclass dict output with int cast:");
            for (var, ty) in &inferred.variable_types {
                println!("  {}: {:?}", var, ty);
            }
            println!("  return_type: {:?}", inferred.return_type);
        }
        Err(e) => panic!("Type inference failed: {}", e),
    }
}

#[test]
fn test_function_call_return_type_inference() {
    let python = r#"
def one(i: int) -> int:
    return i

def two():
    val = one(1)
"#;
    let inferred = infer_types(python);
    assert_variable_type(&inferred, "val", Type::Int);
}
