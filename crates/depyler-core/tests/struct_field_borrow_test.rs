//! TDD Tests for Struct Field Borrowing
//!
//! When passing a struct's String field to a function, the transpiler should
//! generate idiomatic Rust that borrows instead of moving/cloning.
//!
//! Key issue: `take_str(state.field)` where field is String will MOVE the field
//! out of the struct, which is a Rust compile error.
//!
//! The idiomatic solution is:
//! 1. Generate function signature with `&str` instead of `String`
//! 2. Pass `&state.field` (auto-derefs String to &str)
//!
//! This avoids unnecessary allocations and is idiomatic Rust.

use depyler_core::DepylerPipeline;

#[test]
fn test_struct_field_passed_to_fn_should_borrow() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
class State:
    field: str

    def __init__(self, field: str):
        self.field = field

def take_str(s: str) -> None:
    pass

def main() -> None:
    state = State("hello")
    take_str(state.field)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();

    // Function should take &str to allow borrowing
    assert!(
        rust_code.contains("fn take_str(s: &str)"),
        "Function should take &str for efficiency.\nGot:\n{}",
        rust_code
    );

    // Should pass reference to field
    assert!(
        rust_code.contains("take_str(&state.field)"),
        "Should borrow struct field with &state.field.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_self_field_passed_to_fn_should_borrow() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
class State:
    field: str

    def __init__(self, field: str):
        self.field = field

    def process(self) -> None:
        take_str(self.field)

def take_str(s: str) -> None:
    pass
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();

    // Should borrow self.field
    assert!(
        rust_code.contains("take_str(&self.field)"),
        "Should borrow self.field with &self.field.\nGot:\n{}",
        rust_code
    );
}

#[test]
fn test_nested_field_passed_to_fn_should_borrow() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
class Inner:
    value: str

    def __init__(self, value: str):
        self.value = value

class Outer:
    inner: Inner

    def __init__(self, inner: Inner):
        self.inner = inner

def take_str(s: str) -> None:
    pass

def main() -> None:
    inner = Inner("hello")
    outer = Outer(inner)
    take_str(outer.inner.value)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();

    // Nested field access should also borrow
    assert!(
        rust_code.contains("&outer.inner.value"),
        "Should borrow nested field with &outer.inner.value.\nGot:\n{}",
        rust_code
    );
}
