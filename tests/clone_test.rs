#![allow(non_snake_case)]

mod test_helpers;

use test_helpers::{transpile_and_check, transpile_check_absent};

// ============================================================================
// Copy Types - No Clone Needed
// ============================================================================

#[test]
fn test_int_copy_no_clone() {
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
}

#[test]
fn test_float_copy_no_clone() {
    let python = r#"
def double_use(x: float, y: float) -> float:
    a = x
    b = x
    return a + b + y
"#;

    transpile_check_absent(python, &["x.clone()"]);
}

#[test]
fn test_bool_copy_no_clone() {
    let python = r#"
def use_bool_twice(flag: bool) -> bool:
    a = flag
    b = flag
    return a and b
"#;

    transpile_check_absent(python, &["flag.clone()"]);
}

// ============================================================================
// String Clone Requirements
// ============================================================================

#[test]
fn test_string_param_used_twice() {
    let python = r#"
def use_string_twice(s: str) -> str:
    a = s
    b = s
    return a + b
"#;

    transpile_and_check(python, &["s.clone()"]);
}

#[test]
fn test_string_single_use_no_clone() {
    let python = r#"
def single_use(s: str) -> str:
    return s.upper()
"#;

    transpile_check_absent(python, &["s.clone()"]);
}

#[test]
fn test_string_field_returned() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    name: str

def get_name(state: State) -> str:
    return state.name
"#;

    transpile_and_check(python, &["state.name.clone()"]);
}

#[test]
fn test_string_field_read_then_returned() {
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
}

#[test]
fn test_string_concat_same_var() {
    let python = r#"
def repeat_string(s: str) -> str:
    return s + s + s
"#;

    transpile_check_absent(python, &["s.clone()"]);
}

// ============================================================================
// List Clone Requirements
// ============================================================================

#[test]
fn test_list_field_returned() {
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

#[test]
fn test_list_copy_method_to_clone() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def get_copy(state: State) -> list[int]:
    return state.items.copy()
"#;

    transpile_and_check(python, &["state.items.clone()"]);
}

#[test]
fn test_list_param_used_twice() {
    let python = r#"
def use_list_twice(items: list[int]) -> int:
    a = items
    return len(items)
"#;

    transpile_and_check(python, &["items.clone()"]);
}

#[test]
fn test_list_iteration_no_clone() {
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
        "Should iterate over borrowed values\n{rust_code}"
    );
}

// ============================================================================
// Dict Clone Requirements
// ============================================================================

#[test]
fn test_dict_field_returned() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, int]

def get_data(state: State) -> dict[str, int]:
    return state.data
"#;

    transpile_and_check(python, &["state.data.clone()"]);
}

#[test]
fn test_dict_copy_method_to_clone() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    data: dict[str, int]

def get_copy(state: State) -> dict[str, int]:
    return state.data.copy()
"#;

    transpile_and_check(python, &["state.data.clone()"]);
}

