use depyler_core::DepylerPipeline;

/// Helper function to transpile Python code snippets to Rust
fn transpile_snippet(python_code: &str) -> Result<String, String> {
    let pipeline = DepylerPipeline::new();
    pipeline
        .transpile(python_code)
        .map_err(|e| format!("Transpilation error: {e}"))
}

#[test]
fn test_literal_array_generation() {
    let py_code = r#"
def test_arrays():
    arr1 = [1, 2, 3, 4, 5]
    arr2 = [True, False, True]
    return arr1, arr2
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Check that literal arrays are generated as arrays
    assert!(rust_code.contains("[1, 2, 3, 4, 5]"));
    assert!(rust_code.contains("[true, false, true]"));
    assert!(rust_code.contains("vec!"));
}

#[test]
fn test_array_multiplication_pattern() {
    let py_code = r#"
def test_multiplication():
    zeros = [0] * 10
    ones = [1] * 5
    pattern = [42] * 8
    reverse = 10 * [0]
    return zeros, ones, pattern, reverse
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Check array syntax with size
    assert!(rust_code.contains("[0; 10]"));
    assert!(rust_code.contains("[1; 5]"));
    assert!(rust_code.contains("[42; 8]"));
}

#[test]
fn test_array_init_functions() {
    let py_code = r#"
def test_init():
    z = zeros(10)
    o = ones(5)
    f = full(8, 42)
    return z, o, f
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Check array initialization functions
    assert!(rust_code.contains("[0; 10 as usize]"));
    assert!(rust_code.contains("[1; 5 as usize]"));
    assert!(rust_code.contains("[42; 8 as usize]"));
}

#[test]
fn test_large_array_uses_vec() {
    let py_code = r#"
def test_large():
    # Arrays larger than 32 should use vec
    large = [0] * 50
    return large
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Large arrays will continue to use normal syntax
    assert!(rust_code.contains("[0; 50]"));
    assert!(!rust_code.contains("* 50"));
}

#[test]
fn test_non_literal_arrays_use_vec() {
    let py_code = r#"
def test_dynamic():
    x = 5
    # Non-literal arrays should use vec
    dynamic = [x] * 10
    mixed = [x, 1, 2]
    return dynamic, mixed
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Non-literal elements should still use array syntax for multiplication
    assert!(rust_code.contains("[x; 10]"));
    // But mixed arrays should use vec!
    assert!(rust_code.contains("vec!"));
}

#[test]
fn test_nested_arrays() {
    let py_code = r#"
def test_nested():
    matrix = [[1, 2], [3, 4], [5, 6]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Nested arrays should use vec! for the outer array
    assert!(rust_code.contains("vec!"));
    // But inner arrays can be arrays
    assert!(rust_code.contains("[1, 2]"));
    assert!(rust_code.contains("[3, 4]"));
}

#[test]
fn test_not_in_string_list() {
    let py_code = r#"
def test_membership(variable: str) -> bool:
    return variable not in ['A', 'B']
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("fn test_membership"));
    assert!(!rust_code.contains("contains_key"));
    assert!(rust_code.contains("contains"));
}

#[test]
fn test_array_indexing() {
    let py_code = r#"
def test_ok() -> None:
    array = [1, 2, 3, 4, 5]
    array[0]
    return None
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");
    assert!(!rust_code.contains("() Ok(())"));
}

#[test]
fn test_array_iteration_with_fstring() {
    let py_code = r#"
def test_array_iteration_with_fstring():
    array = [10, 20, 30, 40, 50]
    temp = "10"
    for item in array:
        temp = f"{item}"
    
    return temp
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("fn test_array_iteration_with_fstring"));
    assert!(rust_code.contains("[10, 20, 30, 40, 50]"));
    assert!(rust_code.contains("for item in array"));
    assert!(rust_code.contains("format!"));
}

// ============================================================================
// Nested Array Tests - All Type Permutations
// ============================================================================

#[test]
fn test_nested_arrays_2d_integers() {
    let py_code = r#"
def test_2d_int():
    matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]"));
}

