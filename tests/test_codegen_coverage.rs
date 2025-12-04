//! Codegen coverage tests validating end-to-end transpilation behavior.

use crate::test_helpers::transpile;

#[test]
fn test_try_except_variants() {
    // Test that try/except blocks transpile successfully and produce recognizable output
    let cases = [
        (
            r#"
def test_exception() -> int:
    try:
        x = 10 / 2
        return x
    except Exception as e:
        return -1
    finally:
        print("cleanup")
"#,
            vec!["fn test_exception"],
        ),
        (
            r#"
def test_exception() -> int:
    try:
        result = 10 // 0
        return result
    except ZeroDivisionError:
        return -1
"#,
            vec!["fn test_exception"],
        ),
        (
            r#"
def test_finally() -> int:
    try:
        x = 42
        return x
    finally:
        print("always runs")
"#,
            vec!["42"],
        ),
        (
            r#"
def test_nested() -> int:
    try:
        try:
            return 10 // 0
        except:
            return -1
    finally:
        print("outer cleanup")
"#,
            vec!["fn test_nested"],
        ),
    ];

    for (python_code, expected_list) in cases {
        let rust_code = transpile(python_code);
        for expected in expected_list {
            assert!(rust_code.contains(expected), "expected '{expected}' in output");
        }
    }
}

#[test]
fn test_comprehensions() {
    let cases = [
        (
            r#"
def test_comp(nums: list) -> list:
    return [x for x in nums if x > 5]
"#,
            ".filter(",
        ),
        (
            r#"
def test_set_comp(items: list) -> set:
    return {x for x in items if x != 0}
"#,
            "HashSet",
        ),
        (
            r#"
def test_dict_comp(items: list) -> dict:
    return {x: x * 2 for x in items if x > 0}
"#,
            "HashMap",
        ),
        (
            r#"
def test_complex(nums: list) -> list:
    return [x for x in nums if x > 5 and x < 10 and x % 2 == 0]
"#,
            "&&",
        ),
    ];

    for (python_code, expected) in cases {
        let rust_code = transpile(python_code);
        assert!(rust_code.contains(expected), "expected '{expected}' in output");
    }
}

#[test]
fn test_type_conversions() {
    let cases = [
        ("def test_dict(data: dict) -> dict:\n    return data", "HashMap"),
        ("def test_set(items: set) -> set:\n    return items", "HashSet"),
        ("def test_tuple(pair: tuple) -> tuple:\n    return pair", "("),
        ("def test_float(x: float) -> float:\n    return x * 2.0", "f64"),
        ("def test_none() -> None:\n    pass", "fn test_none"),
    ];

    for (python_code, expected) in cases {
        let rust_code = transpile(python_code);
        assert!(rust_code.contains(expected), "expected '{expected}' in output");
    }
}

#[test]
fn test_expressions() {
    let cases = [
        (
            "def test_ternary(x: int) -> int:\n    return 1 if x > 0 else -1",
            vec!["if", "else"],
        ),
        ("def test_set() -> set:\n    return {1, 2, 3}", vec!["HashSet"]),
        (
            "def test_frozenset() -> frozenset:\n    return frozenset({1, 2, 3})",
            vec!["HashSet"],
        ),
        (
            "async def test_await() -> int:\n    result = await async_func()\n    return result",
            vec![".await"],
        ),
        (
            "def test_sorted(items: list) -> list:\n    return sorted(items, key=lambda x: x.lower())",
            vec!["sort"],
        ),
        (
            "def test_sorted_reverse(nums: list) -> list:\n    return sorted(nums, reverse=True)",
            vec!["sort"],
        ),
    ];

    for (python_code, expected_list) in cases {
        let rust_code = transpile(python_code);
        for expected in expected_list {
            assert!(rust_code.contains(expected), "expected '{expected}' in output");
        }
    }
}

#[test]
fn test_literals() {
    let cases = [
        ("def test_float() -> float:\n    return 3.14159", "3.14"),
        ("def test_bytes() -> bytes:\n    return b\"hello\"", "b\""),
    ];

    for (python_code, expected) in cases {
        let rust_code = transpile(python_code);
        assert!(rust_code.contains(expected), "expected '{expected}' in output");
    }
}

#[test]
fn test_none_literal() {
    let python_code = r#"
def test_none(x: int):
    result = None
    if x > 0:
        result = x
    return result
"#;
    let rust_code = transpile(python_code);
    assert!(rust_code.contains("let mut result"));
}

#[test]
fn test_bitwise_operators() {
    let operators = [("&", "&"), ("|", "|"), ("^", "^"), ("<<", "<<"), (">>", ">>")];

    for (py_op, rust_op) in operators {
        let python_code = format!("def test_bitwise(a: int, b: int) -> int:\n    return a {py_op} b");
        let rust_code = transpile(&python_code);
        assert!(rust_code.contains(rust_op), "expected '{rust_op}' in output");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_list_comp_with_conditions_transpiles(threshold in -100i32..100) {
            let python_code = format!(
                "def test_comp(nums: list) -> list:\n    return [x for x in nums if x > {}]",
                threshold
            );
            let _ = transpile(&python_code);
        }

        #[test]
        fn prop_ternary_expressions_transpile(
            true_val in -1000i32..1000,
            false_val in -1000i32..1000,
        ) {
            let python_code = format!(
                "def test_ternary(x: int) -> int:\n    return {} if x > 0 else {}",
                true_val, false_val
            );
            let _ = transpile(&python_code);
        }

        #[test]
        fn prop_bitwise_operations_transpile(op_index in 0usize..5) {
            let ops = ["&", "|", "^", "<<", ">>"];
            let op = ops[op_index];
            let python_code = format!(
                "def test_bitwise(a: int, b: int) -> int:\n    return a {} b",
                op
            );
            let _ = transpile(&python_code);
        }
    }
}
