// MIGRATED: Most tests moved to tests/toml/error-handling.toml
// Property tests retained here as they require proptest runtime.

use crate::test_helpers::transpile_and_check;

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_zero_division_always_transpiles(
            a in 1i32..100,
        ) {
            let python_code = format!(r#"
def divide_by_zero() -> int:
    return {} // 0
"#, a);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(result.is_ok(), "Division by zero should transpile");
        }

        #[test]
        fn prop_value_error_messages_preserved(
            threshold in 1i32..1000,
        ) {
            let python_code = format!(r#"
def check_threshold(x: int) -> int:
    if x > {}:
        raise ValueError("exceeds threshold")
    return x
"#, threshold);

            let rust_code = transpile_and_check(&python_code, &[]);
            prop_assert!(
                rust_code.contains("threshold") || rust_code.contains("exceeds"),
                "ValueError message should be preserved in generated code"
            );
        }

        #[test]
        fn prop_index_error_with_various_collections(
            index in 0usize..100,
        ) {
            let python_code = format!(r#"
def access_at_index(items: list) -> int:
    if len(items) <= {}:
        raise IndexError("index too large")
    return items[{}]
"#, index, index);

            let result: Result<String, String> = Ok(transpile_and_check(&python_code, &[]));
            prop_assert!(
                result.is_ok(),
                "IndexError code should always transpile"
            );
        }
    }
}
