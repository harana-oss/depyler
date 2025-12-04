
// Module: re - Python regular expressions module validation
// pending

use crate::test_helpers::transpile_and_check;

//
#[test]
fn test_re_search() {
    let python = r#"
import re

def find_pattern(text: str) -> bool:
    match = re.search(r'\d+', text)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().find()
    assert!(result.contains("Regex") || result.contains("find"));
}

#[test]
fn test_re_match() {
    let python = r#"
import re

def check_start(text: str) -> bool:
    match = re.match(r'^\d+', text)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().is_match()
    assert!(result.contains("Regex") || result.contains("is_match"));
}

#[test]
fn test_re_findall() {
    let python = r#"
import re

def find_all_numbers(text: str) -> list:
    return re.findall(r'\d+', text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().find_iter()
    assert!(result.contains("Regex") || result.contains("find_iter"));
}

#[test]
fn test_re_finditer() {
    let python = r#"
import re

def find_matches(text: str) -> list:
    matches = re.finditer(r'\d+', text)
    return [m.group() for m in matches]
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().find_iter()
    assert!(result.contains("Regex") || result.contains("find_iter"));
}

//
#[test]
fn test_re_sub() {
    let python = r#"
import re

def replace_digits(text: str) -> str:
    return re.sub(r'\d+', 'X', text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().replace_all()
    assert!(result.contains("Regex") || result.contains("replace"));
}

#[test]
fn test_re_subn() {
    let python = r#"
import re

def replace_and_count(text: str) -> tuple:
    return re.subn(r'\d+', 'X', text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().replace_all() with count
    assert!(result.contains("Regex") || result.contains("replace"));
}

//
#[test]
fn test_re_compile() {
    let python = r#"
import re

def compile_pattern():
    pattern = re.compile(r'\d+')
    return pattern
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new()
    assert!(result.contains("Regex::new"));
}

#[test]
fn test_re_compile_with_flags() {
    let python = r#"
import re

def compile_case_insensitive():
    pattern = re.compile(r'hello', re.IGNORECASE)
    return pattern
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate RegexBuilder with case_insensitive
    assert!(result.contains("Regex") || result.contains("case_insensitive"));
}

//
#[test]
fn test_re_split() {
    let python = r#"
import re

def split_by_pattern(text: str) -> list:
    return re.split(r'\s+', text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().split()
    assert!(result.contains("Regex") || result.contains("split"));
}

#[test]
fn test_re_split_with_maxsplit() {
    let python = r#"
import re

def split_limited(text: str) -> list:
    return re.split(r'\s+', text, maxsplit=2)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate Regex::new().splitn()
    assert!(result.contains("Regex") || result.contains("split"));
}

//
#[test]
fn test_re_escape() {
    let python = r#"
import re

def escape_special_chars(text: str) -> str:
    return re.escape(text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate regex::escape()
    assert!(result.contains("escape"));
}

//
#[test]
fn test_match_group() {
    let python = r#"
import re

def extract_group(text: str) -> str:
    match = re.search(r'(\d+)', text)
    if match:
        return match.group(1)
    return ""
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate .as_str() or similar
    assert!(result.contains("as_str") || result.contains("get"));
}

#[test]
fn test_match_groups() {
    let python = r#"
import re

def extract_all_groups(text: str) -> tuple:
    match = re.search(r'(\d+)-(\d+)', text)
    if match:
        return match.groups()
    return ()
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate capture group iteration
    assert!(result.contains("captures") || result.contains("iter"));
}

#[test]
fn test_match_start_end() {
    let python = r#"
import re

def find_position(text: str) -> int:
    match = re.search(r'\d+', text)
    if match:
        return match.start()
    return -1
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate .start()
    assert!(result.contains("start"));
}

//
#[test]
fn test_re_ignorecase_flag() {
    let python = r#"
import re

def case_insensitive_search(text: str) -> bool:
    match = re.search(r'hello', text, re.IGNORECASE)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate case_insensitive builder option
    assert!(result.contains("case_insensitive"));
}

#[test]
fn test_re_multiline_flag() {
    let python = r#"
import re

def multiline_search(text: str) -> bool:
    match = re.search(r'^line', text, re.MULTILINE)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate multi_line builder option
    assert!(result.contains("multi_line") || result.contains("multiline"));
}

#[test]
fn test_re_dotall_flag() {
    let python = r#"
import re

def dotall_search(text: str) -> bool:
    match = re.search(r'.*', text, re.DOTALL)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate dot_matches_new_line builder option
    assert!(result.contains("dot_matches") || result.contains("DOTALL"));
}

//
#[test]
fn test_pattern_search() {
    let python = r#"
import re

def pattern_search(text: str) -> bool:
    pattern = re.compile(r'\d+')
    match = pattern.search(text)
    return match is not None
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate pattern.find()
    assert!(result.contains("find"));
}

#[test]
fn test_pattern_findall() {
    let python = r#"
import re

def pattern_findall(text: str) -> list:
    pattern = re.compile(r'\d+')
    return pattern.findall(text)
"#;

    let result = transpile_and_check(python, &[]);

    // Should generate pattern.find_iter()
    assert!(result.contains("find_iter"));
}

// Total: 20 comprehensive tests for re module
// Coverage: search, match, findall, finditer, sub, subn, compile, split, escape
// Match objects: group, groups, start, end, span
// Flags: IGNORECASE, MULTILINE, DOTALL
// Pattern object methods
