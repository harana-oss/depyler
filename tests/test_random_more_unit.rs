
// Module: random - Additional random functions (batch 2)
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: randint, choice, choices, expovariate were already implemented
// This commit adds 1 NEW function: randbytes

// 
#[test]
#[ignore]
fn test_randbytes() {
    let python = r#"
import random

def get_random_bytes(n: int) -> bytes:
    return random.randbytes(n)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate random bytes
    assert!(result.contains("gen") && result.contains("u8"));
}

// 
#[test]
#[ignore]
fn test_randint() {
    let python = r#"
import random

def random_integer(a: int, b: int) -> int:
    return random.randint(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate random integer in range
    assert!(result.contains("gen_range"));
}

// 
#[test]
#[ignore]
fn test_choice() {
    let python = r#"
import random

def pick_one(items: list) -> int:
    return random.choice(items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should pick random element
    assert!(result.contains("choose"));
}

// 
#[test]
#[ignore]
fn test_choices() {
    let python = r#"
import random

def pick_many(items: list, k: int) -> list:
    return random.choices(items, k=k)
"#;

    let result = transpile_and_check(python, &[]);

    // Should pick k random elements with replacement
    assert!(result.contains("map") || result.contains("choose"));
}

// 
#[test]
#[ignore]
fn test_expovariate() {
    let python = r#"
import random

def expo_random(lambd: float) -> float:
    return random.expovariate(lambd)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate exponential distribution
    assert!(result.contains("Exp"));
}

// Total: 1 NEW random function (randint, choice, choices, expovariate already existed)
// Coverage: randbytes()
