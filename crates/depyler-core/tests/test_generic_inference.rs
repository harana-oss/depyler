use depyler_core::{DepylerPipeline, hir::Type};

#[test]
fn test_simple_generic_function() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def identity(x: T) -> T:
    return x
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate a generic function
    assert!(rust_code.contains("pub fn identity<T: Clone>(x: T)"));
    assert!(rust_code.contains("-> T"));
    // Function body should be just "x" not "return x;"
    assert!(
        !rust_code.contains("return x"),
        "Final return should use implicit return (idiomatic Rust), not explicit return keyword"
    );
}

#[test]
fn test_generic_list_function() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import List

def first_element(items: List[T]) -> T:
    return items[0]
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate a generic function with Vec<T>
    assert!(rust_code.contains("T: Clone") || rust_code.contains("T:Clone"));
    assert!(rust_code.contains("Vec<T>"));
    assert!(rust_code.contains("-> T") || rust_code.contains("Result<T"));
}

#[test]
fn test_multiple_type_parameters() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Tuple

def pair(a: T, b: U) -> Tuple[T, U]:
    return (a, b)
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate function with two type parameters
    // Check that both T and U are declared with Clone bounds
    assert!(rust_code.contains("T: Clone") || rust_code.contains("T:Clone"));
    assert!(rust_code.contains("U: Clone") || rust_code.contains("U:Clone"));
    // Check parameter types
    assert!(rust_code.contains("a:") && rust_code.contains("T"));
    assert!(rust_code.contains("b:") && rust_code.contains("U"));
    // Return type might be Tuple<T, U> instead of (T, U)
    assert!(rust_code.contains("Tuple<T, U>") || rust_code.contains("(T, U)"));
}

#[test]
fn test_generic_with_constraints() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def compare(a: T, b: T) -> bool:
    return a < b
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should infer PartialOrd constraint from < operator
    assert!(rust_code.contains("T: Clone + PartialOrd") || rust_code.contains("T: PartialOrd + Clone"));
}

#[test]
fn test_union_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Union

def process_value(x: Union[int, str]) -> str:
    if isinstance(x, int):
        return str(x)
    else:
        return x
"#;

    let hir = pipeline.parse_to_hir(python_code).unwrap();
    assert_eq!(hir.functions.len(), 1);

    let func = &hir.functions[0];
    match &func.params[0].ty {
        Type::Union(types) => {
            assert_eq!(types.len(), 2);
            assert!(types.contains(&Type::Int));
            assert!(types.contains(&Type::String));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_generic_dict() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Dict

def get_value(mapping: Dict[K, V], key: K) -> V:
    return mapping[key]
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate function with K and V type parameters
    assert!(rust_code.contains("K: Clone") || rust_code.contains("K:"));
    assert!(rust_code.contains("V: Clone") || rust_code.contains("V:"));
    assert!(rust_code.contains("HashMap<K, V>"));
}

#[test]
fn test_type_var_in_optional() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Optional

def maybe_value(x: Optional[T]) -> T:
    return x
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle Optional with type parameter
    assert!(rust_code.contains("T: Clone") || rust_code.contains("T:Clone"));
    assert!(rust_code.contains("x: Option<T>"));
}

// Test commented out - class instantiation not yet supported
// #[test]
// fn test_generic_class_instantiation() {
//     let pipeline = DepylerPipeline::new();
//     let python_code = r#"
// from typing import Generic
//
// def create_container() -> Container[int]:
//     return Container[int]()
// "#;
//
//     let hir = pipeline.parse_to_hir(python_code).unwrap();
//     assert_eq!(hir.functions.len(), 1);
//
//     let func = &hir.functions[0];
//     match &func.ret_type {
//         Type::Generic { base, params } => {
//             assert_eq!(base, "Container");
//             assert_eq!(params.len(), 1);
//             assert_eq!(params[0], Type::Int);
//         }
//         _ => panic!("Expected Generic type"),
//     }
// }

// ============================================================================
// GENERIC METHOD CALLS - Subscript Type Parameter Syntax
// ============================================================================

#[test]
fn test_generic_method_call_single_type_param() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_infer():
    result = infer[str]("hello")
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate method call with type parameter
    assert!(rust_code.contains("infer::<") || rust_code.contains("infer::"));
}

#[test]
fn test_generic_method_call_multiple_type_params() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_convert():
    result = convert[int, str](42)
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle multiple type parameters
    assert!(rust_code.contains("convert::<") || rust_code.contains("convert::"));
}

#[test]
fn test_generic_method_on_object() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_method():
    obj = SomeClass()
    result = obj.method[int]("value")
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should generate method call on object with type parameter
    assert!(rust_code.contains(".method::<") || rust_code.contains(".method::"));
}

#[test]
fn test_generic_method_with_complex_types() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import List, Dict

def test_complex():
    result = process[List[int], Dict[str, int]]([1, 2, 3])
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle complex generic types as parameters
    assert!(rust_code.contains("process::<Vec<i32>") || rust_code.contains("process::"));
}

#[test]
fn test_chained_generic_method_calls() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_chain():
    result = obj.first[int](10).second[str]("hello")
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle chained method calls with type parameters
    assert!(rust_code.contains(".first::<") && rust_code.contains(".second::<"));
}

#[test]
fn test_generic_method_hir_representation() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_hir():
    result = cast[int]("42")
    return result
"#;

    let hir = pipeline.parse_to_hir(python_code).unwrap();
    assert_eq!(hir.functions.len(), 1);

    // Check HIR contains the generic call information
    let func = &hir.functions[0];
    assert!(!func.body.is_empty());
}

#[test]
fn test_generic_method_with_type_vars() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import TypeVar

T = TypeVar('T')

def wrapper(value: T) -> T:
    result = identity[T](value)
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should use type variable T in generic method call
    assert!(rust_code.contains("identity::<T>") || rust_code.contains("identity::"));
}

#[test]
fn test_generic_constructor_call() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def create_container():
    container = Container[int]()
    return container
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle generic constructor/instantiation
    assert!(rust_code.contains("Container::<") || rust_code.contains("Container::"));
}

#[test]
fn test_generic_static_method_call() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_static():
    result = MyClass.create[int](42)
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok());
    let rust_code = result.unwrap();

    // Should handle static/class method with type parameters
    assert!(rust_code.contains("MyClass::create::<") || rust_code.contains("create::"));
}

#[test]
fn test_generic_call_with_keyword_args_and_array() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def test_func():
    result = call[Type](name = "name", input = [random.random(), float(int(2400 - 1200))])
    return result
"#;

    let result = pipeline.transpile(python_code);
    assert!(result.is_ok(), "Transpilation failed: {:?}", result.err());
    let rust_code = result.unwrap();

    println!("Generated Rust code:\n{}", rust_code);

    // Should generate a generic call with turbofish syntax
    assert!(
        rust_code.contains("call::<Type>"),
        "Expected generic call syntax with turbofish, got:\n{}",
        rust_code
    );

    // Should have the string literal "name" as first argument
    assert!(
        rust_code.contains("\"name\""),
        "Expected first argument 'name', got:\n{}",
        rust_code
    );

    // Should have an array/vector literal with random.random() call
    assert!(
        rust_code.contains("vec!") && rust_code.contains("rand::random"),
        "Expected array/vector literal with rand::random call, got:\n{}",
        rust_code
    );

    // Should have the arithmetic computation (2400 - 1200)
    assert!(
        rust_code.contains("2400") && rust_code.contains("1200"),
        "Expected arithmetic computation with 2400 and 1200, got:\n{}",
        rust_code
    );
}
