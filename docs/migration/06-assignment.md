# Migration Task: Assignment Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_assignment.rs | 22 | 1 | `transpile_and_check` |
| test_stmt_gen_assign_coverage.rs | ~300 | ~18 | `transpile_and_check` |
| test_stmt_gen_assign_index_coverage.rs | ~200 | ~12 | `transpile_and_check` |
| test_stmt_gen_assign_symbol_coverage.rs | ~150 | ~8 | `transpile_and_check` |
| test_slice_assignment_mutability.rs | ~120 | ~6 | `transpile_and_check` |
| test_uninitialized_declarations.rs | ~100 | ~5 | `transpile_and_check` |
| test_optional_augassign.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/06-assignment.toml`

## Tests to Migrate

### Basic Variable Assignment

```toml
[[test]]
name = "assign_integer"
description = "Simple integer assignment"

[test.python]
code =  '''
def assign_int():
    x = 42
    return x
'''

[test.assertions]
contains = ["let x = 42"]
```

```toml
[[test]]
name = "assign_string"
description = "Simple string assignment"

[test.python]
code =  '''
def assign_str():
    s = "hello"
    return s
'''

[test.assertions]
contains = ["let s"]
```

```toml
[[test]]
name = "assign_list"
description = "List assignment"

[test.python]
code =  '''
def assign_list():
    items = [1, 2, 3]
    return items
'''

[test.assertions]
contains = ["let items = vec!"]
```

```toml
[[test]]
name = "assign_dict"
description = "Dictionary assignment"

[test.python]
code =  '''
def assign_dict():
    d = {}
    return d
'''

[test.assertions]
contains = ["HashMap"]
```

### Augmented Assignment

```toml
[[test]]
name = "augassign_add"
description = "+= operator"

[test.python]
code =  '''
def add_assign(x: int) -> int:
    x += 10
    return x
'''

[test.assertions]
contains = ["x += 10", "x = x + 10"]
any_of = ["+=", "= x +"]
```

```toml
[[test]]
name = "augassign_subtract"
description = "-= operator"

[test.python]
code =  '''
def sub_assign(x: int) -> int:
    x -= 5
    return x
'''

[test.assertions]
any_of = ["-=", "= x -"]
```

```toml
[[test]]
name = "augassign_multiply"
description = "*= operator"

[test.python]
code =  '''
def mul_assign(x: int) -> int:
    x *= 2
    return x
'''

[test.assertions]
any_of = ["*=", "= x *"]
```

```toml
[[test]]
name = "augassign_divide"
description = "/= operator"

[test.python]
code =  '''
def div_assign(x: float) -> float:
    x /= 2.0
    return x
'''

[test.assertions]
any_of = ["/=", "= x /"]
```

```toml
[[test]]
name = "augassign_floor_divide"
description = "//= operator"

[test.python]
code =  '''
def floor_div_assign(x: int) -> int:
    x //= 3
    return x
'''

[test.assertions]
any_of = ["/=", "= x /"]
```

```toml
[[test]]
name = "augassign_modulo"
description = "%= operator"

[test.python]
code =  '''
def mod_assign(x: int) -> int:
    x %= 7
    return x
'''

[test.assertions]
any_of = ["%=", "= x %"]
```

### Tuple Unpacking

```toml
[[test]]
name = "tuple_unpack_basic"
description = "Basic tuple unpacking"

[test.python]
code =  '''
def unpack_tuple():
    a, b = (1, 2)
    return a + b
'''

[test.assertions]
contains = ["let (a, b)"]
```

```toml
[[test]]
name = "tuple_swap"
description = "Tuple swap idiom"

[test.python]
code =  '''
def swap(a: int, b: int) -> tuple[int, int]:
    a, b = b, a
    return (a, b)
'''

[test.assertions]
any_of = ["let (a, b)", "std::mem::swap", "_tmp"]
```

```toml
[[test]]
name = "tuple_swap_array_indices"
description = "Swapping array elements via tuple"

