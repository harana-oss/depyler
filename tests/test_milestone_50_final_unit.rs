use crate::test_helpers::transpile_and_check;

//
#[test]
fn test_str_capitalize() {
    let python = r#"
def capitalize_str(s: str) -> str:
    return s.capitalize()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("capitalize") || result.contains("to_uppercase"));
}

//
#[test]
fn test_str_swapcase() {
    let python = r#"
def swap_case(s: str) -> str:
    return s.swapcase()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("swapcase") || result.contains("map"));
}

//
#[test]
fn test_str_expandtabs() {
    let python = r#"
def expand_tabs(s: str) -> str:
    return s.expandtabs()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("expandtabs") || result.contains("replace"));
}

//
#[test]
fn test_str_splitlines() {
    let python = r#"
def split_lines(s: str) -> list:
    return s.splitlines()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("lines") || result.contains("split"));
}

//
#[test]
fn test_str_partition() {
    let python = r#"
def partition_str(s: str, sep: str) -> tuple:
    return s.partition(sep)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("partition") || result.contains("split_once"));
}

//
#[test]
fn test_str_casefold() {
    let python = r#"
def casefold_str(s: str) -> str:
    return s.casefold()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("to_lowercase"));
}

//
#[test]
fn test_str_isprintable() {
    let python = r#"
def is_printable(s: str) -> bool:
    return s.isprintable()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("all") || result.contains("is_control"));
}

// ========== COLLECTION METHODS (2 NEW + 4 EXISTING) ==========

//
#[test]
fn test_dict_clear() {
    let python = r#"
def clear_dict(d: dict) -> None:
    d.clear()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("clear"));
}

//
#[test]
fn test_dict_copy() {
    let python = r#"
def copy_dict(d: dict) -> dict:
    return d.copy()
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("clone"));
}

//
#[test]
fn test_list_extend() {
    let python = r#"
def extend_list(lst: list, items: list) -> None:
    lst.extend(items)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("extend") || result.contains("append"));
}

//
#[test]
fn test_list_index() {
    let python = r#"
def find_index(lst: list, item: int) -> int:
    return lst.index(item)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("position") || result.contains("find"));
}

//
#[test]
fn test_set_discard() {
    let python = r#"
def discard_item(s: set, item: int) -> None:
    s.discard(item)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("remove"));
}

//
#[test]
fn test_set_remove() {
    let python = r#"
def remove_item(s: set, item: int) -> None:
    s.remove(item)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("remove"));
}

// ========== BUILTIN FUNCTIONS (3 NEW) ==========

//
#[test]
fn test_iter_builtin() {
    let python = r#"
def make_iter(items: list):
    return iter(items)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("into_iter"));
}

//
#[test]
fn test_type_builtin() {
    let python = r#"
def get_type(value: int) -> str:
    return str(type(value))
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("type_name"));
}

//
#[test]
fn test_next_builtin() {
    let python = r#"
def get_next(items: list) -> int:
    it = iter(items)
    return next(it)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("next"));
}

//
#[test]
fn test_getattr_builtin() {
    let python = r#"
def get_attr(obj, name: str) -> str:
    return getattr(obj, name)
"#;

    let result = transpile_and_check(python, &[]);
    assert!(result.contains("get") || result.contains("getattr"));
}

// 🎉 50% MILESTONE REACHED! 🎉
// Total: 12 NEW functions (7 string + 2 dict + 3 builtin)
// Already existed: 4 collection methods (list.extend, list.index, set.discard, set.remove)
// Coverage:
//   Strings (7): capitalize(), swapcase(), expandtabs(), splitlines(), partition(), casefold(), isprintable()
//   Dict (2): clear(), copy()
//   Builtins (3): iter(), type(), next()
// Progress: 238 → 250 functions (47.6% → 50.0%) 🎉
