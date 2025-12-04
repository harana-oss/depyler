#![allow(non_snake_case)]

use crate::test_helpers::{transpile_and_check, transpile_check_absent};

// ============================================================================
// Copy Types - No Clone Needed
// ============================================================================

#[test]
fn test_copy_types_no_clone() {
    // int: Copy type
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    value: int

def use_twice(state: State) -> int:
    x = state.value
    y = state.value
    return x + y
"#;
    transpile_check_absent(python, &["state.value.clone()"]);

    // float: Copy type
    let python = "def double_use(x: float, y: float) -> float:\n    a = x\n    b = x\n    return a + b + y";
    transpile_check_absent(python, &["x.clone()"]);

    // bool: Copy type
    let python = "def use_bool_twice(flag: bool) -> bool:\n    a = flag\n    b = flag\n    return a and b";
    transpile_check_absent(python, &["flag.clone()"]);
}

// ============================================================================
// String Clone Requirements
// ============================================================================

#[test]
fn test_string_clone_requirements() {
    // String param used twice needs clone
    let python = "def use_string_twice(s: str) -> str:\n    a = s\n    b = s\n    return a + b";
    transpile_and_check(python, &["s.clone()"]);

    // Single use - no clone
    let python = "def single_use(s: str) -> str:\n    return s.upper()";
    transpile_check_absent(python, &["s.clone()"]);

    // String field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    name: str

def get_name(state: State) -> str:
    return state.name
"#;
    transpile_and_check(python, &["state.name.clone()"]);

    // String field read then returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    name: str

def process_and_return(state: State) -> str:
    length = len(state.name)
    if length > 0:
        return state.name
    return ""
"#;
    transpile_and_check(python, &["state.name.clone()"]);

    // String concat same var - format! borrows, no clone needed
    let python = "def repeat_string(s: str) -> str:\n    return s + s + s";
    transpile_check_absent(python, &["s.clone()"]);
}

// ============================================================================
// Collection Clone Requirements (List, Dict, Set)
// ============================================================================

#[test]
fn test_list_clone_requirements() {
    // List field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def get_items(state: State) -> list[int]:
    return state.items
"#;
    transpile_and_check(python, &["state.items.clone()"]);

    // List copy() method to clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def get_copy(state: State) -> list[int]:
    return state.items.copy()
"#;
    transpile_and_check(python, &["state.items.clone()"]);

    // List param used twice
    let python = "def use_list_twice(items: list[int]) -> int:\n    a = items\n    return len(items)";
    transpile_and_check(python, &["items.clone()"]);

    // List iteration - should borrow, not clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    values: list[int]

def sum_values(state: State) -> int:
    total = 0
    for v in state.values:
        total += v
    return total
"#;
    let rust_code = transpile_check_absent(python, &["state.values.clone()"]);
    assert!(
        rust_code.contains("&state.values"),
        "Should iterate over borrowed values: {}",
        rust_code
    );
}

#[test]
fn test_dict_clone_requirements() {
    // Dict field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, int]

def get_data(state: State) -> dict[str, int]:
    return state.data
"#;
    transpile_and_check(python, &["state.data.clone()"]);

    // Dict copy() method to clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, int]

def get_copy(state: State) -> dict[str, int]:
    return state.data.copy()
"#;
    transpile_and_check(python, &["state.data.clone()"]);

    // Dict value string returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, str]

def get_value(state: State, key: str) -> str:
    return state.data[key]
"#;
    transpile_and_check(python, &[".clone()"]);
}

#[test]
fn test_set_clone_requirements() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    tags: set[str]

def get_tags(state: State) -> set[str]:
    return state.tags
"#;
    transpile_and_check(python, &["state.tags.clone()"]);
}

// ============================================================================
// Struct Clone Requirements
// ============================================================================

#[test]
fn test_struct_clone_requirements() {
    // Nested struct field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class Inner:
    value: int

@dataclass
class Outer:
    inner: Inner

def get_inner(outer: Outer) -> Inner:
    return outer.inner
"#;
    transpile_and_check(python, &["outer.inner.clone()"]);

    // Struct param passed twice
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: int
    y: int

def use_point(p: Point) -> int:
    return p.x + p.y

def double_use(p: Point) -> int:
    a = use_point(p)
    b = use_point(p)
    return a + b
"#;
    transpile_and_check(python, &[]);
}

