// ============================================================================
// 
// ============================================================================
// BUG: Untyped list/dict parameters generate Vec<DynamicType>/HashMap<DynamicType, DynamicType>
// but DynamicType is never defined, causing compilation failures
//
// ROOT CAUSE: type_mapper.rs maps Type::Unknown → RustType::Custom("DynamicType")
// FIX: Map Type::Unknown → serde_json::Value (already used for untyped dict values)
//
// DISCOVERED: Performance Benchmarking Campaign (compute_intensive.py)
// SEVERITY: P0 BLOCKING - prevents transpilation of any untyped collection parameters
// ============================================================================

use crate::test_helpers;
use crate::test_helpers::transpile;

#[test]
#[allow(non_snake_case)]
fn test_untyped_list_parameter_compiles() {
    let python_code = r#"
def sum_list(numbers: list) -> int:
    """Sum all numbers in a list."""
    total = 0
    for num in numbers:
        total = total + num
    return total
"#;

    let rust_code = transpile(python_code);

    // Should NOT contain DynamicType
    assert!(
        !rust_code.contains("DynamicType"),
        "Generated code should not reference DynamicType\n\
         Expected: serde_json::Value or concrete type\n\
         Generated code:\n{}",
        rust_code
    );
}

#[test]
#[allow(non_snake_case)]
fn test_untyped_dict_parameter_compiles() {
    let python_code = r#"
def get_value(data: dict, key: str) -> int:
    """Get a value from dictionary."""
    if key in data:
        return data[key]
    return 0
"#;

    let rust_code = transpile(python_code);

    // Should not contain DynamicType
    assert!(
        !rust_code.contains("DynamicType"),
        "Should use serde_json::Value instead of DynamicType"
    );
}

#[test]
#[allow(non_snake_case)]
fn test_untyped_set_parameter_compiles() {
    let python_code = r#"
def count_unique(items: set) -> int:
    """Count unique items in a set."""
    return len(items)
"#;

    let rust_code = transpile(python_code);

    // Should not contain DynamicType
    assert!(
        !rust_code.contains("DynamicType"),
        "Should use serde_json::Value instead of DynamicType"
    );
}
