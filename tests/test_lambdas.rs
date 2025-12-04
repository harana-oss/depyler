//! Consolidated tests for Python lambda expressions in collections
//!
//! Lambda expressions used with collection operations:
//! Python: lambda x: x * 2 → Rust: |x| x * 2

use crate::test_helpers;
use crate::test_helpers::transpile;

// ============================================================================
// Lambda with Map/Filter Operations
// ============================================================================

mod lambda_map_filter {
    use super::*;

    #[test]
    fn test_map_with_simple_lambda() {
        let python = r#"
def double_numbers(numbers: list) -> list:
    return list(map(lambda x: x * 2, numbers))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn double_numbers"), "Should have double_numbers function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_map = rust_code.contains(".map") || rust_code.contains("map(");
        assert!(has_map, "Should have map operation.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_filter_with_simple_lambda() {
        let python = r#"
def filter_positive(numbers: list) -> list:
    return list(filter(lambda x: x > 0, numbers))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn filter_positive"), "Should have filter_positive function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_filter = rust_code.contains(".filter") || rust_code.contains("filter(");
        assert!(has_filter, "Should have filter operation.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_with_key_lambda() {
        let python = r#"
def sort_by_length(words: list) -> list:
    return sorted(words, key=lambda x: len(x))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn sort_by_length"), "Should have sort_by_length function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_sort = rust_code.contains(".sort") || rust_code.contains("sorted");
        assert!(has_sort, "Should have sort operation.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Lambda with Multiple Parameters
// ============================================================================

mod lambda_multi_param {
    use super::*;

    #[test]
    fn test_lambda_with_multiple_parameters() {
        let python = r#"
def add_pairs(pairs: list) -> list:
    return list(map(lambda x, y: x + y, pairs[0], pairs[1]))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn add_pairs"), "Should have add_pairs function.\nGot:\n{}", rust_code);

        let has_multi_param = rust_code.contains("|x, y|")
            || rust_code.contains("| x, y |")
            || rust_code.contains("|(x, y)");
        assert!(has_multi_param, "Should have multi-parameter closure.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_map_with_zip() {
        let python = r#"
def combine_lists(list1: list, list2: list) -> list:
    return list(map(lambda x, y: x + y, list1, list2))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn combine_lists"), "Should have combine_lists function.\nGot:\n{}", rust_code);

        let has_multi_param = rust_code.contains("|x, y|")
            || rust_code.contains("| x, y |")
            || rust_code.contains("|(x, y)");
        assert!(has_multi_param, "Should have multi-parameter closure.\nGot:\n{}", rust_code);

        let has_zip = rust_code.contains("zip") || rust_code.contains("iter");
        assert!(has_zip, "Should have zip or iteration.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Lambda with Closures
// ============================================================================

mod lambda_closures {
    use super::*;

    #[test]
    fn test_lambda_in_list_comprehension() {
        let python = r#"
def process_items(items: list) -> list:
    transform = lambda x: x * 2
    return [transform(item) for item in items]
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn process_items"), "Should have process_items function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_iter = rust_code.contains("for") || rust_code.contains(".iter") || rust_code.contains(".map");
        assert!(has_iter, "Should have iteration.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_lambda_closure_capturing_variables() {
        let python = r#"
def add_offset(numbers: list, offset: int) -> list:
    return list(map(lambda x: x + offset, numbers))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn add_offset"), "Should have add_offset function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);
        assert!(rust_code.contains("offset"), "Should reference offset variable.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Nested and Complex Lambdas
// ============================================================================

mod lambda_complex {
    use super::*;

    #[test]
    fn test_nested_lambda_expressions() {
        let python = r#"
def nested_transform(matrix: list) -> list:
    return list(map(lambda row: list(map(lambda x: x * 2, row)), matrix))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn nested_transform"), "Should have nested_transform function.\nGot:\n{}", rust_code);

        let has_closures = (rust_code.contains("|row|") || rust_code.contains("| row |"))
            && (rust_code.contains("|x|") || rust_code.contains("| x |"));
        assert!(has_closures, "Should have nested closures.\nGot:\n{}", rust_code);

        let map_count = rust_code.matches("map").count();
        assert!(map_count >= 2, "Should have multiple map operations.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_lambda_with_conditional_expression() {
        let python = r#"
def classify_numbers(numbers: list) -> list:
    return list(map(lambda x: "positive" if x > 0 else "non-positive", numbers))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn classify_numbers"), "Should have classify_numbers function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_conditional = rust_code.contains("if") || rust_code.contains("match");
        assert!(has_conditional, "Should have conditional logic.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_lambda_returning_complex_expression() {
        let python = r#"
def calculate_distances(points: list) -> list:
    return list(map(lambda p: (p[0] ** 2 + p[1] ** 2) ** 0.5, points))
"#;

        let rust_code = transpile(python);
        assert!(rust_code.contains("fn calculate_distances"), "Should have calculate_distances function.\nGot:\n{}", rust_code);

        let has_closure = rust_code.contains("|p|") || rust_code.contains("| p |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        let has_math = rust_code.contains("pow") || rust_code.contains("sqrt") || rust_code.contains("**");
        assert!(has_math, "Should have mathematical operations.\nGot:\n{}", rust_code);
    }
}
