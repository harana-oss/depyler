
// Module: string - Python string constants validation
// Status: GREEN phase - Tests enabled

use crate::test_helpers::transpile_and_check;

// Note: These 3 constants were already implemented (plus 5 more!)
// Already existing: ascii_lowercase, ascii_uppercase, ascii_letters,
//                  digits, hexdigits, octdigits, punctuation, whitespace, printable

// 
#[test]
fn test_ascii_lowercase() {
    let python = r#"
import string

def get_lowercase() -> str:
    return string.ascii_lowercase
"#;

    let result = transpile_and_check(python, &[]);

    // Should return lowercase ASCII letters
    assert!(result.contains("abcdefghijklmnopqrstuvwxyz"));
}

#[test]
fn test_ascii_uppercase() {
    let python = r#"
import string

def get_uppercase() -> str:
    return string.ascii_uppercase
"#;

    let result = transpile_and_check(python, &[]);

    // Should return uppercase ASCII letters
    assert!(result.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
}

#[test]
fn test_ascii_letters() {
    let python = r#"
import string

def get_letters() -> str:
    return string.ascii_letters
"#;

    let result = transpile_and_check(python, &[]);

    // Should return all ASCII letters
    assert!(result.contains("abcdefghijklmnopqrstuvwxyz") && result.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
}

// Total: 3 string module constants verified (8 total already exist)
// Coverage: ascii_lowercase, ascii_uppercase, ascii_letters
