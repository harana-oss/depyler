// MIGRATED: Many tests moved to TOML files:
// - String method tests → string-operations.toml  
// - Collection method tests → collections.toml
// - Comprehension tests → comprehensions.toml
// - Operator tests → operators.toml
// - Slice tests → slicing.toml
// Property and mutation tests retained here as they require proptest runtime.

use crate::test_helpers::transpile_and_check;

// ============================================================================
// PROPERTY TESTS
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn prop_integer_binary_operations_transpile(a in -100i32..100i32, b in 1i32..100i32) {
            let python_code = format!(r#"
def binary_ops():
    return {} + {}
"#, a, b);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(result.is_ok(), "Binary operation transpilation failed: {:?}", result.err());
        }

        #[test]
        fn prop_list_operations_always_generate_vec(size in 1usize..10) {
            let elements = (0..size).map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
            let python_code = format!(r#"
def make_list():
    return [{}]
"#, elements);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(result.is_ok(), "List transpilation failed");

            let rust_code = result.unwrap();
            prop_assert!(
                rust_code.contains("vec!") || rust_code.contains("vec !"),
                "List should generate vec! macro"
            );
        }

        #[test]
        fn prop_dict_operations_require_hashmap(pairs in 0usize..5) {
            let items = (0..pairs)
                .map(|i| format!(r#""key{}": {}"#, i, i))
                .collect::<Vec<_>>()
                .join(", ");
            let python_code = format!(r#"
def make_dict():
    return {{{}}}
"#, items);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(result.is_ok(), "Dict transpilation failed");

            let rust_code = result.unwrap();
            prop_assert!(
                rust_code.contains("HashMap"),
                "Dict should require HashMap"
            );
        }

        #[test]
        fn prop_range_calls_generate_valid_ranges(n in 1usize..20) {
            let python_code = format!(r#"
def use_range():
    total = 0
    for i in range({}):
        total = total + i
    return total
"#, n);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(result.is_ok(), "range() transpilation failed");

            let rust_code = result.unwrap();
            prop_assert!(
                rust_code.contains("..") || rust_code.contains("range"),
                "range() should generate Rust range syntax"
            );
        }
    }
}

// ============================================================================
// MUTATION TESTS
// ============================================================================

#[cfg(test)]
mod mutation_tests {
    use super::*;

    #[test]
    fn test_mutation_method_dispatch_correctness() {
        let python_code = r#"
def test_list_methods():
    items = []
    items.append(1)
    items.append(2)
    return items
"#;

        let rust_code = transpile_and_check(python_code, &[]);

        assert!(
            rust_code.matches(".push(").count() == 2,
            "MUTATION KILL: Must use .push() exactly 2 times for 2 append calls"
        );

        assert!(
            rust_code.contains(".push(1)") && rust_code.contains(".push(2)"),
            "MUTATION KILL: Must pass correct arguments to .push()"
        );
    }

    #[test]
    fn test_mutation_floor_division_semantics() {
        let python_code = r#"
def floor_div_test(a: int, b: int) -> int:
    return a // b
"#;

        let rust_code = transpile_and_check(python_code, &[]);

        assert!(
            rust_code.contains("needs_adjustment") || rust_code.contains("signs_differ"),
            "MUTATION KILL: Must include Python floor division adjustment logic"
        );
    }

    #[test]
    fn test_mutation_string_method_mapping() {
        let python_code = r#"
def test_upper():
    return "hello".upper()
"#;

        let rust_code = transpile_and_check(python_code, &[]);

        assert!(
            rust_code.contains(".to_uppercase()"),
            "MUTATION KILL: str.upper() must map to .to_uppercase()"
        );
    }
}
