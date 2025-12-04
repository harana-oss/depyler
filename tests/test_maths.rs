use crate::test_helpers::transpile;
use crate::test_helpers::transpile_and_check;

#[test]
fn test_abs_variants() {
    let cases = [
        ("def test_abs(value: int) -> int:\n    return abs(value)", "value.abs()"),
        ("def abs_float(x: float) -> float:\n    return abs(x)", "x.abs()"),
        (
            "def abs_expr(a: int, b: int) -> int:\n    return abs(a - b)",
            "(a - b).abs()",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_abs_literals() {
    let cases = [
        (
            "def abs_negative() -> int:\n    return abs(-42)",
            vec!["(-42_i32).abs()", "(-42 as i32).abs()"],
        ),
        (
            "def abs_positive() -> int:\n    return abs(42)",
            vec!["(42 as i32).abs()"],
        ),
        (
            "def abs_float_lit() -> float:\n    return abs(-3.14)",
            vec!["(-3.14_f64).abs()", "(-3.14 as f64).abs()"],
        ),
        (
            "def abs_float_pos() -> float:\n    return abs(2.718)",
            vec!["(2.718 as f64).abs()"],
        ),
    ];

    for (python, expected_options) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(
            expected_options.iter().any(|e| rust.contains(e)),
            "expected one of {:?} in:\n{rust}",
            expected_options
        );
    }
}

#[test]
fn test_round() {
    let cases = [
        (
            "def round_to_int(x: float) -> int:\n    return round(x)",
            "x.round() as i32",
        ),
        (
            "def round_to_float(x: float) -> float:\n    return round(x)",
            "x.round()",
        ),
        (
            "def round_literal() -> int:\n    return round(3.7)",
            "(3.7 as f64).round() as i32",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_pow_variants() {
    let cases = [
        (
            "def pow_literal() -> int:\n    return pow(2, 10)",
            "2_i32.pow(10 as u32)",
        ),
        (
            "def pow_var(base: int, exp: int) -> int:\n    return pow(base, exp)",
            "base.checked_pow(exp as u32)",
        ),
        ("def power_op() -> int:\n    return 2 ** 8", "2_i32.pow(8 as u32)"),
        (
            "def power_op(a: int, b: int) -> int:\n    return a ** b",
            "a.checked_pow(b as u32)",
        ),
        (
            "def pow_var_base(base: int) -> int:\n    return pow(base, 3)",
            "base.pow(3 as u32)",
        ),
        (
            "def pow_const_base(exp: int) -> int:\n    return pow(2, exp)",
            "2_i32.checked_pow(exp as u32)",
        ),
        (
            "def power_op_var_base(x: int) -> int:\n    return x ** 4",
            "x.pow(4 as u32)",
        ),
        (
            "def power_op_const_base(n: int) -> int:\n    return 3 ** n",
            "3_i32.checked_pow(n as u32)",
        ),
        (
            "def pow_float(base: float, exp: float) -> float:\n    return pow(base, exp)",
            "base.powf(exp)",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_min_max_two_args() {
    let cases = [
        (
            "def max_two(a: int, b: int) -> int:\n    return max(a, b)",
            "std::cmp::max(a, b)",
        ),
        (
            "def min_two(a: int, b: int) -> int:\n    return min(a, b)",
            "std::cmp::min(a, b)",
        ),
        (
            "def max_two_f(a: float, b: float) -> float:\n    return max(a, b)",
            "f64::max(a, b)",
        ),
        (
            "def min_two_f(a: float, b: float) -> float:\n    return min(a, b)",
            "f64::min(a, b)",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_min_max_multiple_args() {
    let cases = [
        (
            "def max_three() -> int:\n    return max(10, 20, 5)",
            "std::cmp::max(std::cmp::max(10, 20), 5)",
        ),
        (
            "def min_three() -> float:\n    return min(1.5, 2.7, 0.8)",
            "f64::min(f64::min(1.5, 2.7), 0.8)",
        ),
        (
            "def max_four(a: int, b: int, c: int, d: int) -> int:\n    return max(a, b, c, d)",
            "std::cmp::max(std::cmp::max(std::cmp::max(a, b), c), d)",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_min_max_collections() {
    let cases = [
        (
            "def max_list(items: list[int]) -> int:\n    return max(items)",
            "items.iter().max().unwrap()",
        ),
        (
            "def min_list(items: list[int]) -> int:\n    return min(items)",
            "items.iter().min().unwrap()",
        ),
        (
            "def max_floats(items: list[float]) -> float:\n    return max(items)",
            "items.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))",
        ),
        (
            "def min_floats(items: list[float]) -> float:\n    return min(items)",
            "items.iter().fold(f64::INFINITY, |a, &b| a.min(b))",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
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

#[test]
fn test_indexing() {
    // First element
    let python = "def first(items: list[int]) -> int:\n    return items[0]";
    let rust = transpile_and_check(python, &[]);
    assert!((rust.contains(".get(0)") || rust.contains(".get(0usize)")) && rust.contains(".cloned()"));

    // Last element
    let python = "def last(items: list[int]) -> int:\n    return items[-1]";
    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(".last()") && rust.contains(".cloned()"));

    // Negative indices
    let python = "def second_last(items: list[int]) -> int:\n    return items[-2]";
    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(".get(") && (rust.contains("saturating_sub(2)") || rust.contains(".len()")));

    let python = "def third_last(items: list[float]) -> float:\n    return items[-3]";
    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(".get(") && (rust.contains("saturating_sub(3)") || rust.contains(".len()")));
}

#[test]
fn test_random_functions() {
    // Uses transpile instead of transpile_and_check because rand crate isn't available in test env
    let cases = [
        (
            "import random\n\ndef rand_int(start: int, end: int) -> int:\n    return random.randint(start, end)",
            "rand::thread_rng().gen_range(start..=end)",
        ),
        (
            "import random\n\ndef rand_int() -> int:\n    return random.randint(1, 100)",
            "rand::thread_rng().gen_range(1..=100)",
        ),
        (
            "import random\n\ndef rand_float(a: float, b: float) -> float:\n    return random.uniform(a, b)",
            "rand::thread_rng().gen_range((a as f64)..=(b as f64))",
        ),
        (
            "import random\n\ndef rand_choice(items: list[int]) -> int:\n    return random.choice(items)",
            "*items.choose(&mut rand::thread_rng()).unwrap()",
        ),
        (
            "import random\n\ndef rand() -> float:\n    return random.random()",
            "rand::random::<f64>()",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile(python);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_combined_math_operations() {
    let cases = [
        (
            "def abs_max(x: int, y: int) -> int:\n    return max(abs(x), abs(y))",
            "std::cmp::max(x.abs(), y.abs())",
        ),
        (
            "def nested(x: float) -> int:\n    return abs(round(x))",
            "(x.round() as i32).abs()",
        ),
        (
            "def pythagorean(a: int, b: int) -> int:\n    return a ** 2 + b ** 2",
            "a.pow(2 as u32) + b.pow(2 as u32)",
        ),
        (
            "def clamp(x: int, lo: int, hi: int) -> int:\n    return max(lo, min(x, hi))",
            "std::cmp::max(lo, std::cmp::min(x, hi))",
        ),
        ("def floor_div(a: int, b: int) -> int:\n    return a // b", "a / b"),
        ("def float_div(a: float, b: float) -> float:\n    return a / b", "a / b"),
        ("def mod_op(a: int, b: int) -> int:\n    return a % b", "a % b"),
        (
            "def complex_expr(a: int, b: int) -> int:\n    return abs(a - b) ** 2 + max(a, b)",
            "(a - b).abs().pow(2 as u32) + std::cmp::max(a, b)",
        ),
        (
            "def precedence(a: int, b: int, c: int) -> int:\n    return a + b * c",
            "a + b * c",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_points_calculations() {
    let cases = [
        (
            "def calculate(points: int) -> float:\n    return 0.18 * points / 10.0",
            "0.18 * (points as f64) / 10.0",
        ),
        (
            "def calculate(points: float) -> float:\n    return max(2.5 - points / 10.0, 0.5)",
            "f64::max(2.5 - points / 10.0, 0.5)",
        ),
        (
            "def calculate(points: float) -> float:\n    return min(points / 2.0, 10.0)",
            "f64::min(points / 2.0, 10.0)",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_math_exp() {
    let python = "import math\ndef calculate(points: int) -> float:\n    return math.exp(0.18 * points / 10.0)";
    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(".exp()"));
}

#[test]
fn test_mixed_float_int_arithmetic() {
    let cases = [
        ("def multiply(x: float) -> float:\n    return x * 3", "x * (3 as f64)"),
        ("def multiply(x: float) -> float:\n    return 3 * x", "(3 as f64) * x"),
        ("def multiply(n: int) -> float:\n    return 6.0 * n", "6.0 * (n as f64)"),
        ("def multiply(n: int) -> float:\n    return n * 2.5", "(n as f64) * 2.5"),
        (
            "def calculate(a: int, b: float) -> float:\n    return a * b * 2.0",
            "(a as f64) * b",
        ),
        (
            "def multiply(x: float, n: int) -> float:\n    return x * n",
            "x * (n as f64)",
        ),
        (
            "def multiply(n: int, x: float) -> float:\n    return n * x",
            "(n as f64) * x",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

#[test]
fn test_comparison_operators() {
    let cases = [
        // Less than
        ("def is_small(x: int) -> bool:\n    return x < 1", "x < 1"),
        ("def is_very_negative(x: int) -> bool:\n    return x < -1", "x < -1"),
        ("def below_threshold(x: float) -> bool:\n    return x < 0.5", "x < 0.5"),
        ("def compare_vars(a: int, b: int) -> bool:\n    return a < b", "a < b"),
        // Less than or equal
        ("def at_most_one(x: int) -> bool:\n    return x <= 1", "x <= 1"),
        ("def at_most_negative(x: int) -> bool:\n    return x <= -1", "x <= -1"),
        ("def at_most_half(x: float) -> bool:\n    return x <= 0.5", "x <= 0.5"),
        ("def compare_le(a: int, b: int) -> bool:\n    return a <= b", "a <= b"),
        // Greater than
        ("def is_large(x: int) -> bool:\n    return x > 1", "x > 1"),
        ("def above_negative(x: int) -> bool:\n    return x > -1", "x > -1"),
        ("def above_threshold(x: float) -> bool:\n    return x > 0.5", "x > 0.5"),
        ("def compare_gt(a: int, b: int) -> bool:\n    return a > b", "a > b"),
        // Greater than or equal
        ("def at_least_one(x: int) -> bool:\n    return x >= 1", "x >= 1"),
        ("def at_least_negative(x: int) -> bool:\n    return x >= -1", "x >= -1"),
        ("def at_least_half(x: float) -> bool:\n    return x >= 0.5", "x >= 0.5"),
        ("def compare_ge(a: int, b: int) -> bool:\n    return a >= b", "a >= b"),
        // Equality
        ("def is_one(x: int) -> bool:\n    return x == 1", "x == 1"),
        ("def is_negative_one(x: int) -> bool:\n    return x == -1", "x == -1"),
        ("def is_half(x: float) -> bool:\n    return x == 0.5", "x == 0.5"),
        ("def are_equal(a: int, b: int) -> bool:\n    return a == b", "a == b"),
        // Not equal
        ("def is_not_one(x: int) -> bool:\n    return x != 1", "x != 1"),
        (
            "def is_not_negative_one(x: int) -> bool:\n    return x != -1",
            "x != -1",
        ),
        (
            "def are_not_equal(a: int, b: int) -> bool:\n    return a != b",
            "a != b",
        ),
        // Additional cases
        (
            "def compare_floats(a: float, b: float) -> bool:\n    return a < b",
            "a < b",
        ),
        (
            "def check_sum(a: int, b: int) -> bool:\n    return a + b > 10",
            "a + b > 10",
        ),
        ("def literal_first(x: int) -> bool:\n    return 5 < x", "5 < x"),
        ("def is_positive(x: int) -> bool:\n    return x > 0", "x > 0"),
        (
            "def is_non_negative(x: float) -> bool:\n    return x >= 0.0",
            "x >= 0.0",
        ),
    ];

    for (python, expected) in cases {
        let rust = transpile_and_check(python, &[]);
        assert!(rust.contains(expected), "expected '{expected}' in:\n{rust}");
    }
}

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