[test.python]
code =  '''
def swap_elements():
    a = [1, 2, 3]
    a[0], a[2] = a[2], a[0]
    return a
'''

[test.assertions]
contains = ["vec!", "_swap_tmp", "a[0", "a[2"]
any_of = ["swap", "tmp"]
```

### Indexed Assignment

```toml
[[test]]
name = "list_index_assign"
description = "List element assignment by index"

[test.python]
code =  '''
def set_element():
    items = [1, 2, 3]
    items[1] = 10
    return items
'''

[test.assertions]
contains = ["items[1]", "10"]
```

```toml
[[test]]
name = "dict_key_assign"
description = "Dictionary key assignment"

[test.python]
code =  '''
def set_key():
    d = {}
    d["key"] = "value"
    return d
'''

[test.assertions]
contains = ["insert", "key"]
```

```toml
[[test]]
name = "nested_dict_assign"
description = "Nested dictionary assignment"

[test.python]
code =  '''
def set_nested():
    d = {}
    d["outer"] = {}
    d["outer"]["inner"] = "value"
    return d
'''

[test.assertions]
contains = ["get_mut", "unwrap()"]
count = { "get_mut" = { min = 1 } }
```

```toml
[[test]]
name = "deep_nested_dict_assign"
description = "Deeply nested dictionary assignment"

[test.python]
code =  '''
def set_deep():
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

### Multiple Assignment

```toml
[[test]]
name = "multiple_assign"
description = "Multiple assignment on same line"

[test.python]
code =  '''
def multi_assign():
    a = b = c = 0
    return a + b + c
'''

[test.assertions]
contains = ["let"]
```

### Annotated Assignment

```toml
[[test]]
name = "annotated_assign"
description = "Type-annotated assignment"

[test.python]
code =  '''
def typed_assign():
    x: int = 42
    return x
'''

[test.assertions]
contains = ["let x", "i32", "42"]
```

```toml
[[test]]
name = "annotated_assign_complex"
description = "Complex type-annotated assignment"

[test.python]
code =  '''
from typing import Dict, List

def typed_complex():
    data: Dict[str, List[int]] = {}
    return data
'''

[test.assertions]
contains = ["HashMap", "Vec"]
```

### Slice Assignment

```toml
[[test]]
name = "slice_assign_basic"
description = "Slice assignment"

[test.python]
code =  '''
def slice_assign():
    items = [1, 2, 3, 4, 5]
    items[1:3] = [20, 30]
    return items
'''

[test.assertions]
contains = ["splice", "drain"]
any_of = ["splice", "drain", "extend"]
```

### Mutability Detection

```toml
[[test]]
name = "mutability_detect"
description = "Variable mutability detection"

[test.python]
code =  '''
def mutate_var():
    x = 0
    x = 1
    x = 2
    return x
'''

[test.assertions]
contains = ["let mut x"]
```

```toml
[[test]]
name = "immutable_var"
description = "Immutable variable (single assignment)"

[test.python]
code =  '''
def no_mutate():
    x = 42
    return x
'''

[test.assertions]
contains = ["let x"]
not_contains = ["let mut x"]
```

## Notes

- Python allows reassignment; Rust requires `mut` for reassignment
- Augmented assignment (`+=`, etc.) may expand to full expression
- Tuple unpacking becomes pattern matching
- Dictionary assignment uses `.insert()`
- Nested dict assignment requires `.get_mut()` chains
- Slice assignment may use `.splice()` or `.drain()` + `.extend()`

## Acceptance Criteria

- [ ] Basic assignment tests migrated
- [ ] Augmented assignment tests migrated
- [ ] Tuple unpacking tests migrated
- [ ] Indexed assignment tests migrated
- [ ] Slice assignment tests migrated
- [ ] Mutability detection tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Medium (complex cases like nested dict assignment)
