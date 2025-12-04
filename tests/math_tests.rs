mod test_helpers;

use test_helpers::transpile_and_check;

// ============================================================================
// Absolute Value Tests
// ============================================================================

#[test]
fn test_abs_variable() {
    let python = r#"
def test_abs(value: int) -> int:
    return abs(value)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("value.abs()"));
}

#[test]
fn test_abs_negative_literal() {
    let python = r#"
def abs_negative() -> int:
    return abs(-42)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(
        rust.contains("((-42_i32).abs())") || rust.contains("(-42_i32).abs()") || rust.contains("(-42 as i32).abs()")
    );
}

#[test]
fn test_abs_float() {
    let python = r#"
def abs_float(x: float) -> float:
    return abs(x)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x.abs()"));
}

#[test]
fn test_abs_expression() {
    let python = r#"
def abs_expr(a: int, b: int) -> int:
    return abs(a - b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(a - b).abs()"));
}

#[test]
fn test_abs_positive_literal() {
    let python = r#"
def abs_positive() -> int:
    return abs(42)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(42 as i32).abs()"));
}

#[test]
fn test_abs_float_literal() {
    let python = r#"
def abs_float_lit() -> float:
    return abs(-3.14)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(-3.14_f64).abs()") || rust.contains("(-3.14 as f64).abs()"));
}

#[test]
fn test_abs_float_positive_literal() {
    let python = r#"
def abs_float_pos() -> float:
    return abs(2.718)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(2.718 as f64).abs()"));
}

// ============================================================================
// Rounding Tests
// ============================================================================

#[test]
fn test_round_to_int() {
    let python = r#"
def round_to_int(x: float) -> int:
    return round(x)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x.round() as i32"));
}

#[test]
fn test_round_to_float() {
    let python = r#"
def round_to_float(x: float) -> float:
    return round(x)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x.round()"));
}

#[test]
fn test_round_literal() {
    let python = r#"
def round_literal() -> int:
    return round(3.7)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(3.7 as f64).round() as i32"));
}

// ============================================================================
// Power/Exponentiation Tests
// ============================================================================

#[test]
fn test_pow_literal_exponent() {
    let python = r#"
def pow_literal() -> int:
    return pow(2, 10)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("2_i32.pow(10 as u32)"));
}

#[test]
fn test_pow_variable_exponent() {
    let python = r#"
def pow_var(base: int, exp: int) -> int:
    return pow(base, exp)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("base.checked_pow(exp as u32)"));
}

#[test]
fn test_power_operator_literal() {
    let python = r#"
def power_op() -> int:
    return 2 ** 8
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("2_i32.pow(8 as u32)"));
}

#[test]
fn test_power_operator_variable() {
    let python = r#"
def power_op(a: int, b: int) -> int:
    return a ** b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a.checked_pow(b as u32)"));
}

#[test]
fn test_pow_variable_base_constant_exp() {
    let python = r#"
def pow_var_base(base: int) -> int:
    return pow(base, 3)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("base.pow(3 as u32)"));
}

#[test]
fn test_pow_constant_base_variable_exp() {
    let python = r#"
def pow_const_base(exp: int) -> int:
    return pow(2, exp)
"#;

    let rust = transpile_and_check(python, &[]);
    // When base is constant but exp is variable, need checked_pow with type suffix
    assert!(rust.contains("2_i32.checked_pow(exp as u32)"));
}

#[test]
fn test_power_operator_variable_base_constant_exp() {
    let python = r#"
def power_op_var_base(x: int) -> int:
    return x ** 4
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x.pow(4 as u32)"));
}

#[test]
fn test_power_operator_constant_base_variable_exp() {
    let python = r#"
def power_op_const_base(n: int) -> int:
    return 3 ** n
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("3_i32.checked_pow(n as u32)"));
}

#[test]
fn test_pow_float() {
    let python = r#"
def pow_float(base: float, exp: float) -> float:
    return pow(base, exp)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("base.powf(exp)"));
}

// ============================================================================
// Min/Max Tests - Two Arguments
// ============================================================================

