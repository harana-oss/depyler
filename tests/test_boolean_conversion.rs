use crate::test_helpers::transpile_and_check;

#[test]
fn test_list_empty_check_compiles() {
    let python_code = r#"
def is_empty_list(items: list[int]) -> bool:
    if not items:
        return True
    return False
"#;

    transpile_and_check(python_code, &["fn is_empty_list"]);
}

#[test]
fn test_string_empty_check_compiles() {
    let python_code = r#"
def is_empty_string(text: str) -> bool:
    if not text:
        return True
    return False
"#;

    transpile_and_check(python_code, &["fn is_empty_string"]);
}

#[test]
fn test_dict_empty_check_compiles() {
    let python_code = r#"
def is_empty_dict(mapping: dict[str, int]) -> bool:
    if not mapping:
        return True
    return False
"#;

    transpile_and_check(python_code, &["fn is_empty_dict"]);
}

#[test]
fn test_guard_clause_pattern_compiles() {
    let python_code = r#"
def process_items(items: list[int]) -> int:
    if not items:
        return 0
    total = 0
    for item in items:
        total = total + item
    return total
"#;

    transpile_and_check(python_code, &["fn process_items"]);
}

#[test]
fn test_boolean_not_still_works() {
    let python_code = r#"
def negate_bool(flag: bool) -> bool:
    if not flag:
        return True
    return False
"#;

    transpile_and_check(python_code, &["fn negate_bool"]);
}
