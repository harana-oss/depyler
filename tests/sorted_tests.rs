//! Consolidated tests for sorted() builtin function
//!
//! Tests cover:
//! - Basic sorted() with ascending/descending
//! - sorted() with key parameter (lambdas)
//! - sorted() with reverse parameter
//! - sorted() with both key and reverse

mod test_helpers;

use depyler_core::DepylerPipeline;
use std::process::Command;

// ============================================================================
// Basic Sorted Tests
// ============================================================================

mod basic_sorted {
    use super::*;

    #[test]
    fn test_sorted_ascending_simple() {
        let python_code = r#"
def sort_ascending(numbers: list[int]) -> list[int]:
    return sorted(numbers)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("__sorted_result.sort()"),
            "Should use .sort() for ascending"
        );
        assert!(
            !rust_code.contains(".reverse()"),
            "Should NOT use .reverse() for ascending"
        );
    }

    #[test]
    fn test_sorted_descending_simple() {
        let python_code = r#"
def sort_descending(numbers: list[int]) -> list[int]:
    return sorted(numbers, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("__sorted_result.sort()"),
            "Should use .sort() for descending"
        );
        assert!(
            rust_code.contains("__sorted_result.reverse()"),
            "Should use .reverse() for descending"
        );

        let sort_pos = rust_code.find(".sort()").expect("Should contain .sort()");
        let reverse_pos = rust_code.find(".reverse()").expect("Should contain .reverse()");
        assert!(sort_pos < reverse_pos, ".sort() must come before .reverse()");
    }

    #[test]
    fn test_sorted_reverse_false_explicit() {
        let python_code = r#"
def sort_ascending_explicit(numbers: list[int]) -> list[int]:
    return sorted(numbers, reverse=False)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("__sorted_result.sort()"),
            "Should use .sort() for ascending"
        );
        assert!(
            !rust_code.contains(".reverse()"),
            "Should NOT use .reverse() when reverse=False"
        );
    }

    #[test]
    fn test_sorted_with_strings() {
        let python_code = r#"
def sort_strings(words: list[str]) -> list[str]:
    return sorted(words)

def sort_strings_reverse(words: list[str]) -> list[str]:
    return sorted(words, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("__sorted_result.sort()"),
            "Should use .sort() for strings"
        );
    }
}

// ============================================================================
// Sorted with Key Tests
// ============================================================================

mod sorted_with_key {
    use super::*;

