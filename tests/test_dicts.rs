//! Consolidated dictionary/HashMap tests
//!
//! This file consolidates all dict-related tests:
//! - Basic dict assignment and insertion
//! - Nested dict operations
//! - Variable key access
//! - Dict methods (insert, clear, pop, get, contains_key)
//! - Mutability detection for dict parameters

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

use depyler_core::ast_bridge::AstBridge;
use depyler_core::rust_gen::generate_rust_file;
use depyler_core::type_mapper::TypeMapper;
use rustpython_parser::{Mode, parse};

// ============================================================================
// PHASE 1: BASIC DICT ASSIGNMENT TESTS
// ============================================================================

#[test]
fn test_basic_dict_assignment() {
    let python_code = r#"
def test_basic():
    d = {}
    d["key"] = "value"
    d[42] = "number"
    return d
"#;

    // Parse Python to AST
    let ast = parse(python_code, Mode::Module, "<test>").expect("Failed to parse Python");

    // Convert to HIR
    let module = AstBridge::new()
        .with_source(python_code.to_string())
        .python_to_hir(ast)
        .expect("Failed to convert to HIR");

    // Generate Rust code
    let type_mapper = TypeMapper::default();
    let (result, _dependencies) = generate_rust_file(&module, &type_mapper).expect("Failed to generate Rust");

    assert!(result.contains("d.insert"));
    assert!(result.contains(r#""key".to_string()"#));
    assert!(result.contains(r#""value""#));
}

#[test]
fn test_nested_dict_assignment() {
    let python_code = r#"
def test_nested():
    d = {}
    d["outer"] = {}
    d["outer"]["inner"] = "value"
    return d
"#;

    // Parse Python to AST
    let ast = parse(python_code, Mode::Module, "<test>").expect("Failed to parse Python");

    // Convert to HIR
    let module = AstBridge::new()
        .with_source(python_code.to_string())
        .python_to_hir(ast)
        .expect("Failed to convert to HIR");

    // Generate Rust code
    let type_mapper = TypeMapper::default();
    let (result, _dependencies) = generate_rust_file(&module, &type_mapper).expect("Failed to generate Rust");

    assert!(result.contains("get_mut"));
    assert!(result.contains("unwrap()"));
}

#[test]
fn test_deep_nested_dict_assignment() {
    let python_code = r#"
def test_deep():
    d = {}
    d["l1"] = {}
    d["l1"]["l2"] = {}
    d["l1"]["l2"]["l3"] = "deep"
    return d
"#;

    // Parse Python to AST
    let ast = parse(python_code, Mode::Module, "<test>").expect("Failed to parse Python");

    // Convert to HIR
    let module = AstBridge::new()
        .with_source(python_code.to_string())
        .python_to_hir(ast)
        .expect("Failed to convert to HIR");

    // Generate Rust code
    let type_mapper = TypeMapper::default();
    let (result, _dependencies) = generate_rust_file(&module, &type_mapper).expect("Failed to generate Rust");

    // Should have two get_mut calls for the deepest assignment
    let get_mut_count = result.matches("get_mut").count();
    assert!(get_mut_count >= 2);
}

#[test]
fn test_tuple_key_dict() {
    let python_code = r#"
def test_tuple_keys():
    d = {}
    d[(0, 0)] = "origin"
    d[(1, 2)] = "point"
    return d
"#;

    // Parse Python to AST
    let ast = parse(python_code, Mode::Module, "<test>").expect("Failed to parse Python");

    // Convert to HIR
    let module = AstBridge::new()
        .with_source(python_code.to_string())
        .python_to_hir(ast)
        .expect("Failed to convert to HIR");

    // Generate Rust code
    let type_mapper = TypeMapper::default();
    let (result, _dependencies) = generate_rust_file(&module, &type_mapper).expect("Failed to generate Rust");

    assert!(result.contains("(0, 0)"));
    assert!(result.contains("(1, 2)"));
    assert!(result.contains(r#""origin""#));
}

// ============================================================================
// PHASE 2: VARIABLE KEY ACCESS TESTS
// ============================================================================

#[test]
fn test_dict_access_with_string_variable() {
    let python_code = r#"
from typing import Dict, List

def lookup_values(data: Dict[str, int], keys: List[str]) -> List[int]:
    """Test dict access with string variable keys."""
    results = []
    for key in keys:
        if key in data:
            results.append(data[key])
        else:
            results.append(0)
    return results
"#;

    let rust_code = transpile_and_check(python_code, &["fn lookup_values"]);

    // Should NOT contain "as usize" for string keys
    assert!(
        !rust_code.contains("key as usize"),
        "Dict access with string variable should not cast to usize.\nGenerated code:\n{}",
        rust_code
    );

    // Should contain proper HashMap.get() with string key
    assert!(
        rust_code.contains("data.get(key)") || rust_code.contains("data.get(&key)"),
        "Dict access should use .get(key) or .get(&key).\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_dict_literal_key_access() {
    let python_code = r#"
from typing import Dict

def get_value(data: Dict[str, int]) -> int:
    """Test dict access with string literal."""
    return data["mykey"]
"#;

    let rust_code = transpile_and_check(python_code, &["fn get_value"]);

    // Should NOT contain "as usize" for string literal keys
    assert!(
        !rust_code.contains("as usize"),
        "Dict access with string literal should not cast to usize.\nGenerated code:\n{}",
        rust_code
    );

    // Should contain proper HashMap.get() with string literal
    assert!(
        rust_code.contains(".get(\"mykey\")"),
        "Dict access should use .get(\"mykey\").\nGenerated code:\n{}",
        rust_code
    );
}

#[test]
fn test_list_access_with_int_variable() {
    let python_code = r#"
from typing import List

def get_item(items: List[int], index: int) -> int:
    """Test list access with int variable."""
    return items[index]
"#;

    let rust_code = transpile_and_check(python_code, &["fn get_item"]);

    // SHOULD contain "as usize" for integer index
    assert!(
        rust_code.contains("as usize"),
        "List access with int variable should cast to usize.\nGenerated code:\n{}",
        rust_code
    );
}

// ============================================================================
// PHASE 3: DICT METHOD TESTS WITH MUTABILITY
// ============================================================================

#[test]
fn test_dict_insert_adds_mut() {
    let python_code = r#"
def add_entry(d: dict[str, int], key: str, value: int) -> dict[str, int]:
    d[key] = value
    return d
"#;

    let rust_code = transpile_and_check(
        python_code,
        &["fn add_entry", ".insert(", "mut d: HashMap<String, i32>"],
    );
}

#[test]
fn test_dict_clear_adds_mut() {
    let python_code = r#"
def clear_dict(d: dict[str, int]) -> dict[str, int]:
    d.clear()
    return d
"#;

    transpile_and_check(
        python_code,
        &["fn clear_dict", ".clear()", "mut d: HashMap<String, i32>"],
    );
}

#[test]
fn test_dict_pop_removes_double_ref() {
    let python_code = r#"
def pop_entry(d: dict[str, int], key: str) -> int:
    return d.pop(key, -1)
"#;

    let rust_code = transpile_and_check(python_code, &["fn pop_entry"]);

    // Should NOT have &&key (double reference)
    assert!(
        !rust_code.contains("&&key") && !rust_code.contains("& & key"),
        "Should NOT contain &&key double reference"
    );

    // Should have single reference or no reference
    assert!(
        rust_code.contains(".remove(key)") || rust_code.contains(".remove(&key)"),
        "Should contain .remove(key) or .remove(&key)"
    );
}

#[test]
fn test_dict_contains_key_no_double_ref() {
    let python_code = r#"
def has_key(d: dict[str, int], key: str) -> bool:
    return key in d
"#;

    let rust_code = transpile_and_check(python_code, &["fn has_key"]);

    // Should NOT have &&key (double reference)
    assert!(
        !rust_code.contains("&&key") && !rust_code.contains("& & key"),
        "Should NOT contain &&key double reference"
    );

    // This works for both HashMap and serde_json::Value
    assert!(
        rust_code.contains(".get(&key).is_some()") || rust_code.contains(".get(key).is_some()"),
        "Should contain .get(&key).is_some() or .get(key).is_some()"
    );
}

#[test]
fn test_dict_combined_mutations() {
    let python_code = r#"
def modify_dict(d: dict[str, int], k1: str, k2: str) -> dict[str, int]:
    d[k1] = 10
    if k2 in d:
        d.pop(k2)
    return d
"#;

    let rust_code = transpile_and_check(python_code, &["fn modify_dict", "mut d: HashMap<String, i32>"]);

    // Should NOT have &&k2
    assert!(
        !rust_code.contains("&&k2") && !rust_code.contains("& & k2"),
        "Should NOT contain &&k2 double reference"
    );
}

#[test]
fn test_dict_no_mut_when_not_mutated() {
    let python_code = r#"
def get_value(d: dict[str, int], key: str) -> int:
    return d.get(key, 0)
"#;

    let rust_code = transpile_and_check(python_code, &["fn get_value", "HashMap"]);

    // Should NOT have mut d
    assert!(
        !rust_code.contains("mut d:") && !rust_code.contains("mut d :"),
        "Should NOT contain 'mut d' for read-only parameter"
    );
}

#[test]
fn test_dict_remove_in_conditional() {
    let python_code = r#"
def remove_if_exists(d: dict[str, int], key: str) -> dict[str, int]:
    if key in d:
        d.pop(key)
    return d
"#;

    let rust_code = transpile_and_check(python_code, &["fn remove_if_exists", "mut d: HashMap<String, i32>"]);

    // Should NOT have &&key in contains_key or remove
    assert!(
        !rust_code.contains("&&key") && !rust_code.contains("& & key"),
        "Should NOT contain &&key double reference"
    );
}