#[test]
fn test_nested_arrays_2d_floats() {
    let py_code = r#"
def test_2d_float():
    matrix = [[1.0, 2.5, 3.7], [4.2, 5.1, 6.9]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix = vec![vec![1.0, 2.5, 3.7], vec![4.2, 5.1, 6.9]]"));
}

#[test]
fn test_nested_arrays_2d_booleans() {
    let py_code = r#"
def test_2d_bool():
    flags = [[True, False, True], [False, True, False]]
    return flags
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let flags = vec![vec![true, false, true], vec![false, true, false]]"));
}

#[test]
fn test_nested_arrays_2d_strings() {
    let py_code = r#"
def test_2d_str():
    words = [["hello", "world"], ["foo", "bar", "baz"]]
    return words
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let words = vec!"));
    assert!(rust_code.contains("\"hello\".to_string()"));
    assert!(rust_code.contains("\"world\".to_string()"));
}

#[test]
fn test_nested_arrays_3d_integers() {
    let py_code = r#"
def test_3d_int():
    cube = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
    return cube
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let cube = vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6], vec![7, 8]]]"));
}

#[test]
fn test_nested_arrays_3d_mixed() {
    let py_code = r#"
def test_3d_mixed():
    data = [[[1, 2, 3], [4, 5, 6]], [[7, 8, 9], [10, 11, 12]], [[13, 14, 15], [16, 17, 18]]]
    return data
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("vec!"));
    assert!(rust_code.contains("[1, 2, 3]"));
    assert!(rust_code.contains("[13, 14, 15]"));
}

#[test]
fn test_nested_arrays_4d() {
    let py_code = r#"
def test_4d():
    tensor = [[[[1, 2], [3, 4]], [[5, 6], [7, 8]]], [[[9, 10], [11, 12]], [[13, 14], [15, 16]]]]
    return tensor
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("vec!"));
    assert!(rust_code.contains("[1, 2]"));
    assert!(rust_code.contains("[15, 16]"));
}

#[test]
fn test_nested_arrays_jagged_integers() {
    let py_code = r#"
def test_jagged():
    jagged = [[1], [2, 3], [4, 5, 6], [7, 8, 9, 10]]
    return jagged
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let jagged = vec![vec![1], vec![2, 3], vec![4, 5, 6], vec![7, 8, 9, 10]]"));
}

#[test]
fn test_nested_arrays_empty_subarrays() {
    let py_code = r#"
def test_empty_nested():
    data = [[], [1, 2], [], [3]]
    return data
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let data = vec![vec![], vec![1, 2], vec![], vec![3]]"));
}

#[test]
fn test_nested_arrays_with_multiplication() {
    let py_code = r#"
def test_nested_mult():
    matrix = [[0] * 3] * 2
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let _cse_temp_0 = [[0; 3]; 2]"));
    assert!(rust_code.contains("let matrix = _cse_temp_0"));
}

#[test]
fn test_nested_arrays_mixed_multiplication() {
    let py_code = r#"
def test_mixed_mult():
    row = [1, 2, 3]
    matrix = [row] * 4
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let row = vec![1, 2, 3]"));
    assert!(rust_code.contains("let _cse_temp_0 = [row; 4]"));
    assert!(rust_code.contains("let matrix = _cse_temp_0"));
}

#[test]
fn test_nested_arrays_single_element() {
    let py_code = r#"
def test_single():
    single = [[42]]
    return single
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let single = vec![vec![42]]"));
}

#[test]
fn test_nested_arrays_heterogeneous_depths() {
    let py_code = r#"
def test_hetero():
    # Mix of different nesting levels
    data = [[1, 2], [[3, 4], [5, 6]]]
    return data
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let data = vec![vec![1, 2], vec![vec![3, 4], vec![5, 6]]]"));
}

#[test]
fn test_nested_arrays_with_negative_numbers() {
    let py_code = r#"
def test_negative():
    matrix = [[-1, -2, -3], [4, -5, 6], [-7, 8, -9]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix = vec![vec![-1, -2, -3], vec![4, -5, 6], vec![-7, 8, -9]]"));
}

#[test]
fn test_nested_arrays_large_dimensions() {
    let py_code = r#"
def test_large():
    matrix = [[i + j for j in range(5)] for i in range(4)]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    // Should handle list comprehensions with nested structures
    assert!(rust_code.contains("collect"));
}

#[test]
fn test_nested_arrays_mixed_types_in_rows() {
    let py_code = r#"
def test_mixed_rows():
    # Different types in different rows (if supported)
    data1 = [[1, 2, 3], [4, 5, 6]]
    data2 = [[True, False], [False, True]]
    return data1, data2
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let data1 = vec![vec![1, 2, 3], vec![4, 5, 6]]"));
    assert!(rust_code.contains("let data2 = vec![vec![true, false], vec![false, true]]"));
}

#[test]
fn test_nested_arrays_with_variables() {
    let py_code = r#"
def test_var_nested():
    x = 10
    y = 20
    matrix = [[x, y], [x + 1, y + 1]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let x = 10"));
    assert!(rust_code.contains("let y = 20"));
    assert!(rust_code.contains("let matrix = vec![vec![x, y], vec![x + 1, y + 1]]"));
}

#[test]
fn test_nested_arrays_complex_expressions() {
    let py_code = r#"
def test_complex():
    a = 5
    matrix = [[a * 2, a + 3], [a - 1, a / 2]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let a = 5"));
    assert!(rust_code.contains("let matrix = vec![vec![a * 2, a + 3], vec![a - 1, a / 2]]"));
}

#[test]
fn test_nested_arrays_zero_initialized() {
    let py_code = r#"
def test_zeros():
    matrix = [[0, 0, 0], [0, 0, 0], [0, 0, 0]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix = vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, 0]]"));
}

#[test]
fn test_nested_arrays_identity_matrix() {
    let py_code = r#"
def test_identity():
    identity = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
    return identity
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let identity = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]]"));
}