// ============================================================================
// Move vs Clone in Last Use Position
// ============================================================================

#[test]
fn test_move_vs_clone_last_use() {
    // Last use moves, no clone
    let python = "def identity(items: list[int]) -> list[int]:\n    return items";
    transpile_check_absent(python, &["items.clone()"]);

    // Non-last use clones
    let python = "def process(items: list[int]) -> int:\n    copy = items\n    length = len(items)\n    return length";
    transpile_and_check(python, &["items.clone()"]);

    // Three uses = two clones (last can move)
    let python = "def triple_use(s: str) -> str:\n    a = s\n    b = s\n    c = s\n    return a + b + c";
    let rust_code = transpile_and_check(python, &[]);
    let clone_count = rust_code.matches("s.clone()").count();
    assert_eq!(
        clone_count, 2,
        "3 uses of 's' should have exactly 2 clones (last can move): {}",
        rust_code
    );
}

// ============================================================================
// Clone in Loops
// ============================================================================

#[test]
fn test_clone_in_loops() {
    // Clone string in loop append
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    template: str

def build_list(state: State, n: int) -> list[str]:
    result: list[str] = []
    for i in range(n):
        result.append(state.template)
    return result
"#;
    transpile_and_check(python, &["state.template.clone()"]);

    // Copy type in loop - no clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    multiplier: int

def multiply_all(state: State, items: list[int]) -> list[int]:
    result: list[int] = []
    for item in items:
        result.append(item * state.multiplier)
    return result
"#;
    transpile_check_absent(python, &["state.multiplier.clone()"]);
}

// ============================================================================
// Clone in Conditionals
// ============================================================================

#[test]
fn test_clone_in_conditionals() {
    // Clone in both branches
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: list[int]

def conditional_return(state: State, flag: bool) -> list[int]:
    if flag:
        return state.data
    else:
        return state.data
"#;
    transpile_and_check(python, &["state.data.clone()"]);

    // Use in condition then return
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def check_and_return(state: State) -> list[int]:
    if len(state.items) > 0:
        return state.items
    return []
"#;
    transpile_and_check(python, &["state.items.clone()"]);
}

// ============================================================================
// Clone with Field Extraction
// ============================================================================

#[test]
fn test_clone_with_field_extraction() {
    // Extract string field use twice
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    name: str

def use_name_twice(state: State) -> str:
    n = state.name
    return n + state.name
"#;
    transpile_and_check(python, &["state.name.clone()"]);

    // Extract list field then mutate copy
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def get_sorted_copy(state: State) -> list[int]:
    items_copy = state.items.copy()
    items_copy.sort()
    return items_copy
"#;
    transpile_and_check(python, &["state.items.clone()"]);
}

// ============================================================================
// Clone with Nested Access
// ============================================================================

#[test]
fn test_clone_with_nested_access() {
    // Deep nested field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class Level2:
    data: list[int]

@dataclass
class Level1:
    level2: Level2

@dataclass
class Root:
    level1: Level1

def get_deep_data(root: Root) -> list[int]:
    return root.level1.level2.data
"#;
    transpile_and_check(python, &[".clone()"]);

    // Nested struct returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class Inner:
    value: int

@dataclass
class Middle:
    inner: Inner

@dataclass
class Outer:
    middle: Middle

def get_middle(outer: Outer) -> Middle:
    return outer.middle
"#;
    transpile_and_check(python, &["outer.middle.clone()"]);
}

// ============================================================================
// Clone with Multiple Parameters
// ============================================================================

#[test]
fn test_clone_with_multiple_params() {
    // Clone first param not second
    let python = "def process(a: str, b: str) -> str:\n    x = a\n    y = a\n    return x + y + b";
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a.clone()") || rust_code.contains("a"),
        "'a' should be used: {}",
        rust_code
    );

    // Both params used multiple times
    let python = "def interleave(a: str, b: str) -> str:\n    return a + b + a + b";
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a") && rust_code.contains("b"),
        "Both params should be used: {}",
        rust_code
    );
}