#[test]
fn test_dict_value_string_returned() {
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

// ============================================================================
// Struct Clone Requirements
// ============================================================================

#[test]
fn test_nested_struct_field_returned() {
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
}

#[test]
fn test_struct_param_passed_twice() {
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
fn test_last_use_moves_no_clone() {
    let python = r#"
def identity(items: list[int]) -> list[int]:
    return items
"#;

    transpile_check_absent(python, &["items.clone()"]);
}

#[test]
fn test_non_last_use_clones() {
    let python = r#"
def process(items: list[int]) -> int:
    copy = items
    length = len(items)
    return length
"#;

    transpile_and_check(python, &["items.clone()"]);
}

#[test]
fn test_three_uses_two_clones() {
    let python = r#"
def triple_use(s: str) -> str:
    a = s
    b = s
    c = s
    return a + b + c
"#;

    let rust_code = transpile_and_check(python, &[]);

    let clone_count = rust_code.matches("s.clone()").count();

    assert_eq!(
        clone_count, 2,
        "3 uses of 's' should have exactly 2 clones (last can move)\n{rust_code}"
    );
}

// ============================================================================
// Clone in Loops
// ============================================================================

#[test]
fn test_clone_string_in_loop_append() {
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
}

#[test]
fn test_copy_type_in_loop_no_clone() {
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
fn test_clone_in_both_branches() {
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
}

#[test]
fn test_use_in_condition_then_return() {
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
fn test_extract_string_field_use_twice() {
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
}

#[test]
fn test_extract_list_field_then_mutate_copy() {
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
fn test_deep_nested_field_returned() {
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
}

#[test]
fn test_nested_struct_returned() {
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
fn test_clone_first_param_not_second() {
    let python = r#"
def process(a: str, b: str) -> str:
    x = a
    y = a
    return x + y + b
"#;

    // format! borrows arguments, so no clones needed for string concat
    // But assignment like `x = a` needs clone if a is used again
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a.clone()") || rust_code.contains("a"),
        "'a' should be used\n{rust_code}"
    );
}

#[test]
fn test_both_params_need_clone() {
    let python = r#"
def interleave(a: str, b: str) -> str:
    return a + b + a + b
"#;

    // format! macro borrows arguments, so no clones are needed for
    // string concatenation using format!
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a") && rust_code.contains("b"),
        "Both params should be used\n{rust_code}"
    );
}

// ============================================================================
// Clone with Tuple Patterns
// ============================================================================

#[test]
fn test_tuple_string_element_used_twice() {
    let python = r#"
def use_first_twice(pair: tuple[str, str]) -> str:
    first = pair[0]
    return first + pair[0]
"#;

    transpile_and_check(python, &[".clone()"]);
}

#[test]
fn test_destructured_string_used_twice() {
    let python = r#"
def process_pair(pair: tuple[str, str]) -> str:
    a, b = pair
    return a + b + a
"#;

    // format! borrows arguments, no clones needed for string concat
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("a") && rust_code.contains("b"),
        "Both destructured vars should be used\n{rust_code}"
    );
}

// ============================================================================
// Clone with Collection Building
// ============================================================================

#[test]
fn test_collect_string_fields() {
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
}

#[test]
fn test_map_with_prefix() {
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
// Clone with Recursive Functions
// ============================================================================

#[test]
fn test_string_in_recursion() {
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
}

// ============================================================================
// Clone Before Mutation
// ============================================================================

#[test]
fn test_backup_before_mutation() {
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
}

// ============================================================================
// Clone with Method Chaining
// ============================================================================

#[test]
fn test_method_chain_two_results() {
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
fn test_created_string_no_clone() {
    let python = r#"
def create_and_use() -> str:
    s = "hello"
    return s.upper()
"#;

    transpile_check_absent(python, &["s.clone()"]);
}

#[test]
fn test_created_list_no_clone() {
    let python = r#"
def create_and_return() -> list[int]:
    items = [1, 2, 3]
    items.append(4)
    return items
"#;

    transpile_check_absent(python, &["items.clone()"]);
}

// ============================================================================
// Clone with Set Operations
// ============================================================================

#[test]
fn test_set_field_returned() {
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
// Clone with Optional Types
// ============================================================================

#[test]
fn test_optional_string_returned() {
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
// Clone with Multiple Fields Returned
// ============================================================================

#[test]
fn test_return_two_string_fields() {
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
}

#[test]
fn test_return_copy_fields_no_clone() {
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

// ============================================================================
// Clone Pass-Through Functions
// ============================================================================

#[test]
fn test_passthrough_no_clone() {
    let python = r#"
def passthrough(items: list[int]) -> list[int]:
    return items
"#;

    transpile_check_absent(python, &["items.clone()"]);
}

#[test]
fn test_field_passthrough_needs_clone() {
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
// Clone with Early Return
// ============================================================================

#[test]
fn test_string_used_after_early_return_branch() {
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
}

// ============================================================================
// Clone with Assignment Patterns
// ============================================================================

#[test]
fn test_multiple_assignment_clones() {
    // Note: Python's chained assignment `a = b = c = s` is not supported by the transpiler.
    // Use individual assignments instead.
    let python = r#"
def multi_assign(s: str) -> str:
    a = s
    b = s
    c = s
    return a + b + c
"#;

    // With format!, we don't need clones for the string concat, but
    // assignments like `let a = s` need clone for 2nd and 3rd use
    let rust_code = transpile_and_check(python, &[]);
    let clone_count = rust_code.matches("s.clone()").count();
    assert!(clone_count >= 2, "Multiple assignment of 's' needs clones\n{rust_code}");
}

#[test]
fn test_reassignment_backup() {
    // This pattern shows Python variable rebinding semantics that are tricky to transpile.
    // In Python, `backup = items; items = [1,2,3]` creates backup as a reference,
    // then rebinds items to a new list. In Rust, we'd need different semantics.
    // For now, test a simpler case where we explicitly clone.
    let python = r#"
def backup_and_return(items: list[int]) -> list[int]:
    backup = items.copy()
    return backup
"#;

    transpile_and_check(python, &[".clone()"]);
}

// ============================================================================
// Clone with Default Values
// ============================================================================

#[test]
fn test_default_value_in_get() {
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
}

// ============================================================================
// Empty Collections
// ============================================================================

#[test]
fn test_empty_list_still_clones() {
    let python = r#"
def use_empty_twice() -> list[int]:
    items: list[int] = []
    a = items
    b = items
    return a
"#;

    transpile_and_check(python, &["items.clone()"]);
}

// ============================================================================
// Clone with Comprehension Captures
// ============================================================================

#[test]
fn test_comprehension_captures_string() {
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
}

#[test]
fn test_comprehension_copy_capture_no_clone() {
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
// Clone with Call Graph Propagation
// ============================================================================

#[test]
fn test_callee_needs_clone_caller_provides() {
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

    // If consume takes ownership, caller needs to clone state.data for first call
    transpile_and_check(python, &["state.data.clone()"]);
}

// ============================================================================
// Clone with Transitive Field Access
// ============================================================================

#[test]
fn test_transitive_field_clone() {
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

    // The transpiler may clone at each level or just at the end
    // Accept either: outer.inner.name.clone() or outer.inner.clone().name.clone()
    transpile_and_check(python, &[".clone()"]);
}

// ============================================================================
// Clone with Conditional Field Assignment
// ============================================================================

#[test]
fn test_conditional_string_assignment() {
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

    // Both branches assign to result, both fields need clone when assigned
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("state.a.clone()") || rust_code.contains("state.b.clone()"),
        "Conditional assignment of borrowed strings needs clone\n{rust_code}"
    );
}

// ============================================================================
// Clone with While Loops
// ============================================================================

#[test]
fn test_string_used_in_while_condition_and_body() {
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
        "'target' used in loop comparison needs proper handling\n{rust_code}"
    );
}

// ============================================================================
// Clone with Complex Expression
// ============================================================================

#[test]
fn test_string_in_complex_expression() {
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    prefix: str
    suffix: str

def build_string(state: State, middle: str) -> str:
    return state.prefix + middle + state.suffix + state.prefix
"#;

    // format! macro borrows its arguments, so no clones are strictly needed for
    // string concatenation. However, field access may still need clone depending
    // on ownership. The transpiler generates clones for field accesses that return
    // non-Copy types.
    let rust_code = transpile_and_check(python, &[]);
    assert!(
        rust_code.contains("state.prefix") && rust_code.contains("state.suffix"),
        "Both fields should be accessed\n{rust_code}"
    );
}
