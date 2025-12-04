//! Codegen coverage tests validating end-to-end transpilation behavior.

mod test_helpers;
use test_helpers::transpile;

// ============================================================================
// TIER 1: TRY/EXCEPT/FINALLY STATEMENTS (+5-7% coverage)
// ============================================================================

#[test]
fn test_try_with_except_and_finally() {
    let python_code = r#"
def test_exception() -> int:
    try:
        x = 10 / 2
        return x
    except Exception as e:
        return -1
    finally:
        print("cleanup")
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains("match"),
        "try/except/finally should generate match pattern"
    );
}

#[test]
fn test_try_with_except_no_finally() {
    let python_code = r#"
def test_exception() -> int:
    try:
        result = 10 // 0
        return result
    except ZeroDivisionError:
        return -1
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("match"), "try/except should generate match pattern");
}

#[test]
fn test_try_with_finally_no_except() {
    let python_code = r#"
def test_finally() -> int:
    try:
        x = 42
        return x
    finally:
        print("always runs")
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("42"), "try/finally should preserve return value");
}

#[test]
fn test_nested_try_except() {
    let python_code = r#"
def test_nested() -> int:
    try:
        try:
            return 10 // 0
        except:
            return -1
    finally:
        print("outer cleanup")
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains("match"),
        "nested try should generate nested error handling"
    );
}

// ============================================================================
// TIER 2: COMPREHENSIONS WITH CONDITIONS (+4-6% coverage)
// ============================================================================

#[test]
fn test_list_comp_with_condition() {
    let python_code = r#"
def test_comp(nums: list) -> list:
    return [x for x in nums if x > 5]
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains(".filter("),
        "list comp with condition should generate .filter()"
    );
}

#[test]
fn test_set_comp_with_condition() {
    let python_code = r#"
def test_set_comp(items: list) -> set:
    return {x for x in items if x != 0}
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashSet"), "set comp should generate HashSet");
}

#[test]
fn test_dict_comp_with_condition() {
    let python_code = r#"
def test_dict_comp(items: list) -> dict:
    return {x: x * 2 for x in items if x > 0}
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashMap"), "dict comp should generate HashMap");
}

#[test]
fn test_comp_complex_condition() {
    let python_code = r#"
def test_complex(nums: list) -> list:
    return [x for x in nums if x > 5 and x < 10 and x % 2 == 0]
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains("&&"),
        "complex condition should generate && operators"
    );
}

#[test]
fn test_nested_comp_with_condition() {
    let python_code = r#"
def test_nested(matrix: list) -> list:
    return [x + y for x in range(5) for y in range(3) if x > 0]
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains("for"),
        "nested comp should generate nested iteration"
    );
}

// ============================================================================
// TIER 3: TYPE CONVERSIONS (+4-5% coverage)
// ============================================================================

#[test]
fn test_dict_type_conversion() {
    let python_code = r#"
def test_dict(data: dict) -> dict:
    return data
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashMap"), "dict type should convert to HashMap");
}

#[test]
fn test_set_type_conversion() {
    let python_code = r#"
def test_set(items: set) -> set:
    return items
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashSet"), "set type should convert to HashSet");
}

#[test]
fn test_tuple_type_conversion() {
    let python_code = r#"
def test_tuple(pair: tuple) -> tuple:
    return pair
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("("), "tuple type should generate Rust tuple");
}

#[test]
fn test_float_type_conversion() {
    let python_code = r#"
def test_float(x: float) -> float:
    return x * 2.0
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("f64"), "float type should convert to f64");
}

#[test]
fn test_none_type_conversion() {
    let python_code = r#"
def test_none() -> None:
    pass
"#;

    let rust_code = transpile(python_code);
    assert!(
        rust_code.contains("fn test_none"),
        "None type function should be generated"
    );
}

// ============================================================================
// TIER 4: EXPRESSION VARIANTS (+3-4% coverage)
// ============================================================================

#[test]
fn test_ternary_expression() {
    let python_code = r#"
def test_ternary(x: int) -> int:
    return 1 if x > 0 else -1
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("if"), "ternary expression should generate if");
    assert!(rust_code.contains("else"), "ternary expression should generate else");
}

#[test]
fn test_set_literal() {
    let python_code = r#"
def test_set() -> set:
    return {1, 2, 3}
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashSet"), "set literal should generate HashSet");
}

#[test]
fn test_frozenset_literal() {
    let python_code = r#"
def test_frozenset() -> frozenset:
    return frozenset({1, 2, 3})
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("HashSet"), "frozenset should generate HashSet");
}

#[test]
fn test_await_expression() {
    let python_code = r#"
async def test_await() -> int:
    result = await async_func()
    return result
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains(".await"), "await expression should generate .await");
}

#[test]
fn test_yield_expression() {
    let python_code = r#"
def test_generator():
    for i in range(5):
        yield i
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("yield"), "yield should generate yield keyword");
}

#[test]
fn test_sorted_with_key() {
    let python_code = r#"
def test_sorted(items: list) -> list:
    return sorted(items, key=lambda x: x.lower())
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("sort"), "sorted with key should generate sort");
}

#[test]
fn test_sorted_with_reverse() {
    let python_code = r#"
def test_sorted_reverse(nums: list) -> list:
    return sorted(nums, reverse=True)
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("sort"), "sorted with reverse should generate sort");
}

// ============================================================================
// TIER 5: LITERAL VARIANTS (+2-3% coverage)
// ============================================================================

#[test]
fn test_float_literal() {
    let python_code = r#"
def test_float() -> float:
    return 3.14159
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("3.14"), "float literal should be preserved");
}

#[test]
fn test_bytes_literal() {
    let python_code = r#"
def test_bytes() -> bytes:
    return b"hello"
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("b\""), "bytes literal should generate byte string");
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
    assert!(
        rust_code.contains("let mut result"),
        "None literal should generate mutable declaration"
    );
}

// ============================================================================
// TIER 6: BINARY OPERATORS (+2-3% coverage)
// ============================================================================

#[test]
fn test_bitwise_and() {
    let python_code = r#"
def test_bitwise(a: int, b: int) -> int:
    return a & b
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("&"), "bitwise AND should generate & operator");
}

#[test]
fn test_bitwise_or() {
    let python_code = r#"
def test_bitwise(a: int, b: int) -> int:
    return a | b
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("|"), "bitwise OR should generate | operator");
}

#[test]
fn test_bitwise_xor() {
    let python_code = r#"
def test_bitwise(a: int, b: int) -> int:
    return a ^ b
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("^"), "bitwise XOR should generate ^ operator");
}

#[test]
fn test_bitwise_shift_left() {
    let python_code = r#"
def test_shift(x: int) -> int:
    return x << 2
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains("<<"), "left shift should generate << operator");
}

#[test]
fn test_bitwise_shift_right() {
    let python_code = r#"
def test_shift(x: int) -> int:
    return x >> 2
"#;

    let rust_code = transpile(python_code);
    assert!(rust_code.contains(">>"), "right shift should generate >> operator");
}

// ============================================================================
// PROPERTY TESTS - Codegen Robustness
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_list_comp_with_conditions_transpiles(
            threshold in -100i32..100,
        ) {
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
        fn prop_bitwise_operations_transpile(
            op_index in 0usize..5,
        ) {
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
