//! Tests for Optional values in conditional contexts (if/while conditions).
//!
//! Tests `is None`, `is not None`, and Optional truthiness conversions.

use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_is_none_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def is_missing(p: Person) -> bool:
    return p.age is None
"#;

    let rust_code = transpile_and_check(source, &[".is_none()"]);
    assert!(
        rust_code.contains("p.age.is_none()"),
        "Should convert `is None` to `.is_none()`.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_is_not_none_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def has_age(p: Person) -> bool:
    return p.age is not None
"#;

    let rust_code = transpile_and_check(source, &[".is_some()"]);
    assert!(
        rust_code.contains("p.age.is_some()"),
        "Should convert `is not None` to `.is_some()`.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_is_none_optional_variable() {
    let source = r#"
from typing import Optional

def check_none(x: Optional[int]) -> bool:
    return x is None
"#;

    let rust_code = transpile_and_check(source, &[".is_none()"]);
    assert!(
        rust_code.contains("x.is_none()"),
        "Should convert variable `is None` to `.is_none()`.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_is_not_none_optional_variable() {
    let source = r#"
from typing import Optional

def check_not_none(x: Optional[int]) -> bool:
    return x is not None
"#;

    let rust_code = transpile_and_check(source, &[".is_some()"]);
    assert!(
        rust_code.contains("x.is_some()"),
        "Should convert variable `is not None` to `.is_some()`.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_string_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def check_name(p: Person) -> bool:
    if p.name:
        return True
    return False
"#;

    let rust_code = transpile_and_check(source, &["is_some_and"]);
    assert!(
        rust_code.contains("p.name.as_ref().is_some_and(|s| !s.is_empty())"),
        "Should use is_some_and for Optional[str] truthiness.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_int_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Counter:
    count: Optional[int]

def is_active(c: Counter) -> bool:
    if c.count:
        return True
    return False
"#;

    let rust_code = transpile_and_check(source, &["is_some_and"]);
    assert!(
        rust_code.contains("c.count.is_some_and(|n| n != 0)"),
        "Should use is_some_and for Optional[int] truthiness.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_list_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def has_items(c: Container) -> bool:
    if c.items:
        return True
    return False
"#;

    let rust_code = transpile_and_check(source, &["is_some_and"]);
    assert!(
        rust_code.contains("c.items.as_ref().is_some_and(|v| !v.is_empty())"),
        "Should use is_some_and for Optional[List] truthiness.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_in_if_condition() {
    let source = r#"
from typing import Optional

def check(s: Optional[str]) -> bool:
    if s:
        return True
    return False
"#;

    let rust_code = transpile_and_check(source, &["is_some_and"]);
    assert!(
        rust_code.contains("s.as_ref().is_some_and(|s| !s.is_empty())"),
        "Should use is_some_and for Optional[str] variable truthiness.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_is_none_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def check_missing(p: Person) -> str:
    if p.name is None:
        return "No name"
    return "Has name"
"#;

    let rust_code = transpile_and_check(source, &[".is_none()"]);
    assert!(
        rust_code.contains("p.name.is_none()"),
        "Should convert `is None` in if condition.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_is_not_none_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def check_present(p: Person) -> str:
    if p.name is not None:
        return "Has name"
    return "No name"
"#;

    let rust_code = transpile_and_check(source, &[".is_some()"]);
    assert!(
        rust_code.contains("p.name.is_some()"),
        "Should convert `is not None` in if condition.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_bool_in_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Flags:
    enabled: Optional[bool]

def is_enabled(f: Flags) -> bool:
    if f.enabled:
        return True
    return False
"#;

    let rust_code = transpile_and_check(source, &[".is_some()"]);
    assert!(
        rust_code.contains("f.enabled.is_some()"),
        "Should use is_some() for Optional[bool] truthiness.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_none_is_x_reversed() {
    let source = r#"
from typing import Optional

def check_reversed(x: Optional[int]) -> bool:
    return None is x
"#;

    let rust_code = transpile_and_check(source, &[".is_none()"]);
    assert!(
        rust_code.contains("x.is_none()"),
        "Should handle reversed `None is x` pattern.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_none_is_not_x_reversed() {
    let source = r#"
from typing import Optional

def check_reversed(x: Optional[int]) -> bool:
    return None is not x
"#;

    let rust_code = transpile_and_check(source, &[".is_some()"]);
    assert!(
        rust_code.contains("x.is_some()"),
        "Should handle reversed `None is not x` pattern.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_in_while_condition() {
    let source = r#"
from typing import Optional

def count_down(n: Optional[int]) -> int:
    count: int = 0
    while n:
        count = count + 1
    return count
"#;

    let rust_code = transpile_and_check(source, &["is_some_and"]);
    assert!(
        rust_code.contains("n.is_some_and(|n| n != 0)"),
        "Should use is_some_and for Optional[int] in while condition.\nGenerated:\n{}",
        rust_code
    );
}