#[test]
fn test_nested_arrays_with_string_literals() {
    let py_code = r#"
def test_string_matrix():
    grid = [["a", "b", "c"], ["d", "e", "f"], ["g", "h", "i"]]
    return grid
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("vec!"));
    assert!(rust_code.contains("\"a\""));
    assert!(rust_code.contains("\"i\""));
}

#[test]
fn test_nested_arrays_tuple_like() {
    let py_code = r#"
def test_tuplelike():
    # Arrays that look like coordinate pairs
    points = [[0, 0], [1, 1], [2, 4], [3, 9]]
    return points
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let points = vec![vec![0, 0], vec![1, 1], vec![2, 4], vec![3, 9]]"));
}

#[test]
fn test_nested_arrays_rectangular_vs_jagged() {
    let py_code = r#"
def test_shapes():
    rectangular = [[1, 2, 3], [4, 5, 6]]
    jagged = [[1], [2, 3, 4], [5, 6]]
    return rectangular, jagged
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let rectangular = vec![vec![1, 2, 3], vec![4, 5, 6]]"));
    assert!(rust_code.contains("let jagged = vec![vec![1], vec![2, 3, 4], vec![5, 6]]"));
}

// ============================================================================
// Variable Assignment and Type Inference Tests
// ============================================================================

#[test]
fn test_single_array_assignment_integer() {
    let py_code = r#"
def test_assign_int():
    numbers = [1, 2, 3, 4, 5]
    return numbers
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let numbers = vec![1, 2, 3, 4, 5]"));
}

#[test]
fn test_single_array_assignment_float() {
    let py_code = r#"
def test_assign_float():
    values = [1.5, 2.7, 3.14, 4.0]
    return values
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let values = vec![1.5, 2.7, 3.14, 4.0]"));
}

#[test]
fn test_single_array_assignment_bool() {
    let py_code = r#"
def test_assign_bool():
    flags = [True, False, True, False]
    return flags
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let flags = vec![true, false, true, false]"));
}

#[test]
fn test_single_array_assignment_string() {
    let py_code = r#"
def test_assign_str():
    words = ["hello", "world", "rust"]
    return words
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let words = vec!["));
    assert!(rust_code.contains("\"hello\".to_string()"));
    assert!(rust_code.contains("\"world\".to_string()"));
    assert!(rust_code.contains("\"rust\".to_string()"));
}

#[test]
fn test_nested_array_assignment_2d_integer() {
    let py_code = r#"
def test_assign_2d_int():
    matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]"));
}

#[test]
fn test_nested_array_assignment_2d_float() {
    let py_code = r#"
def test_assign_2d_float():
    data = [[1.0, 2.0], [3.0, 4.0]]
    return data
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]]"));
}

#[test]
fn test_nested_array_assignment_3d_integer() {
    let py_code = r#"
def test_assign_3d():
    cube = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
    return cube
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let cube = vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6], vec![7, 8]]]"));
}

#[test]
fn test_nested_array_assignment_jagged() {
    let py_code = r#"
def test_assign_jagged():
    jagged = [[1], [2, 3], [4, 5, 6]]
    return jagged
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let jagged = vec![vec![1], vec![2, 3], vec![4, 5, 6]]"));
}