#[test]
fn test_max_two_ints() {
    let python = r#"
def max_two(a: int, b: int) -> int:
    return max(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::max(a, b)"));
}

#[test]
fn test_min_two_ints() {
    let python = r#"
def min_two(a: int, b: int) -> int:
    return min(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::min(a, b)"));
}

#[test]
fn test_max_two_floats() {
    let python = r#"
def max_two_f(a: float, b: float) -> float:
    return max(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("f64::max(a, b)"));
}

#[test]
fn test_min_two_floats() {
    let python = r#"
def min_two_f(a: float, b: float) -> float:
    return min(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("f64::min(a, b)"));
}

// ============================================================================
// Min/Max Tests - Multiple Arguments
// ============================================================================

#[test]
fn test_max_three_ints() {
    let python = r#"
def max_three() -> int:
    return max(10, 20, 5)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::max(std::cmp::max(10, 20), 5)"));
}

#[test]
fn test_min_three_floats() {
    let python = r#"
def min_three() -> float:
    return min(1.5, 2.7, 0.8)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("f64::min(f64::min(1.5, 2.7), 0.8)"));
}

#[test]
fn test_max_four_values() {
    let python = r#"
def max_four(a: int, b: int, c: int, d: int) -> int:
    return max(a, b, c, d)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::max(std::cmp::max(std::cmp::max(a, b), c), d)"));
}

// ============================================================================
// Min/Max Tests - Collections
// ============================================================================

#[test]
fn test_max_list() {
    let python = r#"
def max_list(items: list[int]) -> int:
    return max(items)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("items.iter().max().unwrap()"));
}

#[test]
fn test_min_list() {
    let python = r#"
def min_list(items: list[int]) -> int:
    return min(items)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("items.iter().min().unwrap()"));
}

#[test]
fn test_max_float_list() {
    let python = r#"
def max_floats(items: list[float]) -> float:
    return max(items)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("items.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))"));
}

#[test]
fn test_min_float_list() {
    let python = r#"
def min_floats(items: list[float]) -> float:
    return min(items)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("items.iter().fold(f64::INFINITY, |a, &b| a.min(b))"));
}

#[test]
fn test_max_with_computed_expression() {
    let python = r#"
def compute() -> float:
    a = 1
    b = 2
    c = a - b * 4
    return max(1.0, 0.3557 * c - 1.75)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("f64::max(1.0, 0.3557 * (c as f64) - 1.75)"));
}

// ============================================================================
// First/Last Element Tests
// ============================================================================

#[test]
fn test_first_element() {
    let python = r#"
def first(items: list[int]) -> int:
    return items[0]
"#;

    let rust = transpile_and_check(python, &[]);
    // Should use .get(0) or .get(0usize) with .cloned() for both Copy and non-Copy types
    assert!((rust.contains(".get(0)") || rust.contains(".get(0usize)")) && rust.contains(".cloned()"));
}

#[test]
fn test_last_element() {
    let python = r#"
def last(items: list[int]) -> int:
    return items[-1]
"#;

    let rust = transpile_and_check(python, &[]);
    // Should use .last().cloned() for both Copy and non-Copy types
    assert!(rust.contains(".last()") && rust.contains(".cloned()"));
}

#[test]
fn test_negative_index_second_last() {
    let python = r#"
def second_last(items: list[int]) -> int:
    return items[-2]
"#;

    let rust = transpile_and_check(python, &[]);
    // Should use .get() with saturating_sub for safety
    assert!(
        rust.contains(".get(") && (rust.contains("saturating_sub(2)") || rust.contains(".len()") && rust.contains("2"))
    );
}

#[test]
fn test_negative_index_third_last() {
    let python = r#"
def third_last(items: list[float]) -> float:
    return items[-3]
"#;

    let rust = transpile_and_check(python, &[]);
    // Should use .get() with saturating_sub for safety
    assert!(
        rust.contains(".get(") && (rust.contains("saturating_sub(3)") || rust.contains(".len()") && rust.contains("3"))
    );
}

// ============================================================================
// Random Tests
// ============================================================================

#[test]
fn test_randint() {
    let python = r#"
import random

def rand_int(start: int, end: int) -> int:
    return random.randint(start, end)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("rand::thread_rng().gen_range(start..=end)"));
}

#[test]
fn test_randint_literal() {
    let python = r#"
import random

def rand_int() -> int:
    return random.randint(1, 100)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("rand::thread_rng().gen_range(1..=100)"));
}

#[test]
fn test_uniform() {
    let python = r#"
import random

def rand_float(a: float, b: float) -> float:
    return random.uniform(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("rand::thread_rng().gen_range((a as f64)..=(b as f64))"));
}

#[test]
fn test_random_choice() {
    let python = r#"
import random

def rand_choice(items: list[int]) -> int:
    return random.choice(items)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("*items.choose(&mut rand::thread_rng()).unwrap()"));
}

#[test]
fn test_random_random() {
    let python = r#"
import random

def rand() -> float:
    return random.random()
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("rand::random::<f64>()"));
}

// ============================================================================
// Combined Math Operations
// ============================================================================

#[test]
fn test_abs_and_max() {
    let python = r#"
def abs_max(x: int, y: int) -> int:
    return max(abs(x), abs(y))
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::max(x.abs(), y.abs())"));
}

#[test]
fn test_nested_round_abs() {
    let python = r#"
def nested(x: float) -> int:
    return abs(round(x))
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(x.round() as i32).abs()"));
}

