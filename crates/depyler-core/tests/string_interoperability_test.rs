use depyler_core::DepylerPipeline;

#[test]
fn test_str_literal_passed_to_string_param() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def take_str(s: str) -> str:
    return s

def main() -> None:
    result = take_str("msg")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn take_str(s: String) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("take_str(\"msg\".to_string())"), "\n{rust_code}");
}

#[test]
fn test_str_variable_passed_to_string_param() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def take_str(s: str) -> str:
    return s

def main() -> None:
    msg = "msg"
    result = take_str(msg)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn take_str(s: String) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("let msg = \"msg\".to_string()"), "\n{rust_code}");
    assert!(rust_code.contains("let result = take_str(msg)"), "\n{rust_code}");
}

#[test]
fn test_fstring_literal_passed_to_string_param() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def take_str(s: str) -> str:
    return s

def main() -> None:
    test = "test"
    result = take_str(f"msg {test}")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn take_str(s: String) -> String"), "\n{rust_code}");
    assert!(
        rust_code.contains("take_str(format!(\"msg {}\", test))"),
        "\n{rust_code}"
    );
}

#[test]
fn test_fstring_interpolation_returns_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def combine(a: str, b: str) -> str:
    return f"{a} and {b}"

def main() -> None:
    literal = "test"
    result = combine(literal, "another")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("format!(\"{} and {}\", a, b)"), "\n{rust_code}");
    assert!(
        rust_code.contains("let literal = \"test\".to_string()"),
        "\n{rust_code}"
    );
    assert!(
        rust_code.contains("combine(literal, \"another\".to_string())"),
        "\n{rust_code}"
    );
}

#[test]
fn test_str_literal_assignment_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_string() -> str:
    s: str = "literal"
    return s
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("get_string() -> String"), "\n{rust_code}");
    assert!(
        rust_code.contains("let s: String = \"literal\".to_string();"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_param_accepts_variable() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def identity(s: str) -> str:
    return s

def main() -> None:
    x = "test"
    result = identity(x)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn identity(s: String) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("let x = \"test\".to_string()"), "\n{rust_code}");
    assert!(rust_code.contains("let result = identity(x)"), "\n{rust_code}");
}

#[test]
fn test_fstring_return_type_is_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def outer(name: str) -> str:
    return f"Hello, {name}!"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("outer(name: String) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("format!(\"Hello, {}!\", name)"), "\n{rust_code}");
}

#[test]
fn test_string_concatenation_in_nested_calls() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def inner(s: str) -> str:
    return s + "!"

def outer(s: str) -> str:
    return inner(s + "?")

def main() -> None:
    result = outer("test")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn inner(s: String) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("fn outer(s: String) -> String"), "\n{rust_code}");
}

#[test]
fn test_string_concatenation_requires_owned_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def build_string() -> str:
    result = ""
    result = result + "a"
    result = result + "b"
    result = result + "c"
    return result
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("build_string() -> String"), "\n{rust_code}");
}

#[test]
fn test_generic_take_str_with_variable_and_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import TypeVar

T = TypeVar('T')

def take_str(s: str) -> str:
    return s

def main() -> None:
    msg = "hello"
    result1 = take_str(msg)
    result2 = take_str("world")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    // Function signature should use String, not &str
    assert!(rust_code.contains("fn take_str(s: String) -> String"), "\n{rust_code}");

    // Variable should be a String
    assert!(rust_code.contains("let msg = \"hello\".to_string()"), "\n{rust_code}");

    // Variable passed directly (already a String)
    assert!(rust_code.contains("let result1 = take_str(msg)"), "\n{rust_code}");

    assert!(
        rust_code.contains("let result2 = take_str(\"world\".to_string())"),
        "\n{rust_code}"
    );
}

#[test]
fn test_dataclass_state_mutation_with_chained_calls() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from dataclasses import dataclass

@dataclass
class State:
    val: str

def one(state: State) -> None:
    state.val = "new"
    two(state)

def two(state: State) -> None:
    three(state, val=state.val)

def three(state: State, val: str) -> None:
    pass
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("struct State"), "\n{rust_code}");
    assert!(rust_code.contains("val: String"), "\n{rust_code}");
    assert!(rust_code.contains("fn one(state: &mut State)"), "\n{rust_code}");
    assert!(rust_code.contains("fn two(state: &mut State)"), "\n{rust_code}");
    assert!(
        rust_code.contains("fn three(state: &mut State, val: String)"),
        "\n{rust_code}"
    );
    assert!(rust_code.contains("three(state, state.val.clone()"), "\n{rust_code}");
}

#[test]
fn test_ternary_assignment_in_nested_if() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class State:
    val: str

def one(state: State) -> None:
    if True:
        var = "One" if True else "Two"
    else:
        var = "Three"
    var = state.val
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    println!("{rust_code}");
    assert!(
        rust_code.contains("var = if true { \"One\".to_string() } else { \"Two\".to_string() };"),
        "\n{rust_code}"
    );
    assert!(rust_code.contains("\"Three\".to_string()"), "\n{rust_code}");
    assert!(rust_code.contains("var = state.val.clone();"), "\n{rust_code}");
}

#[test]
fn test_generic_function_call_with_string_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Test:
  val: str

def infer[T](str: str) -> None:
  pass

def main() -> None:
  infer[Test]("test")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("struct Test"), "\n{rust_code}");
    assert!(rust_code.contains("val: String"), "\n{rust_code}");
    assert!(
        rust_code.contains("infer::<Test>(\"test\".to_string())"),
        "\n{rust_code}"
    );
}
