//! Consolidated tests for index access, negative indexing, and nested indexing
//!
//! Tests cover:
//! - Index access with Copy vs non-Copy types (DEPYLER-0267)
//! - Negative index handling (DEPYLER-0268)
//! - Nested/multi-dimensional indexing

mod test_helpers;

use depyler_core::DepylerPipeline;
use std::process::Command;

// ============================================================================
// Index Access Tests (DEPYLER-0267)
// ============================================================================

mod index_access {
    use super::*;

    #[test]
    fn test_string_list_index_compiles() {
        let python_code = r#"
def get_string(items: list[str], index: int) -> str:
    """Get string from list by index."""
    return items[index]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_string_index.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_string_index.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("String: Copy"),
                "Cannot use .copied() on String!\nGenerated:\n{}\nError:\n{}",
                rust_code,
                stderr
            );
        }

        assert!(
            output.status.success(),
            "String index should compile\nGenerated:\n{}\nErrors:\n{}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_vec_list_index_compiles() {
        let python_code = r#"
def get_row(matrix: list[list[int]], row: int) -> list[int]:
    """Get row from 2D matrix."""
    return matrix[row]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_vec_index.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_vec_index.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("Vec<i32>: Copy"),
                "Vec doesn't implement Copy!\nError: {}\nCode: {}",
                stderr,
                rust_code
            );
        }

        assert!(
            output.status.success(),
            "Vec index should compile\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_copy_types_still_work() {
        let python_code = r#"
def get_int(nums: list[int], index: int) -> int:
    """Get int from list - Copy type."""
    return nums[index]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_copy_type.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_copy_type.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Copy type (int) should still work\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_negative_index_string_compiles() {
        let python_code = r#"
def get_last_string(items: list[str]) -> str:
    """Get last string from list using negative index."""
    return items[-1]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_negative_string.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_negative_string.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("String: Copy"),
                "Should use .cloned() not .copied()!\nError: {}\nCode: {}",
                stderr,
                rust_code
            );
        }
    }
}

// ============================================================================
// Negative Index Tests (DEPYLER-0268)
// ============================================================================

mod negative_index {
    use super::*;

    #[test]
    fn test_negative_index_last_item_compiles() {
        let python_code = r#"
def get_last(items: list[str]) -> str:
    """Get last item from list using negative index."""
    return items[-1]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_negative_last.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_negative_last.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("cannot apply unary operator `-`") && !stderr.contains("usize: Neg"),
                "Cannot negate usize!\nCode: {}\nError: {}",
                rust_code,
                stderr
            );
        }

        assert!(
            output.status.success(),
            "Negative index -1 should compile\nGenerated:\n{}\nErrors:\n{}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_negative_index_second_last_compiles() {
        let python_code = r#"
def get_second_last(nums: list[int]) -> int:
    """Get second-to-last item using negative index."""
    return nums[-2]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_negative_second_last.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_negative_second_last.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("cannot apply unary operator `-`") && !stderr.contains("usize: Neg"),
                "Cannot negate usize!\nError: {}\nCode: {}",
                stderr,
                rust_code
            );
        }

        assert!(
            output.status.success(),
            "Negative index -2 should compile\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_runtime_negative_index_compiles() {
        let python_code = r#"
def get_by_index(items: list[str], idx: int) -> str:
    """Get item by index - handles both positive and negative at runtime."""
    return items[idx]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_runtime_index.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_runtime_index.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("cannot apply unary operator `-`") && !stderr.contains("usize: Neg"),
                "Runtime index negation error!\nError: {}\nCode: {}",
                stderr,
                rust_code
            );
        }

        assert!(
            output.status.success(),
            "Runtime index should compile\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_nested_collection_negative_index_compiles() {
        let python_code = r#"
def get_last_row(matrix: list[list[int]]) -> list[int]:
    """Get last row from 2D matrix using negative index."""
    return matrix[-1]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_nested_negative.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_nested_negative.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            assert!(
                !stderr.contains("cannot apply unary operator `-`") && !stderr.contains("usize: Neg"),
                "Nested collection negative index error!\nError: {}\nCode: {}",
                stderr,
                rust_code
            );
        }

        assert!(
            output.status.success(),
            "Nested negative index should compile\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }

    #[test]
    fn test_positive_index_still_works() {
        let python_code = r#"
def get_first(items: list[int]) -> int:
    """Get first item - positive index should still work."""
    return items[0]
"#;

        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(python_code);

        assert!(result.is_ok(), "Transpilation should succeed");
        let rust_code = result.unwrap();

        let temp_file = "/tmp/test_positive_index.rs";
        std::fs::write(temp_file, &rust_code).expect("Failed to write temp file");

        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--edition")
            .arg("2021")
            .arg(temp_file)
            .arg("-o")
            .arg("/tmp/test_positive_index.rlib")
            .output()
            .expect("Failed to run rustc");

        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "Positive index should still work (regression test)\nCode: {}\nErrors: {}",
            rust_code,
            stderr
        );
    }
}

