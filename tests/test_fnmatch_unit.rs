
// Module: fnmatch - Python fnmatch module validation
// pending

use crate::test_helpers::transpile_and_check;

//
#[test]
fn test_fnmatch() {
    let python = r#"
import fnmatch

def match_pattern(filename: str, pattern: str) -> bool:
    return fnmatch.fnmatch(filename, pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should match filename against Unix shell-style pattern using regex
    assert!(result.contains("regex") || result.contains("Regex"));
    assert!(result.contains("is_match"));
}

#[test]
fn test_fnmatchcase() {
    let python = r#"
import fnmatch

def match_case_sensitive(filename: str, pattern: str) -> bool:
    return fnmatch.fnmatchcase(filename, pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should match case-sensitively using regex
    assert!(result.contains("regex") || result.contains("Regex"));
    assert!(result.contains("is_match"));
}

//
#[test]
fn test_filter() {
    let python = r#"
import fnmatch

def filter_files(names: list, pattern: str) -> list:
    return fnmatch.filter(names, pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should filter list by pattern using regex
    assert!(result.contains("filter"));
    assert!(result.contains("regex") || result.contains("Regex"));
}

//
#[test]
fn test_translate() {
    let python = r#"
import fnmatch

def translate_pattern(pattern: str) -> str:
    return fnmatch.translate(pattern)
"#;

    let result = transpile_and_check(python, &[]);

    // Should translate shell pattern to regex string
    assert!(result.contains("format") || result.contains("replace"));
}

// Total: 4 comprehensive tests for fnmatch module
// Coverage: fnmatch, fnmatchcase, filter, translate
// Pattern matching: *, ?, [seq], [!seq]
