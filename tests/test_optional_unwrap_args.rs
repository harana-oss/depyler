//! Tests for Optional auto-unwrap when passing Optional fields/variables to non-Optional parameters.
//!
//! When a dataclass field is Optional[T] and is passed to a function expecting T,
//! the transpiler should automatically add `.unwrap()` to unwrap the Option.

use crate::test_helpers::{transpile, transpile_and_check};

/// Basic test: Optional field passed to function expecting non-Optional int
#[test]
fn test_optional_field_to_non_optional_param() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def process_age(age: int) -> int:
    return age + 10

def use_person(person: Person) -> int:
    return process_age(person.age)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("person.age.unwrap()") || rust_code.contains("person.age . unwrap()"),
        "Should unwrap Optional field when passed to non-Optional param.\nGenerated:\n{}",
        rust_code
    );
}

/// Optional to Optional: No unwrap needed when both arg and param are Optional
#[test]
fn test_optional_to_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    age: Optional[int]

def process_maybe_age(age: Optional[int]) -> Optional[int]:
    if age is None:
        return None
    return age + 10

def use_person(person: Person) -> Optional[int]:
    return process_maybe_age(person.age)
"#;

    let rust_code = transpile(source);
    // Should NOT have unwrap when both are Optional
    let use_person_fn = rust_code.split("fn use_person").nth(1).unwrap_or(&rust_code);

    // Check that person.age is NOT followed by .unwrap() in the function call
    // The pattern should be process_maybe_age(person.age) not process_maybe_age(person.age.unwrap())
    assert!(
        !use_person_fn.contains("person.age.unwrap()") && !use_person_fn.contains("person.age . unwrap()"),
        "Should NOT unwrap when both arg and param are Optional.\nGenerated:\n{}",
        rust_code
    );
}

/// Multiple arguments: Only some need unwrapping
#[test]
fn test_multiple_args_partial_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Data:
    value: Optional[int]
    name: str

def process(value: int, name: str) -> str:
    return name + str(value)

def use_data(d: Data) -> str:
    return process(d.value, d.name)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    // value should be unwrapped, name should not
    assert!(
        rust_code.contains("d.value.unwrap()") || rust_code.contains("d.value . unwrap()"),
        "Should unwrap Optional d.value.\nGenerated:\n{}",
        rust_code
    );
}

/// Optional float field
#[test]
fn test_optional_float_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Measurement:
    reading: Optional[float]

def double_reading(x: float) -> float:
    return x * 2.0

def process_measurement(m: Measurement) -> float:
    return double_reading(m.reading)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("m.reading.unwrap()") || rust_code.contains("m.reading . unwrap()"),
        "Should unwrap Optional float field.\nGenerated:\n{}",
        rust_code
    );
}

/// Optional string field
#[test]
fn test_optional_string_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class User:
    nickname: Optional[str]

def greet(name: str) -> str:
    return "Hello, " + name

def greet_user(u: User) -> str:
    return greet(u.nickname)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    // String fields may have .clone() before .unwrap()
    assert!(
        rust_code.contains("nickname.unwrap()") || rust_code.contains("nickname.clone().unwrap()"),
        "Should unwrap Optional string field.\nGenerated:\n{}",
        rust_code
    );
}

/// Non-optional field should not get unwrap
#[test]
fn test_non_optional_field_no_unwrap() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

def add_coords(x: int, y: int) -> int:
    return x + y

def process_point(p: Point) -> int:
    return add_coords(p.x, p.y)
"#;

    let rust_code = transpile(source);
    // Should NOT have unwrap for non-Optional fields
    assert!(
        !rust_code.contains("p.x.unwrap()") && !rust_code.contains("p.y.unwrap()"),
        "Should NOT unwrap non-Optional fields.\nGenerated:\n{}",
        rust_code
    );
}

/// Mixed: some fields Optional, some not
#[test]
fn test_mixed_optional_non_optional() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Config:
    required_value: int
    optional_value: Optional[int]

def compute(a: int, b: int) -> int:
    return a + b

def use_config(c: Config) -> int:
    return compute(c.required_value, c.optional_value)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    // optional_value should be unwrapped
    assert!(
        rust_code.contains("c.optional_value.unwrap()") || rust_code.contains("c.optional_value . unwrap()"),
        "Should unwrap Optional field.\nGenerated:\n{}",
        rust_code
    );
    // required_value should NOT be unwrapped
    assert!(
        !rust_code.contains("c.required_value.unwrap()"),
        "Should NOT unwrap non-Optional field.\nGenerated:\n{}",
        rust_code
    );
}

/// Optional bool field
#[test]
fn test_optional_bool_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Settings:
    enabled: Optional[bool]

def check_enabled(flag: bool) -> bool:
    return flag

def apply_settings(s: Settings) -> bool:
    return check_enabled(s.enabled)
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("s.enabled.unwrap()") || rust_code.contains("s.enabled . unwrap()"),
        "Should unwrap Optional bool field.\nGenerated:\n{}",
        rust_code
    );
}
