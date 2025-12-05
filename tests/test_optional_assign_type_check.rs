//! Tests for Optional auto-unwrap when assigning to non-Optional annotated variables.

use crate::test_helpers::{transpile, transpile_and_check, transpile_check_absent};

#[test]
fn test_assign_optional_field_to_int() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def extract_age(p: Person) -> None:
    age: int = p.age
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("p.age.unwrap()"),
        "Should unwrap Optional field when assigning to non-Optional annotated variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_optional_field_to_str() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def extract_name(p: Person) -> None:
    name: str = p.name
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()"),
        "Should unwrap Optional str field when assigning to non-Optional annotated variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_optional_to_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def copy_age(p: Person) -> None:
    age: Optional[int] = p.age
"#;

    let rust_code = transpile_check_absent(source, &[".unwrap()"]);
    assert!(
        !rust_code.contains(".unwrap()"),
        "Should NOT unwrap when both sides are Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_optional_param_to_int() {
    let source = r#"
from typing import Optional

def extract(x: Optional[int]) -> None:
    value: int = x
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("x.unwrap()"),
        "Should unwrap Optional parameter when assigning to non-Optional annotated variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_non_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Person:
    age: int

def extract_age(p: Person) -> None:
    age: int = p.age
"#;

    let rust_code = transpile_check_absent(source, &[".unwrap()"]);
    assert!(
        !rust_code.contains(".unwrap()"),
        "Should NOT add unwrap when source is not Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_optional_field_to_float() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Data:
    value: Optional[float]

def extract(d: Data) -> None:
    val: float = d.value
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("d.value.unwrap()"),
        "Should unwrap Optional float field when assigning to non-Optional annotated variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_assign_optional_field_to_bool() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Flags:
    active: Optional[bool]

def extract(f: Flags) -> None:
    is_active: bool = f.active
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("f.active.unwrap()"),
        "Should unwrap Optional bool field when assigning to non-Optional annotated variable.\nGenerated:\n{}",
        rust_code
    );
}
