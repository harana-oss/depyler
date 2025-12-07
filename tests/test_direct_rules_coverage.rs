// MIGRATED: tests/test_direct_rules_coverage.rs
// Tests moved to:
// - tests/toml/operators.toml (power operators, floor division, set operations)
// - tests/toml/lists-arrays.toml (array initialization: zeros, ones, full)
// - tests/toml/regressions.toml (keyword method name handling)
//
// Property tests kept below as they require proptest runtime.

#[cfg(test)]
mod property_tests {
    use depyler_core::DepylerPipeline;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_power_operations_transpile(
            exp in -5i32..10
        ) {
            let python_code = format!(
                "def test_pow(a: float) -> float:\n    return a ** {}",
                exp
            );

            let pipeline = DepylerPipeline::new();
            let _result = pipeline.transpile(&python_code);
        }

        #[test]
        fn prop_floor_division_transpiles(
            dividend in -100i32..100,
            divisor in 1i32..100  // Avoid division by zero
        ) {
            let python_code = format!(
                "def test_floordiv() -> int:\n    return {} // {}",
                dividend, divisor
            );

            let pipeline = DepylerPipeline::new();
            let _result = pipeline.transpile(&python_code);
        }

        #[test]
        fn prop_method_names_transpile(
            keyword in prop::sample::select(vec!["type", "as", "in", "mut", "ref", "match"])
        ) {
            let python_code = format!(
                "def test_method(obj):\n    return obj.{}()",
                keyword
            );

            let pipeline = DepylerPipeline::new();
            let _result = pipeline.transpile(&python_code);
        }
    }
}
