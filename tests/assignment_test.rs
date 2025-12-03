mod test_helpers;

use test_helpers::transpile_and_check;

#[test]
fn test_tuple_swap_array_indices() {
    let py_code = r#"
def swap_elements():
    a = [1, 2, 3]
    a[0], a[2] = a[2], a[0]
    return a
"#;

    let rust_code = transpile_and_check(py_code, &["vec!", "_swap_tmp", "a[0", "a[2"]);

    assert!(rust_code.contains("vec!"), "Should contain vec! for array init");
}
