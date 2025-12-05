//! Tests for Optional auto-unwrap when calling methods on Optional fields/variables.

use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_len_on_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def count_items(c: Container) -> int:
    return len(c.items)
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap().len()"),
        "Should unwrap Optional before calling len().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_method_on_optional_string_upper() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    name: Optional[str]

def upper_name(p: Person) -> str:
    return p.name.upper()
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains("to_uppercase"),
        "Should unwrap Optional string before calling upper().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_append_on_optional_list_uses_as_mut() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def add_item(c: Container, item: int) -> None:
    c.items.append(item)
"#;

    let rust_code = transpile_and_check(source, &[".as_mut().unwrap()"]);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for mutating methods on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_str_on_optional_value() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Data:
    value: Optional[int]

def stringify(d: Data) -> str:
    return str(d.value)
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap().to_string()"),
        "Should unwrap Optional before calling str().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_option_native_method_is_some_not_unwrapped() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Container:
    value: Optional[int]

def check_has_value(c: Container) -> bool:
    return c.value.is_some()
"#;

    let rust_code = transpile(source);
    let check_fn = rust_code.split("fn check_has_value").nth(1).unwrap_or(&rust_code);
    assert!(
        !check_fn.contains(".as_ref().unwrap().is_some"),
        "Should NOT unwrap before Option-native method is_some().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_option_native_method_is_none_not_unwrapped() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Container:
    value: Optional[int]

def check_empty(c: Container) -> bool:
    return c.value.is_none()
"#;

    let rust_code = transpile(source);
    let check_fn = rust_code.split("fn check_empty").nth(1).unwrap_or(&rust_code);
    assert!(
        !check_fn.contains(".as_ref().unwrap().is_none"),
        "Should NOT unwrap before Option-native method is_none().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_option_native_method_unwrap_not_double_unwrapped() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Container:
    value: Optional[int]

def get_value(c: Container) -> int:
    return c.value.unwrap()
"#;

    let rust_code = transpile(source);
    let get_fn = rust_code.split("fn get_value").nth(1).unwrap_or(&rust_code);
    assert!(
        !get_fn.contains(".as_ref().unwrap().unwrap()"),
        "Should NOT double-unwrap for Option.unwrap().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_non_optional_list_method_no_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import List

@dataclass
class Container:
    items: List[int]

def count_items(c: Container) -> int:
    return len(c.items)
"#;

    let rust_code = transpile(source);
    assert!(
        !rust_code.contains(".as_ref().unwrap()"),
        "Should NOT add unwrap for non-Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_sort_on_optional_list_uses_as_mut() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def sort_items(c: Container) -> None:
    c.items.sort()
"#;

    let rust_code = transpile_and_check(source, &[".as_mut().unwrap()"]);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for sort() on Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_extend_on_optional_list_uses_as_mut() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def extend_items(c: Container, more: List[int]) -> None:
    c.items.extend(more)
"#;

    let rust_code = transpile_and_check(source, &[".as_mut().unwrap()"]);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for extend() on Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_clear_on_optional_list_uses_as_mut() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def clear_items(c: Container) -> None:
    c.items.clear()
"#;

    let rust_code = transpile_and_check(source, &[".as_mut().unwrap()"]);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for clear() on Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_method_call() {
    let source = r#"
from typing import Optional, List

def get_len(items: Optional[List[int]]) -> int:
    return len(items)
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains(".as_ref().unwrap().len()"),
        "Should unwrap Optional variable before calling len().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_string_method_lower() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Person:
    nickname: Optional[str]

def lower_name(p: Person) -> str:
    return p.nickname.lower()
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains("to_lowercase"),
        "Should unwrap Optional string before calling lower().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_string_method_strip() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Data:
    text: Optional[str]

def clean_text(d: Data) -> str:
    return d.text.strip()
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains("trim"),
        "Should unwrap Optional string before calling strip().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_single_optional_chain_access() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Inner:
    value: int

@dataclass
class Outer:
    inner: Optional[Inner]

def get_value(o: Outer) -> int:
    return o.inner.value
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains("o.inner.as_ref().unwrap().value"),
        "Should unwrap Optional inner before accessing value field.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_double_optional_chain_access() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Level3:
    data: int

@dataclass
class Level2:
    level3: Optional[Level3]

@dataclass
class Level1:
    level2: Optional[Level2]

def deep_get(l1: Level1) -> int:
    return l1.level2.level3.data
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains("l1.level2.as_ref().unwrap().level3.as_ref().unwrap().data"),
        "Should unwrap both Optional levels before accessing data.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_mixed_optional_chain_access() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Level3:
    data: int

@dataclass
class Level2:
    level3: Level3

@dataclass
class Level1:
    level2: Optional[Level2]

def mixed_get(l1: Level1) -> int:
    return l1.level2.level3.data
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    let get_fn = rust_code.split("fn mixed_get").nth(1).unwrap_or(&rust_code);
    assert!(
        get_fn.contains("l1.level2.as_ref().unwrap().level3.data"),
        "Should only unwrap the Optional field (level2), not the non-Optional (level3).\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_non_optional_chain_no_unwrap() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Inner:
    value: int

@dataclass
class Outer:
    inner: Inner

def get_value(o: Outer) -> int:
    return o.inner.value
"#;

    let rust_code = transpile_and_check(source, &[]);
    let get_fn = rust_code.split("fn get_value").nth(1).unwrap_or(&rust_code);
    assert!(
        !get_fn.contains(".as_ref().unwrap()"),
        "Should NOT add unwrap for non-Optional chain.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_dict_get_chain_upper() {
    let source = r#"
from typing import Dict

def process(d: Dict[str, str], key: str) -> str:
    return d.get(key).upper()
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()", "to_uppercase"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains("to_uppercase"),
        "Should unwrap dict.get() result before calling upper().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_list_pop_chain_lower() {
    let source = r#"
from typing import List

def process(items: List[str]) -> str:
    return items.pop().lower()
"#;

    let rust_code = transpile_and_check(source, &[".pop()", ".unwrap()", "to_lowercase"]);
    assert!(
        rust_code.contains(".pop().unwrap()") && rust_code.contains("to_lowercase"),
        "Should unwrap list.pop() result before calling lower().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_double_method_chain() {
    let source = r#"
from typing import Dict

def process(d: Dict[str, str], key: str) -> str:
    return d.get(key).strip().upper()
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()", "trim", "to_uppercase"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains("trim") && rust_code.contains("to_uppercase"),
        "Should unwrap once for double chain.\nGenerated:\n{}",
        rust_code
    );
    let unwrap_count = rust_code.matches(".unwrap()").count();
    assert!(
        unwrap_count == 1,
        "Should have exactly one unwrap in the chain, not {}.\nGenerated:\n{}",
        unwrap_count,
        rust_code
    );
}

#[test]
fn test_dict_get_chain_len() {
    let source = r#"
from typing import Dict

def get_length(d: Dict[str, str], key: str) -> int:
    return len(d.get(key))
"#;

    let rust_code = transpile_and_check(source, &[".unwrap()", ".len()"]);
    assert!(
        rust_code.contains(".unwrap()") && rust_code.contains(".len()"),
        "Should unwrap dict.get() result before calling len().\nGenerated:\n{}",
        rust_code
    );
}

// ============================================================================
// Collection Operations with Optional: For loops and comprehensions
// ============================================================================

#[test]
fn test_for_loop_over_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def sum_items(c: Container) -> int:
    total = 0
    for item in c.items:
        total += item
    return total
"#;

    let rust_code = transpile_and_check(source, &["as_ref()", ".unwrap()"]);
    assert!(
        rust_code.contains(".items") && rust_code.contains("as_ref()") && rust_code.contains(".unwrap()"),
        "Should unwrap Optional list before iterating.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_for_loop_over_optional_variable() {
    let source = r#"
from typing import Optional, List

def process_items(items: Optional[List[int]]) -> int:
    result = 0
    for item in items:
        result += item
    return result
"#;

    let rust_code = transpile_and_check(source, &["as_ref()", ".unwrap()"]);
    assert!(
        rust_code.contains("items") && rust_code.contains("as_ref()") && rust_code.contains(".unwrap()"),
        "Should unwrap Optional variable before iterating.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_list_comprehension_optional_iterable() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def double_items(c: Container) -> List[int]:
    return [x * 2 for x in c.items]
"#;

    let rust_code = transpile_and_check(source, &["as_ref()", ".unwrap()"]);
    assert!(
        rust_code.contains(".items") && rust_code.contains("as_ref()") && rust_code.contains(".unwrap()"),
        "Should unwrap Optional list in comprehension.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_list_comprehension_optional_variable() {
    let source = r#"
from typing import Optional, List

def triple_values(values: Optional[List[int]]) -> List[int]:
    return [v * 3 for v in values]
"#;

    let rust_code = transpile_and_check(source, &["as_ref()", ".unwrap()"]);
    assert!(
        rust_code.contains("values") && rust_code.contains("as_ref()") && rust_code.contains(".unwrap()"),
        "Should unwrap Optional variable in comprehension.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_list_comprehension_optional_with_filter() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Data:
    numbers: Optional[List[int]]

def get_positives(d: Data) -> List[int]:
    return [x for x in d.numbers if x > 0]
"#;

    let rust_code = transpile_and_check(source, &["as_ref()", ".unwrap()"]);
    assert!(
        rust_code.contains(".numbers") && rust_code.contains("as_ref()") && rust_code.contains(".unwrap()"),
        "Should unwrap Optional list in comprehension with filter.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_for_loop_non_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import List

@dataclass
class Container:
    items: List[int]

def sum_items(c: Container) -> int:
    total = 0
    for item in c.items:
        total += item
    return total
"#;

    let rust_code = transpile(source);
    let sum_fn = rust_code.split("fn sum_items").nth(1).unwrap_or(&rust_code);
    assert!(
        !sum_fn.contains(".as_ref().unwrap()"),
        "Should NOT add unwrap for non-Optional list.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_comprehension_non_optional_no_unwrap() {
    let source = r#"
from dataclasses import dataclass
from typing import List

@dataclass
class Container:
    items: List[int]

def double_items(c: Container) -> List[int]:
    return [x * 2 for x in c.items]
"#;

    let rust_code = transpile(source);
    let double_fn = rust_code.split("fn double_items").nth(1).unwrap_or(&rust_code);
    assert!(
        !double_fn.contains(".as_ref().unwrap()"),
        "Should NOT add unwrap for non-Optional list in comprehension.\nGenerated:\n{}",
        rust_code
    );
}

// ============================================================================
// Additional Collection Operations from 012-collection-operations spec
// ============================================================================

#[test]
fn test_dict_update_on_optional_dict() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Dict

@dataclass
class Config:
    settings: Optional[Dict[str, str]]

def update_config(c: Config, updates: Dict[str, str]) -> None:
    c.settings.update(updates)
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for dict.update() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_set_add_on_optional_set() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Set

@dataclass
class Container:
    tags: Optional[Set[str]]

def add_tag(c: Container, tag: str) -> None:
    c.tags.add(tag)
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for set.add() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_set_discard_on_optional_set() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Set

@dataclass
class Container:
    tags: Optional[Set[str]]

def remove_tag(c: Container, tag: str) -> None:
    c.tags.discard(tag)
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for set.discard() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_in_operator_with_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def contains_value(c: Container, value: int) -> bool:
    return value in c.items
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_ref().unwrap()") && rust_code.contains("contains"),
        "Should unwrap Optional list when using 'in' operator.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_in_operator_with_optional_dict() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Dict

@dataclass
class Config:
    settings: Optional[Dict[str, str]]

def has_setting(c: Config, key: str) -> bool:
    return key in c.settings
"#;

    let rust_code = transpile(source);
    // Note: The transpiler uses .get().is_some() for dict containment checks
    // (works for both HashMap and serde_json::Value)
    assert!(
        rust_code.contains(".as_ref().unwrap()")
            && (rust_code.contains("contains_key") || rust_code.contains(".get(") && rust_code.contains(").is_some()")),
        "Should unwrap Optional dict when using 'in' operator.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_in_operator_with_optional_set() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Set

@dataclass
class Container:
    tags: Optional[Set[str]]

def has_tag(c: Container, tag: str) -> bool:
    return tag in c.tags
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_ref().unwrap()") && rust_code.contains("contains"),
        "Should unwrap Optional set when using 'in' operator.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_filter_none_in_list_comprehension() {
    let source = r#"
from typing import List, Optional

def get_values(items: List[Optional[int]]) -> List[int]:
    return [x for x in items if x is not None]
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains("filter") && rust_code.contains("is_some"),
        "Should filter Optional items using is_some().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_filter_none_with_transform() {
    let source = r#"
from typing import List, Optional

def double_non_none(items: List[Optional[int]]) -> List[int]:
    return [x * 2 for x in items if x is not None]
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains("filter") || rust_code.contains("filter_map"),
        "Should filter out None values before mapping.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[ignore = "Complex comprehension targets (tuple unpacking) not yet supported"]
fn test_dict_items_iteration_optional() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, Dict, List

@dataclass
class Config:
    settings: Optional[Dict[str, str]]

def get_keys(c: Config) -> List[str]:
    return [k for k, v in c.settings.items()]
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_ref().unwrap()"),
        "Should unwrap Optional dict before calling items().\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_insert_on_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def insert_at(c: Container, index: int, value: int) -> None:
    c.items.insert(index, value)
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for list.insert() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_remove_on_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def remove_value(c: Container, value: int) -> None:
    c.items.remove(value)
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for list.remove() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_pop_on_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def pop_last(c: Container) -> int:
    return c.items.pop()
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for list.pop() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_reverse_on_optional_list() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class Container:
    items: Optional[List[int]]

def reverse_items(c: Container) -> None:
    c.items.reverse()
"#;

    let rust_code = transpile(source);
    assert!(
        rust_code.contains(".as_mut().unwrap()"),
        "Should use as_mut().unwrap() for list.reverse() on Optional.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_variable_in_if_condition() {
    let source = r#"
from typing import Optional

def get_value_or_default(opt: Optional[int]) -> int:
    if opt:
        return opt
    return 0
"#;

    // Should convert Optional to boolean with is_some_and() or is_some() in if condition
    let rust_code = transpile_and_check(source, &["is_some"]);
    assert!(
        rust_code.contains("is_some()") || rust_code.contains("is_some_and("),
        "Should convert Optional to boolean with .is_some() or .is_some_and() in if condition.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_field_in_if_condition() {
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Incident:
    time: int

@dataclass
class Game:
    home_first_try_incident: Optional[Incident]

def get_try_time(game: Game) -> int:
    home_first_try_time = 0
    if game.home_first_try_incident:
        home_first_try_time = game.home_first_try_incident.time
    return home_first_try_time
"#;

    // Should convert Optional field to boolean with is_some() or is_some_and()
    let rust_code = transpile_and_check(source, &["is_some"]);
    assert!(
        rust_code.contains("is_some()") || rust_code.contains("is_some_and("),
        "Should convert Optional field to boolean with .is_some() or .is_some_and() in if condition.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_optional_assigned_variable_in_if_condition() {
    // Tests the pattern where an Optional value is assigned to a variable,
    // then that variable is used in an if condition
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Incident:
    time: int

@dataclass
class Game:
    home_first_try_incident: Optional[Incident]

def get_try_time(game: Game) -> int:
    home_first_try_incident = game.home_first_try_incident
    home_first_try_time = 0
    if home_first_try_incident:
        home_first_try_time = 42
    return home_first_try_time
"#;

    // Should convert Optional variable to boolean with is_some()
    let rust_code = transpile_and_check(source, &["is_some"]);
    assert!(
        rust_code.contains("is_some()") || rust_code.contains("is_some_and("),
        "Should convert assigned Optional variable to boolean with .is_some() in if condition.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_field_access_on_optional_variable() {
    // Tests accessing a field on a variable that is itself Optional<SomeClass>
    // This is different from accessing an Optional field on a class - here the variable itself is Optional
    let source = r#"
from dataclasses import dataclass
from typing import Optional

@dataclass
class Incident:
    time_elapsed: int

@dataclass
class Game:
    home_first_try_incident: Optional[Incident]

def get_try_minute(game: Game) -> int:
    home_first_try_incident = game.home_first_try_incident
    return home_first_try_incident.time_elapsed
"#;

    let rust_code = transpile_and_check(source, &[".as_ref().unwrap()"]);
    assert!(
        rust_code.contains("home_first_try_incident.as_ref().unwrap().time_elapsed"),
        "Should unwrap Optional variable before accessing field.\nGenerated:\n{}",
        rust_code
    );
}