// ============================================================================
// Clone with Tuple Patterns
// ============================================================================

#[test]
fn test_clone_with_tuples() {
    // Tuple string element used twice
    let python = "def use_first_twice(pair: tuple[str, str]) -> str:\n    first = pair[0]\n    return first + pair[0]";
    transpile_and_check(python, &[".clone()"]);

    // Destructured string used multiple times
    let python = "def process_pair(pair: tuple[str, str]) -> str:\n    a, b = pair\n    return a + b + a";
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a") && rust_code.contains("b"),
        "Both destructured vars should be used"
    );
}

// ============================================================================
// Clone with Collection Building
// ============================================================================

#[test]
fn test_clone_with_collections() {
    // Collect string fields
    let python = r#"
from dataclasses import dataclass

@dataclass
class Item:
    name: str

def collect_names(items: list[Item]) -> list[str]:
    names: list[str] = []
    for item in items:
        names.append(item.name)
    return names
"#;
    transpile_and_check(python, &["item.name.clone()"]);

    // Map with prefix
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    prefix: str

def prefix_all(state: State, items: list[str]) -> list[str]:
    return [state.prefix + item for item in items]
"#;
    transpile_and_check(python, &["state.prefix.clone()"]);
}

// ============================================================================
// Clone with Recursive Functions and Special Cases
// ============================================================================

#[test]
fn test_clone_special_cases() {
    // String in recursion
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    prefix: str

def recurse_with_prefix(state: State, n: int) -> str:
    if n <= 0:
        return ""
    return state.prefix + recurse_with_prefix(state, n - 1)
"#;
    transpile_and_check(python, &["state.prefix.clone()"]);

    // Backup before mutation
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def backup_then_mutate(state: State) -> list[int]:
    backup = state.items
    state.items.append(42)
    return backup
"#;
    transpile_and_check(python, &["state.items.clone()"]);

    // Method chain two results
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    text: str

def get_both_cases(state: State) -> tuple[str, str]:
    upper = state.text.upper()
    lower = state.text.lower()
    return (upper, lower)
"#;
    transpile_and_check(python, &["state.text.clone()"]);
}

// ============================================================================
// Clone Elision for Owned Values
// ============================================================================

#[test]
fn test_clone_elision() {
    // Created string no clone
    let python = r#"
def create_and_use() -> str:
    s = "hello"
    return s.upper()
"#;
    transpile_check_absent(python, &["s.clone()"]);

    // Created list no clone
    let python = r#"
def create_and_return() -> list[int]:
    items = [1, 2, 3]
    items.append(4)
    return items
"#;
    transpile_check_absent(python, &["items.clone()"]);
}

// ============================================================================
// Clone with Set and Optional Types
// ============================================================================

#[test]
fn test_clone_sets_and_optionals() {
    // Set field returned
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    tags: set[str]

def get_tags(state: State) -> set[str]:
    return state.tags
"#;
    transpile_and_check(python, &["state.tags.clone()"]);

    // Optional string returned
    let python = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class State:
    maybe_name: Optional[str]

def get_maybe(state: State) -> Optional[str]:
    return state.maybe_name
"#;
    transpile_and_check(python, &["state.maybe_name.clone()"]);
}

// ============================================================================
// Clone with Multiple Fields and Pass-Through
// ============================================================================

#[test]
fn test_clone_multiple_fields() {
    // Return two string fields
    let python = r#"
from dataclasses import dataclass

@dataclass
class Person:
    first_name: str
    last_name: str

def get_names(person: Person) -> tuple[str, str]:
    return (person.first_name, person.last_name)
"#;
    transpile_and_check(python, &["person.first_name.clone()", "person.last_name.clone()"]);

    // Return copy fields no clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class Point3D:
    x: int
    y: int
    z: int

def get_xy(point: Point3D) -> tuple[int, int]:
    return (point.x, point.y)
"#;
    transpile_check_absent(python, &["point.x.clone()", "point.y.clone()"]);
}