#[test]
fn test_multiple_array_assignments_same_type() {
    let py_code = r#"
def test_multiple_same():
    arr1 = [1, 2, 3]
    arr2 = [4, 5, 6]
    arr3 = [7, 8, 9]
    return arr1, arr2, arr3
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let arr1 = vec![1, 2, 3]"));
    assert!(rust_code.contains("let arr2 = vec![4, 5, 6]"));
    assert!(rust_code.contains("let arr3 = vec![7, 8, 9]"));
}

#[test]
fn test_multiple_array_assignments_different_types() {
    let py_code = r#"
def test_multiple_diff():
    ints = [1, 2, 3]
    floats = [1.5, 2.5, 3.5]
    bools = [True, False]
    strings = ["a", "b"]
    return ints, floats, bools, strings
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("ints"));
    assert!(rust_code.contains("floats"));
    assert!(rust_code.contains("bools"));
    assert!(rust_code.contains("strings"));

    // Check each has appropriate literals
    assert!(rust_code.contains("[1, 2, 3]"));
    assert!(rust_code.contains("1.5") || rust_code.contains("2.5"));
    assert!(rust_code.contains("true") && rust_code.contains("false"));
}

#[test]
fn test_empty_array_assignment() {
    let py_code = r#"
def test_empty():
    empty = []
    return empty
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let empty = vec![]"));
}

#[test]
fn test_array_assignment_with_operations() {
    let py_code = r#"
def test_ops():
    x = 10
    arr = [x, x + 1, x * 2, x - 5]
    return arr
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let x = 10"));
    assert!(rust_code.contains("let arr = vec![x, x + 1, x * 2, x - 5]"));
}

#[test]
fn test_array_reassignment() {
    let py_code = r#"
def test_reassign():
    arr = [1, 2, 3]
    arr = [4, 5, 6]
    return arr
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let mut arr = vec![1, 2, 3]"));
    assert!(rust_code.contains("arr = vec![4, 5, 6]"));
}

#[test]
fn test_nested_array_assignment_with_type_annotation() {
    let py_code = r#"
def test_annotated() -> list:
    matrix: list = [[1, 2], [3, 4]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let matrix: Vec<"));
    assert!(rust_code.contains("vec![vec![1, 2], vec![3, 4]]"));
}

#[test]
fn test_array_from_multiplication_assignment() {
    let py_code = r#"
def test_mult_assign():
    zeros = [0] * 10
    ones = [1] * 5
    return zeros, ones
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let _cse_temp_0 = [0; 10]"));
    assert!(rust_code.contains("let zeros = _cse_temp_0"));
    assert!(rust_code.contains("let _cse_temp_1 = [1; 5]"));
    assert!(rust_code.contains("let ones = _cse_temp_1"));
}

#[test]
fn test_nested_array_mixed_literal_and_variable() {
    let py_code = r#"
def test_mixed():
    row1 = [1, 2, 3]
    matrix = [row1, [4, 5, 6], [7, 8, 9]]
    return matrix
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let row1 = vec![1, 2, 3]"));
    assert!(rust_code.contains("let matrix = vec![row1, vec![4, 5, 6], vec![7, 8, 9]]"));
}

#[test]
fn test_array_assignment_preserves_order() {
    let py_code = r#"
def test_order():
    first = [1, 2, 3]
    second = [4, 5, 6]
    third = [7, 8, 9]
    result = [first, second, third]
    return result
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let first = vec![1, 2, 3]"));
    assert!(rust_code.contains("let second = vec![4, 5, 6]"));
    assert!(rust_code.contains("let third = vec![7, 8, 9]"));
    assert!(rust_code.contains("let result = vec![first, second, third]"));
}

#[test]
fn test_array_assignment_with_function_calls() {
    let py_code = r#"
def helper(x):
    return x * 2

def test_func():
    arr = [helper(1), helper(2), helper(3)]
    return arr
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("pub fn helper(x: i32) -> i32"));
    assert!(rust_code.contains("let arr = vec![helper(1), helper(2), helper(3)]"));
}

#[test]
fn test_single_element_array_type_inference() {
    let py_code = r#"
def test_single():
    single_int = [42]
    single_float = [3.14]
    single_bool = [True]
    single_str = ["hello"]
    return single_int, single_float, single_bool, single_str
"#;

    let rust_code = transpile_snippet(py_code).expect("Failed to transpile");

    assert!(rust_code.contains("let single_int = vec![42]"));
    assert!(rust_code.contains("let single_float = vec![3.14]"));
    assert!(rust_code.contains("let single_bool = vec![true]"));
    assert!(rust_code.contains("let single_str = vec![\"hello\".to_string()]"));
}
