// Module: math - Additional math functions (batch 2)
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: factorial() was already implemented
// This commit adds 4 NEW functions: remainder, comb, perm, expm1

//
#[test]
fn test_remainder() {
    let python = r#"
import math

def get_remainder(x: float, y: float) -> float:
    return math.remainder(x, y)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute IEEE remainder
    assert!(result.contains("round"));
}

//
#[test]
fn test_factorial() {
    let python = r#"
import math

def get_factorial(n: int) -> int:
    return math.factorial(n)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute factorial
    assert!(result.contains("for i in") || result.contains("result"));
}

//
#[test]
fn test_comb() {
    let python = r#"
import math

def combinations(n: int, k: int) -> int:
    return math.comb(n, k)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute combinations
    assert!(result.contains("result") && result.contains("for i in"));
}

//
#[test]
fn test_perm() {
    let python = r#"
import math

def permutations(n: int, k: int) -> int:
    return math.perm(n, k)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute permutations
    assert!(result.contains("result") && result.contains("for i in"));
}

//
#[test]
fn test_expm1() {
    let python = r#"
import math

def exp_minus_one(x: float) -> float:
    return math.expm1(x)
"#;

    let result = transpile_and_check(python, &[]);

    // Should compute exp(x) - 1
    assert!(result.contains("exp_m1"));
}

// Total: 4 NEW math functions (factorial already existed)
// Coverage: remainder(), comb(), perm(), expm1()