#[test]
fn test_clone_passthrough() {
    // Passthrough no clone
    let python = "def passthrough(items: list[int]) -> list[int]:\n    return items";
    transpile_check_absent(python, &["items.clone()"]);

    // Field passthrough needs clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def get_items(state: State) -> list[int]:
    return state.items
"#;
    transpile_and_check(python, &["state.items.clone()"]);
}

// ============================================================================
// Clone with Early Return and Assignment Patterns
// ============================================================================

#[test]
fn test_clone_early_return_and_assignment() {
    // String used after early return branch
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    name: str

def process(state: State, empty: bool) -> str:
    if empty:
        return ""
    first = state.name
    second = state.name
    return first + second
"#;
    transpile_and_check(python, &["state.name.clone()"]);

    // Multiple assignment clones
    let python = "def multi_assign(s: str) -> str:\n    a = s\n    b = s\n    c = s\n    return a + b + c";
    let rust_code = transpile_and_check(python, &[]);
    let clone_count = rust_code.matches("s.clone()").count();
    assert!(
        clone_count >= 2,
        "Multiple assignment of 's' needs clones: {}",
        rust_code
    );

    // Reassignment backup
    let python = "def backup_and_return(items: list[int]) -> list[int]:\n    backup = items.copy()\n    return backup";
    transpile_and_check(python, &[".clone()"]);
}

// ============================================================================
// Clone with Default Values and Empty Collections
// ============================================================================

#[test]
fn test_clone_defaults_and_empty() {
    // Default value in get
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, str]
    default: str

def get_or_default(state: State, key: str) -> str:
    return state.data.get(key, state.default)
"#;
    transpile_and_check(python, &["state.default.clone()"]);

    // Empty list still clones
    let python =
        "def use_empty_twice() -> list[int]:\n    items: list[int] = []\n    a = items\n    b = items\n    return a";
    transpile_and_check(python, &["items.clone()"]);
}

// ============================================================================
// Clone with Comprehension Captures
// ============================================================================

#[test]
fn test_clone_comprehension_captures() {
    // Comprehension captures string
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    separator: str
    words: list[str]

def join_words(state: State) -> list[str]:
    return [state.separator + w for w in state.words]
"#;
    transpile_and_check(python, &["state.separator.clone()"]);

    // Comprehension copy capture no clone
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    factor: int
    values: list[int]

def scale_values(state: State) -> list[int]:
    return [v * state.factor for v in state.values]
"#;
    transpile_check_absent(python, &["state.factor.clone()"]);
}

// ============================================================================
// Clone with Call Graph, Transitive Access, and Conditionals
// ============================================================================

#[test]
fn test_clone_call_graph() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: list[int]

def consume(items: list[int]) -> int:
    return len(items)

def caller(state: State) -> int:
    a = consume(state.data)
    b = consume(state.data)
    return a + b
"#;
    transpile_and_check(python, &["state.data.clone()"]);
}

#[test]
fn test_clone_transitive_field() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class Inner:
    name: str

@dataclass
class Outer:
    inner: Inner

def get_name(outer: Outer) -> str:
    return outer.inner.name
"#;
    transpile_and_check(python, &[".clone()"]);
}

#[test]
fn test_clone_conditional_and_loops() {
    // Conditional string assignment
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    a: str
    b: str

def select(state: State, flag: bool) -> str:
    if flag:
        result = state.a
    else:
        result = state.b
    return result
"#;
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("state.a.clone()") || rust_code.contains("state.b.clone()"),
        "Conditional assignment of borrowed strings needs clone: {}",
        rust_code
    );

    // String used in while condition and body
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    target: str
    items: list[str]

def find_match(state: State) -> int:
    i = 0
    while i < len(state.items):
        if state.items[i] == state.target:
            return i
        i += 1
    return -1
"#;
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("state.target.clone()") || rust_code.contains("&state.target"),
        "target used in loop comparison needs proper handling: {}",
        rust_code
    );
}

// ============================================================================
// Clone with Complex Expression
// ============================================================================

#[test]
fn test_clone_complex_expression() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    prefix: str
    suffix: str

def build_string(state: State, middle: str) -> str:
    return state.prefix + middle + state.suffix + state.prefix
"#;
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("state.prefix") && rust_code.contains("state.suffix"),
        "Both fields should be accessed: {}",
        rust_code
    );
}
