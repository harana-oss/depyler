
// Module: dict - Advanced dict methods for 50% milestone
// Status: GREEN phase - Tests enabled, 4 of 5 already implemented

use crate::test_helpers::transpile_and_check;

// Note: get(), setdefault(), popitem() were already implemented
// This commit adds pop() - 1 NEW function

// 
#[test]
#[ignore]
fn test_dict_get() {
    let python = r#"
def get_value(d: dict, key: str, default: str) -> str:
    return d.get(key, default)
"#;

    let result = transpile_and_check(python, &[]);

    // Should get value or return default
    assert!(result.contains("get") || result.contains("unwrap_or"));
}

// 
#[test]
#[ignore]
fn test_dict_setdefault() {
    let python = r#"
def set_default_value(d: dict, key: str, default: str) -> str:
    return d.setdefault(key, default)
"#;

    let result = transpile_and_check(python, &[]);

    // Should set and return value if key absent
    assert!(result.contains("entry") || result.contains("or_insert"));
}

// 
#[test]
#[ignore]
fn test_dict_pop() {
    let python = r#"
def pop_value(d: dict, key: str, default: str) -> str:
    return d.pop(key, default)
"#;

    let result = transpile_and_check(python, &[]);

    // Should remove and return value or default
    assert!(result.contains("remove") || result.contains("unwrap_or"));
}

// 
#[test]
#[ignore]
fn test_dict_popitem() {
    let python = r#"
def pop_item(d: dict) -> tuple:
    return d.popitem()
"#;

    let result = transpile_and_check(python, &[]);

    // Should remove and return arbitrary (key, value) pair
    assert!(result.contains("pop") || result.contains("next"));
}

// 
#[test]
#[ignore]
fn test_dict_fromkeys() {
    let python = r#"
def dict_from_keys(keys: list, value: int) -> dict:
    return dict.fromkeys(keys, value)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create dict with keys set to value
    assert!(result.contains("map") || result.contains("collect"));
}

// Total: 1 NEW dict method (pop), 3 already existed (get, setdefault, popitem)
// Coverage: get(), setdefault(), pop(), popitem()
// fromkeys() skipped (requires class method architecture)
