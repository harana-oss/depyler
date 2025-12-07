# Migration Task: Dictionary Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_dicts.rs | 253 | ~15 | `transpile_and_check` |
| test_dict_advanced_unit.rs | ~200 | ~12 | `transpile_and_check` |
| test_dict_value_operations.rs | ~150 | ~8 | `transpile_and_check` |
| test_debug_collections.rs | ~100 | ~5 | `transpile_and_check` |

## Target File

`tests/toml/07-dictionaries.toml`

## Tests to Migrate

### Basic Dict Operations

```toml
[[test]]
name = "dict_empty_creation"
description = "Create empty dictionary"

[test.python]
code =  '''
def create_empty() -> dict:
    d = {}
    return d
'''

[test.assertions]
contains = ["HashMap::new()"]
```

```toml
[[test]]
name = "dict_literal_creation"
description = "Create dictionary with literal"

[test.python]
code =  '''
def create_dict() -> dict:
    d = {"a": 1, "b": 2}
    return d
'''

[test.assertions]
contains = ["HashMap", "insert"]
```

```toml
[[test]]
name = "dict_basic_assignment"
description = "Basic dictionary assignment"

[test.python]
code =  '''
def test_basic():
    d = {}
    d["key"] = "value"
    d[42] = "number"
    return d
'''

[test.assertions]
contains = ["d.insert", "\"key\".to_string()", "\"value\""]
```

### Nested Dict Operations

```toml
[[test]]
name = "dict_nested_assignment"
description = "Nested dictionary assignment"

[test.python]
code =  '''
def test_nested():
    d = {}
    d["outer"] = {}
    d["outer"]["inner"] = "value"
    return d
'''

[test.assertions]
contains = ["get_mut", "unwrap()"]
```

```toml
[[test]]
name = "dict_deep_nested"
description = "Deeply nested dictionary"

[test.python]
code =  '''
def test_deep():
    d = {}
    d["l1"] = {}
    d["l1"]["l2"] = {}
    d["l1"]["l2"]["l3"] = "deep"
    return d
'''

[test.assertions]
contains = ["get_mut"]
count = { "get_mut" = { min = 2 } }
```

### Dict Access

```toml
[[test]]
name = "dict_access_string_key"
description = "Access dict with string variable key"

[test.python]
code =  '''
from typing import Dict, List

def lookup_values(data: Dict[str, int], keys: List[str]) -> List[int]:
    results = []
    for key in keys:
        if key in data:
            results.append(data[key])
        else:
            results.append(0)
    return results
'''

[test.assertions]
contains = ["fn lookup_values"]
not_contains = ["key as usize"]
```

```toml
[[test]]
name = "dict_get_method"
description = "Dictionary get() method"

[test.python]
code =  '''
def safe_get(d: dict, key: str) -> int:
    return d.get(key, 0)
'''

[test.assertions]
contains = ["get(", "unwrap_or"]
any_of = ["unwrap_or(0)", "unwrap_or_default()"]
```

```toml
[[test]]
name = "dict_get_default"
description = "Dictionary get() with default value"

[test.python]
code =  '''
def get_or_default(d: dict, key: str, default: str) -> str:
    return d.get(key, default)
'''

[test.assertions]
contains = ["get(", "unwrap_or"]
```

### Dict Methods

```toml
[[test]]
name = "dict_keys"
description = "Dictionary keys() method"

[test.python]
code =  '''
def get_keys(d: dict) -> list:
    return list(d.keys())
'''

[test.assertions]
contains = ["keys()"]
```

```toml
[[test]]
name = "dict_values"
description = "Dictionary values() method"

[test.python]
code =  '''
def get_values(d: dict) -> list:
    return list(d.values())
'''

[test.assertions]
contains = ["values()"]
```

```toml
[[test]]
name = "dict_items"
description = "Dictionary items() method"

[test.python]
code =  '''
def get_items(d: dict) -> list:
    return list(d.items())
'''

[test.assertions]
contains = ["iter()"]
```

```toml
[[test]]
name = "dict_contains_key"
description = "Check if key exists in dict"

[test.python]
code =  '''
def has_key(d: dict, key: str) -> bool:
    return key in d
'''

[test.assertions]
contains = ["contains_key"]
```

