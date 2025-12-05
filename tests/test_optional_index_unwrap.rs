//! Tests for Optional auto-unwrap when indexing into Optional collections.

use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_optional_list_indexing_by_literal() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def get_first(c: Container) -> int:
    return c.items[0]
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional list before indexing.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_list_indexing_by_variable() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def get_item(c: Container, idx: int) -> int:
    return c.items[idx]
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional list before indexing with variable.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_dict_indexing() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Dict

@dataclass
class Config:
    settings: Optional[Dict[str, int]]

def get_setting(c: Config, key: str) -> int:
    return c.settings[key]
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional dict before indexing.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_list_negative_index() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[str]]

def get_last(c: Container) -> str:
    return c.items[-1]
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional list before negative indexing.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_non_optional_list_no_extra_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import List

@dataclass
class Container:
    items: List[int]

def get_first(c: Container) -> int:
    return c.items[0]
"#;

    let rust_code = transpile(source);
    assert!(
        !rust_code.contains(".as_ref().unwrap()"),
        "Should NOT add as_ref().unwrap() for non-Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_indexing() {
    let source = r#"
from typing import Optional, List

def get_first(items: Optional[List[int]]) -> int:
    return items[0]
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional variable before indexing.\nGenerated:\n{}",
        rust_code
    );
}
