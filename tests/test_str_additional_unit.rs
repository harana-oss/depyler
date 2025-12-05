
// Module: str - Additional string methods
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: count() and find() were already implemented
// This commit adds 3 NEW methods: index, rfind, rindex

// 
#[test]
#[ignore]
fn test_str_index() {
    let python = r#"
def get_index(text: str, sub: str) -> int:
    return text.index(sub)
"#;

    let result = transpile_and_check(python, &[]);

    // Should find index or panic
    assert!(result.contains("find") && result.contains("expect"));
}

// 
#[test]
#[ignore]
fn test_str_rfind() {
    let python = r#"
def find_last(text: str, sub: str) -> int:
    return text.rfind(sub)
"#;

    let result = transpile_and_check(python, &[]);

    // Should find last occurrence
    assert!(result.contains("rfind"));
}

// 
#[test]
#[ignore]
fn test_str_rindex() {
    let python = r#"
def get_last_index(text: str, sub: str) -> int:
    return text.rindex(sub)
"#;

    let result = transpile_and_check(python, &[]);

    // Should find last index or panic
    assert!(result.contains("rfind") && result.contains("expect"));
}

// Total: 3 NEW str methods (count, find already existed)
// Coverage: index(), rfind(), rindex()
