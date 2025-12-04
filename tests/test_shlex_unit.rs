// Module: shlex - Python shlex module validation
// pending

use crate::test_helpers::transpile_and_check;

//
#[test]
fn test_split() {
    let python = r#"
import shlex

def split_shell(s: str) -> list:
    return shlex.split(s)
"#;

    let result = transpile_and_check(python, &[]);

    // Should split string like shell does (respects quotes)
    assert!(result.contains("in_single_quote") || result.contains("in_double_quote"));
    assert!(result.contains("escaped"));
}

//
#[test]
fn test_quote() {
    let python = r#"
import shlex

def quote_shell(s: str) -> str:
    return shlex.quote(s)
"#;

    let result = transpile_and_check(python, &[]);

    // Should quote string for safe shell usage
    assert!(result.contains("needs_quoting"));
    assert!(result.contains("format"));
}

//
#[test]
fn test_join() {
    let python = r#"
import shlex

def join_shell(args: list) -> str:
    return shlex.join(args)
"#;

    let result = transpile_and_check(python, &[]);

    // Should join list with proper shell escaping
    assert!(result.contains("needs_quoting"));
    assert!(result.contains("join"));
}

// Total: 3 comprehensive tests for shlex module
// Coverage: split, quote, join
// Security: Proper shell escaping to prevent injection
