//! Tests for Optional auto-unwrap when applying unary operators.

use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_unary_neg_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def negate(n: Numbers) -> int:
    return -n.value
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("-n.value.unwrap()") || rust_code.contains("(- n.value.unwrap())"),
        "Should unwrap Optional before applying unary negation.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_not_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Flags:
    active: Optional[bool]

def invert(f: Flags) -> bool:
    return not f.active
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("!f.active.unwrap()"),
        "Should unwrap Optional before applying logical not.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_bitnot_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    mask: Optional[int]

def flip(b: Bits) -> int:
    return ~b.mask
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("!b.mask.unwrap()"),
        "Should unwrap Optional before applying bitwise not.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_neg_optional_variable() {
    let source = r#"
from typing import Optional

def negate_var(x: Optional[int]) -> int:
    return -x
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("-x.unwrap()") || rust_code.contains("(- x.unwrap())"),
        "Should unwrap Optional variable before applying unary negation.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_neg_non_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Numbers:
    value: int

def negate(n: Numbers) -> int:
    return -n.value
"#;

    let rust_code = transpile(source);
    assert!(
        !rust_code.contains(".unwrap()"),
        "Should NOT unwrap non-Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_not_non_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Flags:
    active: bool

def invert(f: Flags) -> bool:
    return not f.active
"#;

    let rust_code = transpile(source);
    assert!(
        !rust_code.contains(".unwrap()"),
        "Should NOT unwrap non-Optional bool field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_double_negation_optional() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def double_negate(n: Numbers) -> int:
    return --n.value
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()"),
        "Should handle double negation with Optional unwrap.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_combined_with_binary() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def compute(n: Numbers) -> int:
    return -n.value + 5
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()"),
        "Should unwrap Optional in combined unary/binary expression.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_pos_optional_field() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def positive(n: Numbers) -> int:
    return +n.value
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains("n.value.unwrap()"),
        "Should unwrap Optional for unary plus.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_unary_neg_optional_float() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Measurements:
    temperature: Optional[float]

def negate(m: Measurements) -> float:
    return -m.temperature
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()"),
        "Should unwrap Optional float before negation.\nGenerated:\n{}",
        rust_code
    );
}
