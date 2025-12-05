
// Module: str - More string methods
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: count() was already implemented
// This commit adds 4 NEW methods: center, ljust, rjust, zfill

// 
#[test]
fn test_center() {
    let python = r#"
def center_text(text: str, width: int) -> str:
    return text.center(width)
"#;

    let result = transpile_and_check(python, &[]);

    // Should center string in field
    assert!(result.contains("left_pad") || result.contains("right_pad"));
}

// 
#[test]
#[ignore]
fn test_count() {
    let python = r#"
def count_substring(text: str, sub: str) -> int:
    return text.count(sub)
"#;

    let result = transpile_and_check(python, &[]);

    // Should count substring occurrences
    assert!(result.contains("matches"));
}

// 
#[test]
fn test_ljust() {
    let python = r#"
def left_justify(text: str, width: int) -> str:
    return text.ljust(width)
"#;

    let result = transpile_and_check(python, &[]);

    // Should left justify string
    assert!(result.contains("repeat") && result.contains("width"));
}

// 
#[test]
fn test_rjust() {
    let python = r#"
def right_justify(text: str, width: int) -> str:
    return text.rjust(width)
"#;

    let result = transpile_and_check(python, &[]);

    // Should right justify string
    assert!(result.contains("repeat") && result.contains("width"));
}

// 
#[test]
fn test_zfill() {
    let python = r#"
def zero_fill(text: str, width: int) -> str:
    return text.zfill(width)
"#;

    let result = transpile_and_check(python, &[]);

    // Should fill with zeros
    assert!(result.contains("\"0\"") && result.contains("repeat"));
}

// Total: 4 NEW string methods (count already existed)
// Coverage: center(), ljust(), rjust(), zfill()
