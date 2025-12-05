// Module: math - Python math module validation

use crate::test_helpers::transpile_and_check;

#[test]
fn test_math_cos() {
    let python = r#"
import math

def calculate_cos(x: float) -> float:
    return math.cos(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.cos()") || result.contains("f64::cos"));
}

#[test]
fn test_math_tan() {
    let python = r#"
import math

def calculate_tan(x: float) -> float:
    return math.tan(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.tan()") || result.contains("f64::tan"));
}

//
#[test]
fn test_math_sqrt() {
    let python = r#"
import math

def calculate_sqrt(x: float) -> float:
    return math.sqrt(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.sqrt()") || result.contains("f64::sqrt"));
}

#[test]
fn test_math_pow() {
    let python = r#"
import math

def calculate_pow(x: float, y: float) -> float:
    return math.pow(x, y)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("powf") || result.contains("powi"));
}

#[test]
fn test_math_exp() {
    let python = r#"
import math

def calculate_exp(x: float) -> float:
    return math.exp(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.exp()") || result.contains("f64::exp"));
}

/// Test that sum() on list comprehensions with math.exp() uses f64 type inference
#[test]
fn test_sum_of_math_exp_list_comprehension() {
    let python = r#"
import math

def softmax(logits: list[float]) -> list[float]:
    max_logit = max(logits)
    exps = [math.exp(x - max_logit) for x in logits]
    total = sum(exps)
    return [e / total for e in exps]
"#;

    let result = transpile_and_check(python, &[]);
    // sum(exps) should use f64 since exps contains math.exp() results
    assert!(
        result.contains("sum::<f64>()"),
        "sum() on list of math.exp() results should use f64, got:\n{}",
        result
    );
    // Should NOT use i32 for the sum
    assert!(
        !result.contains("exps") || !result.contains("sum::<i32>()") || result.contains("sum::<f64>()"),
        "sum() on float list should not use i32, got:\n{}",
        result
    );
}

/// Test that sum() on list comprehensions with other math functions also uses f64
#[test]
fn test_sum_of_math_sqrt_list_comprehension() {
    let python = r#"
import math

def sum_of_roots(values: list[float]) -> float:
    roots = [math.sqrt(x) for x in values]
    return sum(roots)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(
        result.contains("sum::<f64>()"),
        "sum() on list of math.sqrt() results should use f64, got:\n{}",
        result
    );
}

#[test]
fn test_math_log() {
    let python = r#"
import math

def calculate_log(x: float) -> float:
    return math.log(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.ln()") || result.contains("f64::ln"));
}

#[test]
fn test_math_log10() {
    let python = r#"
import math

def calculate_log10(x: float) -> float:
    return math.log10(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.log10()") || result.contains("f64::log10"));
}

//
#[test]
fn test_math_ceil() {
    let python = r#"
import math

def calculate_ceil(x: float) -> int:
    return math.ceil(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.ceil()") || result.contains("f64::ceil"));
}

#[test]
fn test_math_floor() {
    let python = r#"
import math

def calculate_floor(x: float) -> int:
    return math.floor(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.floor()") || result.contains("f64::floor"));
}

#[test]
fn test_math_trunc() {
    let python = r#"
import math

def calculate_trunc(x: float) -> int:
    return math.trunc(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.trunc()") || result.contains("f64::trunc"));
}

//
#[test]
fn test_math_pi() {
    let python = r#"
import math

def get_pi() -> float:
    return math.pi
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("std::f64::consts::PI") || result.contains("PI"));
}

#[test]
fn test_math_e() {
    let python = r#"
import math

def get_e() -> float:
    return math.e
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("std::f64::consts::E") || result.contains("::E"));
}

#[test]
fn test_math_tau() {
    let python = r#"
import math

def get_tau() -> float:
    return math.tau
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("std::f64::consts::TAU") || result.contains("TAU"));
}

//
#[test]
fn test_math_sinh() {
    let python = r#"
import math

def calculate_sinh(x: float) -> float:
    return math.sinh(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.sinh()") || result.contains("f64::sinh"));
}

#[test]
fn test_math_cosh() {
    let python = r#"
import math

def calculate_cosh(x: float) -> float:
    return math.cosh(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.cosh()") || result.contains("f64::cosh"));
}

#[test]
fn test_math_tanh() {
    let python = r#"
import math

def calculate_tanh(x: float) -> float:
    return math.tanh(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.tanh()") || result.contains("f64::tanh"));
}

//
#[test]
fn test_math_fabs() {
    let python = r#"
import math

def calculate_fabs(x: float) -> float:
    return math.fabs(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.abs()") || result.contains("f64::abs"));
}

#[test]
fn test_math_copysign() {
    let python = r#"
import math

def calculate_copysign(x: float, y: float) -> float:
    return math.copysign(x, y)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("copysign"));
}

//
#[test]
fn test_math_factorial() {
    let python = r#"
import math

def calculate_factorial(n: int) -> int:
    return math.factorial(n)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("factorial"));
}

//
#[test]
fn test_math_degrees() {
    let python = r#"
import math

def convert_to_degrees(x: float) -> float:
    return math.degrees(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("to_degrees()") || result.contains("degrees"));
}

#[test]
fn test_math_radians() {
    let python = r#"
import math

def convert_to_radians(x: float) -> float:
    return math.radians(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("to_radians()") || result.contains("radians"));
}

//
#[test]
fn test_math_gcd() {
    let python = r#"
import math

def calculate_gcd(a: int, b: int) -> int:
    return math.gcd(a, b)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("gcd") || result.contains("num::integer"));
}

//
#[test]
fn test_math_inf() {
    let python = r#"
import math

def get_inf() -> float:
    return math.inf
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("f64::INFINITY") || result.contains("INFINITY"));
}

#[test]
fn test_math_nan() {
    let python = r#"
import math

def get_nan() -> float:
    return math.nan
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("f64::NAN") || result.contains("::NAN"));
}

#[test]
fn test_math_isnan() {
    let python = r#"
import math

def check_nan(x: float) -> bool:
    return math.isnan(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.is_nan()") || result.contains("f64::is_nan"));
}

#[test]
fn test_math_isinf() {
    let python = r#"
import math

def check_inf(x: float) -> bool:
    return math.isinf(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.is_infinite()") || result.contains("f64::is_infinite"));
}

#[test]
fn test_math_isfinite() {
    let python = r#"
import math

def check_finite(x: float) -> bool:
    return math.isfinite(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.is_finite()") || result.contains("f64::is_finite"));
}

//
#[test]
fn test_math_combined_operations() {
    let python = r#"
import math

def calculate_distance(x1: float, y1: float, x2: float, y2: float) -> float:
    dx = x2 - x1
    dy = y2 - y1
    return math.sqrt(dx * dx + dy * dy)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("sqrt"));
}

#[test]
fn test_math_circle_area() {
    let python = r#"
import math

def circle_area(radius: float) -> float:
    return math.pi * radius * radius
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("PI"));
}

//
#[test]
fn test_math_asin() {
    let python = r#"
import math

def calculate_asin(x: float) -> float:
    return math.asin(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.asin()") || result.contains("f64::asin"));
}

#[test]
fn test_math_acos() {
    let python = r#"
import math

def calculate_acos(x: float) -> float:
    return math.acos(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.acos()") || result.contains("f64::acos"));
}

#[test]
fn test_math_atan() {
    let python = r#"
import math

def calculate_atan(x: float) -> float:
    return math.atan(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("x.atan()") || result.contains("f64::atan"));
}

#[test]
fn test_math_atan2() {
    let python = r#"
import math

def calculate_atan2(y: float, x: float) -> float:
    return math.atan2(y, x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("atan2"));
}

//
#[test]
fn test_math_ldexp() {
    let python = r#"
import math

def calculate_ldexp(x: float, i: int) -> float:
    return math.ldexp(x, i)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("ldexp"));
}

#[test]
fn test_math_frexp() {
    let python = r#"
import math

def calculate_frexp(x: float) -> tuple[float, int]:
    return math.frexp(x)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("frexp"));
}

#[test]
fn test_nested_min_max_with_division() {
    let python = r#"
def clamp_half(x: int) -> float:
    return float(max(min(x / 2.0, 10.0), -10.0))
"#;

    let result = transpile_and_check(python, &[]);
    println!("{}", result);
    assert!(result.contains("(f64::max(f64::min((x as f64) / 2.0, 10.0), -10.0)) as f64"));
}

#[test]
fn test_float_conversion_of_dataclass_field_sum() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    val1: int
    val2: int

def compute(state: State):
    x = float(state.val1 + state.val2)
"#;

    let result = transpile_and_check(python, &[]);
    // The transpiler may optimize with CSE, so check for either form
    assert!(
        result.contains("(state.val1 + state.val2) as f64")
            || (result.contains("state.val1 + state.val2") && result.contains("as f64")),
        "Expected integer addition followed by float conversion, got:\n{}",
        result
    );
    // Ensure no string concatenation (format!) is used
    assert!(!result.contains("format!"), "Should not generate string concatenation");
}