#[test]
fn test_power_sum() {
    let python = r#"
def pythagorean(a: int, b: int) -> int:
    return a ** 2 + b ** 2
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a.pow(2 as u32) + b.pow(2 as u32)"));
}

#[test]
fn test_clamp_pattern() {
    let python = r#"
def clamp(x: int, lo: int, hi: int) -> int:
    return max(lo, min(x, hi))
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("std::cmp::max(lo, std::cmp::min(x, hi))"));
}

#[test]
fn test_floor_division() {
    let python = r#"
def floor_div(a: int, b: int) -> int:
    return a // b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a / b"));
}

#[test]
fn test_float_division() {
    let python = r#"
def float_div(a: float, b: float) -> float:
    return a / b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a / b"));
}

#[test]
fn test_modulo() {
    let python = r#"
def mod_op(a: int, b: int) -> int:
    return a % b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a % b"));
}

#[test]
fn test_complex_expression() {
    let python = r#"
def complex_expr(a: int, b: int) -> int:
    return abs(a - b) ** 2 + max(a, b)
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("(a - b).abs().pow(2 as u32) + std::cmp::max(a, b)"));
}

#[test]
fn test_arithmetic_precedence() {
    let python = r#"
def precedence(a: int, b: int, c: int) -> int:
    return a + b * c
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a + b * c"));
}

// ============================================================================
// Points Calculation Tests
// ============================================================================

#[test]
fn test_calculate_simple_multiplication_division() {
    let python = r#"
def calculate(points: int) -> float:
    return 0.18 * points / 10.0
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for simple multiplication/division:\n{}", rust);
    assert!(rust.contains("0.18 * (points as f64) / 10.0"));
}

#[test]
fn test_calculate_exp_expression() {
    let python = r#"
import math
def calculate(points: int) -> float:
    return math.exp(0.18 * points / 10.0)
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for math.exp expression:\n{}", rust);
    assert!(rust.contains(".exp()"));
}

#[test]
fn test_calculate_max_with_subtraction() {
    let python = r#"
def calculate(points: float) -> float:
    return max(2.5 - points / 10.0, 0.5)
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for max with subtraction:\n{}", rust);
    assert!(rust.contains("f64::max(2.5 - points / 10.0, 0.5)"));
}

#[test]
fn test_calculate_min_with_division() {
    let python = r#"
def calculate(points: float) -> float:
    return min(points / 2.0, 10.0)
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for min with division:\n{}", rust);
    assert!(rust.contains("f64::min(points / 2.0, 10.0)"));
}

// ============================================================================
// Mixed Float/Int Arithmetic Tests
// ============================================================================

#[test]
fn test_float_times_int_literal() {
    let python = r#"
def multiply(x: float) -> float:
    return x * 3
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for float * int literal:\n{}", rust);
    assert!(rust.contains("x * (3 as f64)"));
}

#[test]
fn test_int_literal_times_float() {
    let python = r#"
def multiply(x: float) -> float:
    return 3 * x
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for int literal * float:\n{}", rust);
    assert!(rust.contains("(3 as f64) * x"));
}

#[test]
fn test_float_literal_times_int_var() {
    let python = r#"
def multiply(n: int) -> float:
    return 6.0 * n
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for float literal * int var:\n{}", rust);
    assert!(rust.contains("6.0 * (n as f64)"));
}

#[test]
fn test_int_var_times_float_literal() {
    let python = r#"
def multiply(n: int) -> float:
    return n * 2.5
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for int var * float literal:\n{}", rust);
    assert!(rust.contains("(n as f64) * 2.5"));
}

#[test]
fn test_mixed_int_float_chain() {
    let python = r#"
def calculate(a: int, b: float) -> float:
    return a * b * 2.0
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for mixed chain:\n{}", rust);
    assert!(rust.contains("(a as f64) * b"));
}

#[test]
fn test_float_var_times_int_var() {
    let python = r#"
def multiply(x: float, n: int) -> float:
    return x * n
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for float var * int var:\n{}", rust);
    assert!(rust.contains("x * (n as f64)"));
}

#[test]
fn test_int_var_times_float_var() {
    let python = r#"
def multiply(n: int, x: float) -> float:
    return n * x
"#;

    let rust = transpile_and_check(python, &[]);
    println!("Rust code for int var * float var:\n{}", rust);
    assert!(rust.contains("(n as f64) * x"));
}

// ============================================================================
// Comparison Operators Tests
// ============================================================================

#[test]
fn test_less_than_int_literal() {
    let python = r#"
def is_small(x: int) -> bool:
    return x < 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x < 1"));
}

#[test]
fn test_less_than_negative_literal() {
    let python = r#"
def is_very_negative(x: int) -> bool:
    return x < -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x < -1"));
}

#[test]
fn test_less_than_float_literal() {
    let python = r#"
def below_threshold(x: float) -> bool:
    return x < 0.5
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x < 0.5"));
}

