
// Module: math - Additional math functions  
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: degrees, radians, gcd, copysign were pre-existing
// New in this commit: lcm, isclose, modf, fmod, hypot, dist

#[test]
fn test_lcm() {
    let python = r#"
import math

def least_common_multiple(a: int, b: int) -> int:
    return math.lcm(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute LCM
    assert!(result.contains("gcd") || result.contains("%"));
}

#[test]
fn test_isclose() {
    let python = r#"
import math

def are_close(a: float, b: float) -> bool:
    return math.isclose(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    // Should check if floats are close
    assert!(result.contains("abs") && result.contains("rel_tol"));
}

#[test]
fn test_modf() {
    let python = r#"
import math

def split_parts(x: float) -> tuple:
    return math.modf(x)
"#;

    let result = transpile_and_check(python, &[]);

    // Should split into fractional and integer parts
    assert!(result.contains("trunc") && result.contains("frac_part"));
}

#[test]
fn test_fmod() {
    let python = r#"
import math

def float_remainder(x: float, y: float) -> float:
    return math.fmod(x, y)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute floating point remainder
    assert!(result.contains("%"));
}

#[test]
fn test_hypot() {
    let python = r#"
import math

def hypotenuse(x: float, y: float) -> float:
    return math.hypot(x, y)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute hypotenuse
    assert!(result.contains("hypot"));
}

#[test]
fn test_dist() {
    let python = r#"
import math

def distance(p: list, q: list) -> float:
    return math.dist(p, q)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute distance between points
    assert!(result.contains("sqrt") && (result.contains("dx") || result.contains("dy")));
}

// Total: 6 NEW math functions
// Coverage: lcm(), isclose(), modf(), fmod(), hypot(), dist()