```toml
[[test]]
name = "dict_pop"
description = "Dictionary pop() method"

[test.python]
code =  '''
def pop_key(d: dict, key: str) -> int:
    return d.pop(key)
'''

[test.assertions]
contains = ["remove("]
```

```toml
[[test]]
name = "dict_pop_default"
description = "Dictionary pop() with default"

[test.python]
code =  '''
def pop_or_default(d: dict, key: str, default: int) -> int:
    return d.pop(key, default)
'''

[test.assertions]
contains = ["remove("]
any_of = ["unwrap_or", "Option"]
```

```toml
[[test]]
name = "dict_clear"
description = "Dictionary clear() method"

[test.python]
code =  '''
def clear_dict(d: dict) -> None:
    d.clear()
'''

[test.assertions]
contains = ["clear()"]
```

```toml
[[test]]
name = "dict_update"
description = "Dictionary update() method"

[test.python]
code =  '''
def merge_dicts(d1: dict, d2: dict) -> dict:
    d1.update(d2)
    return d1
'''

[test.assertions]
contains = ["extend("]
```

### Dict Comprehensions

```toml
[[test]]
name = "dict_comprehension_basic"
description = "Basic dictionary comprehension"

[test.python]
code =  '''
def squares_dict(n: int) -> dict:
    return {i: i*i for i in range(n)}
'''

[test.assertions]
contains = ["HashMap", "collect"]
```

```toml
[[test]]
name = "dict_comprehension_filter"
description = "Dictionary comprehension with filter"

[test.python]
code =  '''
def even_squares(n: int) -> dict:
    return {i: i*i for i in range(n) if i % 2 == 0}
'''

[test.assertions]
contains = ["filter", "collect"]
```

### Tuple Key Dicts

```toml
[[test]]
name = "dict_tuple_keys"
description = "Dictionary with tuple keys"

[test.python]
code =  '''
def test_tuple_keys():
    d = {}
    d[(0, 0)] = "origin"
    d[(1, 2)] = "point"
    return d
'''

[test.assertions]
contains = ["(0, 0)", "(1, 2)", "\"origin\""]
```

### Dict Iteration

```toml
[[test]]
name = "dict_iterate_keys"
description = "Iterate over dictionary keys"

[test.python]
code =  '''
def sum_keys(d: dict) -> int:
    total = 0
    for key in d:
        total += key
    return total
'''

[test.assertions]
contains = ["for", "keys()"]
any_of = ["keys()", "iter()"]
```

```toml
[[test]]
name = "dict_iterate_items"
description = "Iterate over dictionary items"

[test.python]
code =  '''
def process_items(d: dict) -> list:
    result = []
    for key, value in d.items():
        result.append(f"{key}: {value}")
    return result
'''

[test.assertions]
contains = ["for", "iter()"]
```

### Dict Typing

```toml
[[test]]
name = "dict_typed_annotation"
description = "Typed dictionary annotation"

[test.python]
code =  '''
from typing import Dict

def typed_dict(data: Dict[str, int]) -> int:
    return data.get("key", 0)
'''

[test.assertions]
contains = ["HashMap<String, i32>"]
```

### Dict with Mutability

```toml
[[test]]
name = "dict_mutability_param"
description = "Dict parameter mutability detection"

[test.python]
code =  '''
def add_entry(d: dict, key: str, value: int) -> None:
    d[key] = value
'''

[test.assertions]
contains = ["&mut", "insert"]
```

## Notes

- Python `dict` → Rust `HashMap`
- `d["key"]` read → `d.get("key")` or `d["key"]` with index trait
- `d["key"] = value` → `d.insert("key", value)`
- Nested assignment requires `get_mut()` chain
- `key in d` → `d.contains_key(&key)`
- Dict comprehension → iterator chain with `.collect::<HashMap<_, _>>()`

## Acceptance Criteria

- [ ] Basic dict creation tests migrated
- [ ] Nested dict tests migrated
- [ ] Dict access tests migrated
- [ ] Dict method tests migrated
- [ ] Dict comprehension tests migrated
- [ ] Dict iteration tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Medium (nested dict access is complex)
