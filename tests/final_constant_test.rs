mod test_helpers;

use test_helpers::{transpile, transpile_and_check, transpile_check_absent};

#[test]
fn test_final_int_constant() {
    let python_code = r#"
from typing import Final

FIELD_GOAL: Final[int] = 600
"#;

    let rust_code = transpile_and_check(python_code, &["const FIELD_GOAL", ": i32", "= 600"]);

    assert!(
        !rust_code.contains("let FIELD_GOAL"),
        "Should NOT use 'let' for Final annotation, but got:\n{}",
        rust_code
    );
}

#[test]
fn test_final_string_constant() {
    let python_code = r#"
from typing import Final

API_KEY: Final[str] = "secret_key_123"
"#;

    let rust_code = transpile_and_check(python_code, &["const API_KEY"]);

    assert!(
        rust_code.contains(": &str") || rust_code.contains(": String"),
        "Should have string type annotation, but got:\n{}",
        rust_code
    );
}

#[test]
fn test_final_float_constant() {
    let python_code = r#"
from typing import Final

PI: Final[float] = 3.14159
"#;

    transpile_and_check(python_code, &["const PI", ": f64"]);
}

#[test]
fn test_multiple_final_constants() {
    let python_code = r#"
from typing import Final

MAX_CONNECTIONS: Final[int] = 100
TIMEOUT_MS: Final[int] = 5000
DEFAULT_NAME: Final[str] = "unnamed"
"#;

    transpile_and_check(
        python_code,
        &["const MAX_CONNECTIONS", "const TIMEOUT_MS", "const DEFAULT_NAME"],
    );
}

#[test]
fn test_final_vs_regular_variable() {
    let python_code = r#"
from typing import Final

def process() -> int:
    MAX_VALUE: Final[int] = 100
    current: int = 50
    return current
"#;

    transpile_and_check(python_code, &["const MAX_VALUE", "let current"]);
}