#[test]
fn test_less_than_variable() {
    let python = r#"
def compare_vars(a: int, b: int) -> bool:
    return a < b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a < b"));
}

#[test]
fn test_less_equal_int_literal() {
    let python = r#"
def at_most_one(x: int) -> bool:
    return x <= 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x <= 1"));
}

#[test]
fn test_less_equal_negative_literal() {
    let python = r#"
def at_most_negative(x: int) -> bool:
    return x <= -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x <= -1"));
}

#[test]
fn test_less_equal_float_literal() {
    let python = r#"
def at_most_half(x: float) -> bool:
    return x <= 0.5
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x <= 0.5"));
}

#[test]
fn test_less_equal_variable() {
    let python = r#"
def compare_le(a: int, b: int) -> bool:
    return a <= b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a <= b"));
}

#[test]
fn test_greater_than_int_literal() {
    let python = r#"
def is_large(x: int) -> bool:
    return x > 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x > 1"));
}

#[test]
fn test_greater_than_negative_literal() {
    let python = r#"
def above_negative(x: int) -> bool:
    return x > -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x > -1"));
}

#[test]
fn test_greater_than_float_literal() {
    let python = r#"
def above_threshold(x: float) -> bool:
    return x > 0.5
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x > 0.5"));
}

#[test]
fn test_greater_than_variable() {
    let python = r#"
def compare_gt(a: int, b: int) -> bool:
    return a > b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a > b"));
}

#[test]
fn test_greater_equal_int_literal() {
    let python = r#"
def at_least_one(x: int) -> bool:
    return x >= 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x >= 1"));
}

#[test]
fn test_greater_equal_negative_literal() {
    let python = r#"
def at_least_negative(x: int) -> bool:
    return x >= -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x >= -1"));
}

#[test]
fn test_greater_equal_float_literal() {
    let python = r#"
def at_least_half(x: float) -> bool:
    return x >= 0.5
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x >= 0.5"));
}

#[test]
fn test_greater_equal_variable() {
    let python = r#"
def compare_ge(a: int, b: int) -> bool:
    return a >= b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a >= b"));
}

#[test]
fn test_equal_int_literal() {
    let python = r#"
def is_one(x: int) -> bool:
    return x == 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x == 1"));
}

#[test]
fn test_equal_negative_literal() {
    let python = r#"
def is_negative_one(x: int) -> bool:
    return x == -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x == -1"));
}

#[test]
fn test_equal_float_literal() {
    let python = r#"
def is_half(x: float) -> bool:
    return x == 0.5
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x == 0.5"));
}

#[test]
fn test_equal_variable() {
    let python = r#"
def are_equal(a: int, b: int) -> bool:
    return a == b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a == b"));
}

#[test]
fn test_not_equal_int_literal() {
    let python = r#"
def is_not_one(x: int) -> bool:
    return x != 1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x != 1"));
}

#[test]
fn test_not_equal_negative_literal() {
    let python = r#"
def is_not_negative_one(x: int) -> bool:
    return x != -1
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x != -1"));
}

#[test]
fn test_not_equal_variable() {
    let python = r#"
def are_not_equal(a: int, b: int) -> bool:
    return a != b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a != b"));
}

#[test]
fn test_compare_float_variables() {
    let python = r#"
def compare_floats(a: float, b: float) -> bool:
    return a < b
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a < b"));
}

#[test]
fn test_compare_expression_to_literal() {
    let python = r#"
def check_sum(a: int, b: int) -> bool:
    return a + b > 10
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("a + b > 10"));
}

#[test]
fn test_compare_literal_to_variable() {
    let python = r#"
def literal_first(x: int) -> bool:
    return 5 < x
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("5 < x"));
}

#[test]
fn test_compare_zero() {
    let python = r#"
def is_positive(x: int) -> bool:
    return x > 0
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x > 0"));
}

#[test]
fn test_compare_negative_zero_float() {
    let python = r#"
def is_non_negative(x: float) -> bool:
    return x >= 0.0
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("x >= 0.0"));
}

// ============================================================================
// Top-Level Constant Tests
// ============================================================================

#[test]
fn test_top_level_constant_with_arithmetic() {
    let python = r#"
TWO = 52.0

def compute() -> float:
    one = 6 * 42
    three = one + TWO
    return three
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains("const TWO: f64 = 52.0"));
    // one = 6 * 42 may be constant-folded to 252
    assert!(rust.contains("6 * 42") || rust.contains("252"));
    assert!(rust.contains("(one as f64) + TWO"));
}

#[test]
fn test_float_sum_of_function_result() {
    let python = r#"
def get_floats() -> list[float]:
    return [1.0, 2.0, 3.0]

def compute() -> None:
    val = float(sum(get_floats()))
"#;

    transpile_and_check(python, &[]);
}
