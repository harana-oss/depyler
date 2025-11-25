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
