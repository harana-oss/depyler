//
// Tests that custom validators using argparse.ArgumentTypeError
// correctly map to Rust Result<T, String> pattern.
//
// Root cause: raise argparse.ArgumentTypeError(msg) not mapped to Err()
// Solution: Detect ArgumentTypeError and generate Result return type

use crate::test_helpers::transpile_and_check;

#[test]
fn test_port_number_validator() {
    // Port number validator from complex_cli.py
    let python = r#"
import argparse

def port_number(value):
    try:
        port = int(value)
        if port < 1 or port > 65535:
            raise argparse.ArgumentTypeError(f"Port must be between 1 and 65535, got {port}")
        return port
    except ValueError:
        raise argparse.ArgumentTypeError(f"Port must be an integer, got '{value}'") from None
"#;

    let rust = transpile_and_check(python, &["Result<", "Err("]);

    // Should not reference Exception type (which doesn't exist in Rust)
    assert!(!rust.contains("Exception"), "Should not reference Exception: {}", rust);
}

#[test]
fn test_positive_int_validator() {
    // Positive int validator from complex_cli.py
    let python = r#"
import argparse

def positive_int(value):
    try:
        num = int(value)
        if num < 1:
            raise argparse.ArgumentTypeError(f"Value must be positive (>= 1), got {num}")
        return num
    except ValueError:
        raise argparse.ArgumentTypeError(f"Value must be an integer, got '{value}'") from None
"#;

    transpile_and_check(python, &["Result<", "Err("]);
}

#[test]
fn test_email_validator() {
    // Email address validator from complex_cli.py
    let python = r#"
import argparse
import re

def email_address(value):
    pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    if not re.match(pattern, value):
        raise argparse.ArgumentTypeError(f"Invalid email address: '{value}'")
    return value
"#;

    transpile_and_check(python, &["Result<", "Err("]);
}

#[test]
fn test_simple_validator() {
    // Simplified validator (no try/except)
    let python = r#"
import argparse

def validate_range(value):
    num = int(value)
    if num < 0 or num > 100:
        raise argparse.ArgumentTypeError("Value must be 0-100")
    return num
"#;

    transpile_and_check(python, &["Result<"]);
}

#[test]
fn test_real_world_complex_cli() {
    // Actual pattern from complex_cli.py (all three validators)
    let python = r#"
import argparse
import re

def port_number(value):
    try:
        port = int(value)
        if port < 1 or port > 65535:
            raise argparse.ArgumentTypeError(f"Port must be between 1 and 65535, got {port}")
        return port
    except ValueError:
        raise argparse.ArgumentTypeError(f"Port must be an integer, got '{value}'") from None

def positive_int(value):
    try:
        num = int(value)
        if num < 1:
            raise argparse.ArgumentTypeError(f"Value must be positive (>= 1), got {num}")
        return num
    except ValueError:
        raise argparse.ArgumentTypeError(f"Value must be an integer, got '{value}'") from None

def email_address(value):
    pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    if not re.match(pattern, value):
        raise argparse.ArgumentTypeError(f"Invalid email address: '{value}'")
    return value
"#;

    let rust = transpile_and_check(python, &[]);

    // All three functions should use Result<T, String>
    assert!(rust.matches("Result<").count() >= 3, "Should have 3+ Result types");

    // Should not have any Exception references
    assert!(!rust.contains("Exception"), "Should not reference Exception type");
}

#[test]
fn test_property_based_error_messages() {
    // Property: ArgumentTypeError can have any expression as message
    let test_cases = vec![
        (r#"raise argparse.ArgumentTypeError("literal string")"#, "literal"),
        (r#"raise argparse.ArgumentTypeError(f"formatted {value}")"#, "f-string"),
        (r#"raise argparse.ArgumentTypeError(msg)"#, "variable"),
    ];

    for (raise_stmt, _description) in test_cases {
        let python = format!(
            "import argparse\ndef validate(value):\n    {}\n    return value",
            raise_stmt
        );
        transpile_and_check(&python, &[]);
    }
}
