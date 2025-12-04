//! Tests for copy.copy() and copy.deepcopy() code generation

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

#[test]
fn test_copy_copy_list_generates_clone() {
    let python_code = r#"
import copy

def test_shallow_copy() -> int:
    original = [1, 2, 3]
    copied = copy.copy(original)
    copied.append(4)
    return len(original)
"#;
    let rust_code = transpile_and_check(python_code, &["fn test_shallow_copy"]);
    assert!(
        !rust_code.contains(".copy()"),
        "Should not generate invalid .copy() method"
    );
}

#[test]
fn test_copy_copy_dict() {
    let python_code = r#"
import copy

def test_dict_copy() -> int:
    original = {"a": 1, "b": 2}
    copied = copy.copy(original)
    copied["c"] = 3
    return len(original)
"#;
    transpile_and_check(python_code, &["fn test_dict_copy"]);
}

#[test]
fn test_copy_deepcopy_list() {
    let python_code = r#"
import copy

def test_deep_copy() -> int:
    original = [[1, 2], [3, 4]]
    copied = copy.deepcopy(original)
    copied[0].append(5)
    return len(original[0])
"#;
    transpile_and_check(python_code, &["fn test_deep_copy"]);
}
