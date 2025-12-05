//! Tests for augmented assignment on Optional fields and variables.
//!
//! Note: The implementation may use CSE (Common Subexpression Elimination) which
//! transforms `c.value += 1` into:
//!   let _cse_temp_0 = c.value.unwrap() + 1;
//!   c.value = Some(_cse_temp_0);
//! This is semantically equivalent to the direct pattern `c.value = Some(c.value.unwrap() + 1)`

use crate::test_helpers::{transpile, transpile_and_check};

/// Check if generated code has the Optional unwrap-modify-rewrap pattern.
/// Accepts both direct and CSE-optimized patterns.
fn has_optional_augassign_pattern(rust_code: &str, field: &str, op: &str) -> bool {
    // Direct pattern: field = Some(field.unwrap() OP value)
    let direct_pattern = format!("{} = Some({}.unwrap() {} ", field, field, op);
    if rust_code.contains(&direct_pattern) {
        return true;
    }

    // CSE pattern: let _cse_temp_X = field.unwrap() OP value; field = Some(_cse_temp_X);
    // Check for .unwrap() OP and assignment to Some(_cse_temp
    let unwrap_pattern = format!("{}.unwrap() {} ", field, op);
    let some_assign_pattern = format!("{} = Some(_cse_temp", field);

    rust_code.contains(&unwrap_pattern) && rust_code.contains(&some_assign_pattern)
}

#[test]
fn test_optional_field_add_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Counter:
    value: Optional[int]

def increment(c: Counter) -> None:
    c.value += 1
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "c.value", "+"),
        "Should generate unwrap-modify-rewrap pattern for += on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_sub_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Counter:
    value: Optional[int]

def decrement(c: Counter) -> None:
    c.value -= 1
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "c.value", "-"),
        "Should generate unwrap-modify-rewrap pattern for -= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_mul_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    factor: Optional[int]

def double(n: Numbers) -> None:
    n.factor *= 2
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "n.factor", "*"),
        "Should generate unwrap-modify-rewrap pattern for *= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_div_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def halve(n: Numbers) -> None:
    n.value /= 2
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "n.value", "/"),
        "Should generate unwrap-modify-rewrap pattern for /= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_mod_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Numbers:
    value: Optional[int]

def mod_by(n: Numbers, d: int) -> None:
    n.value %= d
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "n.value", "%"),
        "Should generate unwrap-modify-rewrap pattern for %%= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_bitand_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    mask: Optional[int]

def apply_mask(b: Bits, m: int) -> None:
    b.mask &= m
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "b.mask", "&"),
        "Should generate unwrap-modify-rewrap pattern for &= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_bitor_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    flags: Optional[int]

def set_flag(b: Bits, flag: int) -> None:
    b.flags |= flag
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "b.flags", "|"),
        "Should generate unwrap-modify-rewrap pattern for |= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_bitxor_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    bits: Optional[int]

def toggle(b: Bits, mask: int) -> None:
    b.bits ^= mask
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "b.bits", "^"),
        "Should generate unwrap-modify-rewrap pattern for ^= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_add_assign() {
    let source = r#"
from typing import Optional

def increment_var(x: Optional[int]) -> Optional[int]:
    x += 5
    return x
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "x", "+"),
        "Should generate unwrap-modify-rewrap pattern for += on Optional variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_mul_assign() {
    let source = r#"
from typing import Optional

def scale_var(x: Optional[int], factor: int) -> Optional[int]:
    x *= factor
    return x
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "x", "*"),
        "Should generate unwrap-modify-rewrap pattern for *= on Optional variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_non_optional_field_no_wrapping() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Counter:
    value: int

def increment(c: Counter) -> None:
    c.value += 1
"#;

    let rust_code = transpile(source);
    // Non-optional field should NOT have unwrap-rewrap pattern
    assert!(
        !has_optional_augassign_pattern(&rust_code, "c.value", "+"),
        "Non-optional field should not use unwrap-rewrap pattern.\nGenerated:\n{}",
        rust_code
    );
    // Should either use c.value += 1 or c.value = c.value + 1 (CSE pattern)
    assert!(
        rust_code.contains("c.value += 1")
            || rust_code.contains("c.value = c.value + 1")
            || (rust_code.contains("c.value + 1") && rust_code.contains("c.value = _cse_temp")),
        "Non-optional field should use regular augmented assignment.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_lshift_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    bits: Optional[int]

def shift_left(b: Bits, n: int) -> None:
    b.bits <<= n
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "b.bits", "<<"),
        "Should generate unwrap-modify-rewrap pattern for <<= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_rshift_assign() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Bits:
    bits: Optional[int]

def shift_right(b: Bits, n: int) -> None:
    b.bits >>= n
"#;

    let rust_code = transpile_and_check(source, &["Some(", ".unwrap()"]);
    assert!(
        has_optional_augassign_pattern(&rust_code, "b.bits", ">>"),
        "Should generate unwrap-modify-rewrap pattern for >>= on Optional field.\nGenerated:\n{}",
        rust_code
    );
}
