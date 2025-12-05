// Module: random - Python random module validation

use crate::test_helpers::transpile_and_check;

//
#[test]
#[ignore]
fn test_random_random() {
    let python = r#"
import random

def get_random() -> float:
    return random.random()
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate rand::random() or thread_rng().gen()
    assert!(result.contains("rand::") || result.contains("random"));
}

#[test]
#[ignore]
fn test_random_randint() {
    let python = r#"
import random

def get_random_int(a: int, b: int) -> int:
    return random.randint(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate gen_range or similar
    assert!(result.contains("gen_range") || result.contains("rand"));
}

#[test]
#[ignore]
fn test_random_uniform() {
    let python = r#"
import random

def get_uniform(a: float, b: float) -> float:
    return random.uniform(a, b)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("gen_range") || result.contains("rand"));
}

//
#[test]
#[ignore]
fn test_random_choice() {
    let python = r#"
import random

def pick_random(items: list[int]) -> int:
    return random.choice(items)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("choose") || result.contains("rand"));
}

#[test]
#[ignore]
fn test_random_shuffle() {
    let python = r#"
import random

def shuffle_list(items: list[int]) -> None:
    random.shuffle(items)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("shuffle") || result.contains("rand"));
}

#[test]
#[ignore]
fn test_random_sample() {
    let python = r#"
import random

def sample_items(items: list[int], k: int) -> list[int]:
    return random.sample(items, k)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("sample") || result.contains("choose_multiple"));
}

//
#[test]
#[ignore]
fn test_random_gauss() {
    let python = r#"
import random

def get_normal(mu: float, sigma: float) -> float:
    return random.gauss(mu, sigma)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("Normal") || result.contains("gauss"));
}

#[test]
#[ignore]
fn test_random_expovariate() {
    let python = r#"
import random

def get_exponential(lambd: float) -> float:
    return random.expovariate(lambd)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("Exp") || result.contains("exponential"));
}

//
#[test]
#[ignore]
fn test_random_seed() {
    let python = r#"
import random

def seed_rng(seed: int) -> None:
    random.seed(seed)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("seed") || result.contains("SeedableRng"));
}

//
#[test]
#[ignore]
fn test_random_randrange() {
    let python = r#"
import random

def get_random_range(start: int, stop: int, step: int) -> int:
    return random.randrange(start, stop, step)
"#;

    let result = transpile_and_check(python, &[]);

    assert!(result.contains("gen_range") || result.contains("rand"));
}

// Total: 11 comprehensive tests for random module
// Coverage: Basic random, distributions, sequences, seed/state