// ============================================================================
// Nested Indexing Tests
// ============================================================================

mod nested_indexing {
    use super::*;

    #[test]
    fn test_nested_2d_find_first_match() {
        let python_code = r#"
def find_first_match(matrix: list[list[int]], target: int) -> tuple[int, int]:
    for i in range(len(matrix)):
        for j in range(len(matrix[i])):
            if matrix[i][j] == target:
                return (i, j)
    return (-1, -1)
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for j in 0..{"),
            "Should NOT use block expression in range"
        );
        // Either .get() or direct indexing is acceptable
        let has_indexing = rust_code.contains(".get(") || rust_code.contains("matrix[");
        assert!(has_indexing, "Should have indexing pattern.\nGot:\n{}", rust_code);
    }

    #[test]
    fn test_nested_2d_count_matches() {
        let python_code = r#"
def count_matches_in_matrix(matrix: list[list[int]], target: int) -> int:
    count = 0
    for i in range(len(matrix)):
        for j in range(len(matrix[i])):
            if matrix[i][j] == target:
                count = count + 1
    return count
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for j in 0..{"),
            "Should NOT use block expression in range"
        );
        assert!(
            rust_code.contains(".get(i as usize)"),
            "Should use inline .get() for indexing"
        );
    }

    #[test]
    fn test_nested_2d_sum_matrix() {
        let python_code = r#"
def sum_matrix(matrix: list[list[int]]) -> int:
    total = 0
    for i in range(len(matrix)):
        for j in range(len(matrix[i])):
            total = total + matrix[i][j]
    return total
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for j in 0..{"),
            "Should NOT use block expression in range"
        );
    }

    #[test]
    fn test_3d_nested_indexing() {
        let python_code = r#"
def sum_3d(cube: list[list[list[int]]]) -> int:
    total = 0
    for i in range(len(cube)):
        for j in range(len(cube[i])):
            for k in range(len(cube[i][j])):
                total = total + cube[i][j][k]
    return total
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for j in 0..{"),
            "Should NOT use block expression in j loop"
        );
        assert!(
            !rust_code.contains("for k in 0..{"),
            "Should NOT use block expression in k loop"
        );
        assert!(
            rust_code.contains(".get(i as usize)"),
            "Should use inline .get() for i indexing"
        );
        assert!(
            rust_code.contains(".get(j as usize)"),
            "Should use inline .get() for j indexing"
        );
    }

    #[test]
    fn test_regression_diagonal_access_still_works() {
        let python_code = r#"
def sum_diagonal(matrix: list[list[int]]) -> int:
    total = 0
    for i in range(len(matrix)):
        total = total + matrix[i][i]
    return total
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".get(i as usize)"),
            "Should use inline .get() for simple variable index"
        );
    }

    #[test]
    fn test_regression_complex_index_still_uses_block() {
        let python_code = r#"
def access_next(arr: list[int], i: int) -> int:
    return arr[i + 1]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains("let base =") || rust_code.contains("let idx"),
            "Complex expressions should still use block with negative index handling"
        );
    }

    #[test]
    fn test_regression_negative_literal_index() {
        let python_code = r#"
def get_last(arr: list[int]) -> int:
    return arr[-1]
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            rust_code.contains(".last()") || rust_code.contains("saturating_sub") || rust_code.contains("len()"),
            "Negative literal indices should use special handling (like .last())"
        );
    }

    #[test]
    fn test_matrix_transpose() {
        let python_code = r#"
def transpose(matrix: list[list[int]]) -> list[list[int]]:
    if len(matrix) == 0:
        return []
    rows = len(matrix)
    cols = len(matrix[0])
    result: list[list[int]] = []
    for j in range(cols):
        row: list[int] = []
        for i in range(rows):
            row.append(matrix[i][j])
        result.append(row)
    return result
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for j in 0..{"),
            "Should NOT use block expression in j range"
        );
        assert!(
            !rust_code.contains("for i in 0..{"),
            "Should NOT use block expression in i range"
        );
    }

    #[test]
    fn test_matrix_multiply() {
        let python_code = r#"
def matrix_multiply(a: list[list[int]], b: list[list[int]]) -> list[list[int]]:
    rows_a = len(a)
    cols_a = len(a[0])
    cols_b = len(b[0])
    result: list[list[int]] = []
    for i in range(rows_a):
        row: list[int] = []
        for j in range(cols_b):
            total = 0
            for k in range(cols_a):
                total = total + a[i][k] * b[k][j]
            row.append(total)
        result.append(row)
    return result
"#;

        let pipeline = DepylerPipeline::new();
        let rust_code = pipeline.transpile(python_code).expect("Transpilation failed");

        assert!(
            !rust_code.contains("for i in 0..{")
                && !rust_code.contains("for j in 0..{")
                && !rust_code.contains("for k in 0..{"),
            "Should NOT use block expressions in any range"
        );
    }
}