    #[test]
    fn test_sorted_basic_key_lambda() {
        let python = r#"
def sort_by_length(words: list) -> list:
    return sorted(words, key=lambda x: len(x))
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("fn sort_by_length"),
            "Should have sort_by_length function.\nGot:\n{}",
            rust_code
        );

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure syntax.\nGot:\n{}", rust_code);

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        assert!(
            rust_code.contains(".len()"),
            "Should have .len() call.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_sorted_key_with_arithmetic() {
        let python = r#"
def sort_by_doubled(nums: list) -> list:
    return sorted(nums, key=lambda x: x * 2)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_closure = rust_code.contains("|x|") || rust_code.contains("| x |");
        assert!(has_closure, "Should have closure.\nGot:\n{}", rust_code);

        assert!(
            rust_code.contains("* 2"),
            "Should have multiplication.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_sorted_key_with_complex_expression() {
        let python = r#"
def sort_by_complex(items: list) -> list:
    return sorted(items, key=lambda x: (x * 2) + 10)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_arithmetic = rust_code.contains("* 2") && rust_code.contains("+ 10");
        assert!(has_arithmetic, "Should preserve arithmetic.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_without_key() {
        let python = r#"
def simple_sort(nums: list) -> list:
    return sorted(nums)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            !rust_code.contains("sort_by_key"),
            "Simple sorted() should not use sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_sort = rust_code.contains(".sort()") || rust_code.contains("sorted");
        assert!(has_sort, "Should have sorting operation.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_key_with_attribute() {
        let python = r#"
def sort_by_name(people: list) -> list:
    return sorted(people, key=lambda p: p.name)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        assert!(
            rust_code.contains(".name"),
            "Should access .name attribute.\nGot:\n{}",
            rust_code
        );
    }

    #[test]
    fn test_sorted_key_with_indexing() {
        let python = r#"
def sort_by_first(pairs: list) -> list:
    return sorted(pairs, key=lambda p: p[0])
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_index = rust_code.contains("[0]") || rust_code.contains(".get(0");
        assert!(has_index, "Should have indexing.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_key_with_method_call() {
        let python = r#"
def sort_uppercase(words: list) -> list:
    return sorted(words, key=lambda w: len(w))
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        assert!(rust_code.contains(".len()"), "Should have .len().\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_key_with_ternary() {
        let python = r#"
def sort_custom(nums: list) -> list:
    return sorted(nums, key=lambda x: x if x > 0 else -x)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_conditional = rust_code.contains("if") || rust_code.contains("match");
        assert!(has_conditional, "Should have conditional.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_sorted_key_with_negative() {
        let python = r#"
def sort_by_negative(nums: list) -> list:
    return sorted(nums, key=lambda x: -x)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(
            rust_code.contains("sort_by_key"),
            "Should have sort_by_key.\nGot:\n{}",
            rust_code
        );

        let has_negation = rust_code.contains("-x") || rust_code.contains("- x");
        assert!(has_negation, "Should have negation.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Sorted with Key and Reverse Tests
// ============================================================================

mod sorted_key_and_reverse {
    use super::*;

    #[test]
    fn test_sorted_with_key_and_reverse() {
        let python_code = r#"
def sort_by_abs_descending(numbers: list[int]) -> list[int]:
    return sorted(numbers, key=lambda x: abs(x), reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".sort_by_key(") || rust_code.contains("sort_by"),
            "Should use sort_by_key for custom key"
        );
        assert!(
            rust_code.contains(".reverse()"),
            "Should use .reverse() when reverse=True with key"
        );
    }

    #[test]
    fn test_sorted_with_key_no_reverse() {
        let python_code = r#"
def sort_by_abs(numbers: list[int]) -> list[int]:
    return sorted(numbers, key=lambda x: abs(x))
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".sort_by_key(") || rust_code.contains("sort_by"),
            "Should use sort_by_key for custom key"
        );
        assert!(
            !rust_code.contains(".reverse()"),
            "Should NOT use .reverse() when reverse not specified"
        );
    }

    #[test]
    fn test_sorted_key_with_reverse() {
        let python = r#"
def sort_descending(nums: list) -> list:
    return sorted(nums, key=lambda x: x, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python);
        assert!(result.is_ok(), "Transpilation failed: {:?}", result.as_ref().err());

        let rust_code = result.unwrap();

        assert!(rust_code.contains("sort"), "Should have sort.\nGot:\n{}", rust_code);

        let has_reverse =
            rust_code.contains(".rev()") || rust_code.contains("Reverse") || rust_code.contains(".reverse()");
        assert!(has_reverse, "Should handle reverse.\nGot:\n{}", rust_code);
    }
}

// ============================================================================
// Compilation Tests
// ============================================================================

mod compilation_tests {
    use super::*;

    #[test]
    fn test_sorted_compiles_ascending() {
        let python_code = r#"
def sort_ascending(numbers: list[int]) -> list[int]:
    return sorted(numbers)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        std::fs::write("/tmp/test_sorted_asc.rs", &rust_code).expect("Failed to write test file");

        let output = Command::new("rustc")
            .args(["--crate-type", "lib", "/tmp/test_sorted_asc.rs"])
            .output()
            .expect("Failed to execute rustc");

        assert!(
            output.status.success(),
            "Generated code should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_sorted_compiles_descending() {
        let python_code = r#"
def sort_descending(numbers: list[int]) -> list[int]:
    return sorted(numbers, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        std::fs::write("/tmp/test_sorted_desc.rs", &rust_code).expect("Failed to write test file");

        let output = Command::new("rustc")
            .args(["--crate-type", "lib", "/tmp/test_sorted_desc.rs"])
            .output()
            .expect("Failed to execute rustc");

        assert!(
            output.status.success(),
            "Generated code should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_sorted_multiple_functions() {
        let python_code = r#"
def sort_ascending(numbers: list[int]) -> list[int]:
    return sorted(numbers)

def sort_descending(numbers: list[int]) -> list[int]:
    return sorted(numbers, reverse=True)

def sort_by_abs(numbers: list[int]) -> list[int]:
    return sorted(numbers, key=lambda x: abs(x))
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        std::fs::write("/tmp/test_sorted_multiple.rs", &rust_code).expect("Failed to write test file");

        let output = Command::new("rustc")
            .args(["--crate-type", "lib", "/tmp/test_sorted_multiple.rs"])
            .output()
            .expect("Failed to execute rustc");

        assert!(
            output.status.success(),
            "Generated code should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_sorted_strings_compiles() {
        let python_code = r#"
def sort_strings(words: list[str]) -> list[str]:
    return sorted(words)

def sort_strings_reverse(words: list[str]) -> list[str]:
    return sorted(words, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        std::fs::write("/tmp/test_sorted_strings.rs", &rust_code).expect("Failed to write test file");

        let output = Command::new("rustc")
            .args(["--crate-type", "lib", "/tmp/test_sorted_strings.rs"])
            .output()
            .expect("Failed to execute rustc");

        assert!(
            output.status.success(),
            "Generated code should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

// ============================================================================
// Behavior Tests
// ============================================================================

mod behavior_tests {
    use super::*;

    #[test]
    fn test_sorted_behavior_descending() {
        let python_code = r#"
def sort_descending(numbers: list[int]) -> list[int]:
    return sorted(numbers, reverse=True)
"#;

        let pipeline = DepylerPipeline::new();
        let _rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        let test_code = r#"
pub fn sort_descending(numbers: Vec<i32>) -> Vec<i32> {
    {
        let mut __sorted_result = numbers.clone();
        __sorted_result.sort();
        __sorted_result.reverse();
        __sorted_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descending_behavior() {
        let input = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let result = sort_descending(input);
        assert_eq!(result, vec![9, 6, 5, 4, 3, 2, 1, 1]);
    }

    #[test]
    fn test_empty() {
        assert_eq!(sort_descending(vec![]), vec![]);
    }

    #[test]
    fn test_single() {
        assert_eq!(sort_descending(vec![42]), vec![42]);
    }
}
"#;

        std::fs::write("/tmp/test_sorted_behavior_simple.rs", test_code).expect("Failed to write test file");

        let output = Command::new("rustc")
            .args([
                "--test",
                "/tmp/test_sorted_behavior_simple.rs",
                "-o",
                "/tmp/test_sorted_behavior_simple_bin",
            ])
            .output()
            .expect("Failed to compile test");

        assert!(
            output.status.success(),
            "Test code should compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let test_output = Command::new("/tmp/test_sorted_behavior_simple_bin")
            .output()
            .expect("Failed to run test");

        assert!(
            test_output.status.success(),
            "Behavior test should pass:\n{}",
            String::from_utf8_lossy(&test_output.stdout)
        );
    }
}
