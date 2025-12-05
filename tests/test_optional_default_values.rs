//! Tests for Optional with default values (or pattern).

use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_optional_string_or_default() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def get_name(p: Person) -> str:
    return p.name or "Unknown"
"#;

    let rust_code = transpile_and_check(source, &["unwrap_or_else"]);
    assert!(
        rust_code.contains("unwrap_or_else"),
        "Should use unwrap_or_else for Optional or default.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_or_default() {
    let source = r#"
from typing import Optional

def get_value(x: Optional[str]) -> str:
    return x or "default"
"#;

    let rust_code = transpile_and_check(source, &["unwrap_or_else"]);
    assert!(
        rust_code.contains("unwrap_or_else"),
        "Should use unwrap_or_else for Optional variable or default.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_int_or_default() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Counter:
    value: Optional[int]

def get_value(c: Counter) -> int:
    return c.value or 0
"#;

    let rust_code = transpile_and_check(source, &["unwrap_or_else"]);
    assert!(
        rust_code.contains("unwrap_or_else"),
        "Should use unwrap_or_else for Optional[int] or default.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_chained_optional_or() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    nickname: Optional[str]
    name: Optional[str]

def get_display(p: Person) -> str:
    return p.nickname or p.name or "Anonymous"
"#;

    let rust_code = transpile_and_check(source, &["or_else"]);
    assert!(
        rust_code.contains("or_else"),
        "Chained optionals should use or_else.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_non_optional_or_unchanged() {
    let source = r#"
def check_flags(a: bool, b: bool) -> bool:
    return a or b
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains("||"),
        "Non-optional or should remain as || operator.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("unwrap_or_else"),
        "Non-optional or should not use unwrap_or_else.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_or_in_assignment() {
    let source = r#"
from typing import Optional

def process(name: Optional[str]) -> str:
    result = name or "fallback"
    return result
"#;

    let rust_code = transpile_and_check(source, &["unwrap_or_else"]);
    assert!(
        rust_code.contains("unwrap_or_else"),
        "Optional or in assignment should use unwrap_or_else.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_or_method_call() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Config:
    prefix: Optional[str]

def get_prefix(c: Config) -> str:
    return c.prefix or "default_"
"#;

    let rust_code = transpile_and_check(source, &["unwrap_or_else"]);
    assert!(
        rust_code.contains("unwrap_or_else"),
        "Optional field or should use unwrap_or_else.\nGenerated:\n{}",
        rust_code
    );
}
