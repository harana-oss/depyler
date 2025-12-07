# Migration Task: Lists and Arrays Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_array_generation.rs | ~150 | ~8 | `transpile_and_check` |
| test_array_literal_regression.rs | ~80 | ~4 | `transpile_and_check` |
| test_array_unit.rs | ~200 | ~12 | `transpile_and_check` |
| test_list_methods_unit.rs | ~250 | ~15 | `transpile_and_check` |
| test_collection_operations.rs | ~180 | ~10 | `transpile_and_check` |

## Target File

`tests/toml/08-lists-arrays.toml`

## Tests to Migrate

### List Creation

```toml
[[test]]
name = "list_empty"
description = "Create empty list"

[test.python]
code =  '''
def create_empty() -> list:
    return []
'''

[test.assertions]
contains = ["Vec::new()"]
any_of = ["Vec::new()", "vec![]"]
```

```toml
[[test]]
name = "list_literal"
description = "Create list with literal values"

[test.python]
code =  '''
def create_list() -> list[int]:
    return [1, 2, 3, 4, 5]
'''

[test.assertions]
contains = ["vec![1, 2, 3, 4, 5]"]
```

```toml
[[test]]
name = "list_typed"
description = "Typed list annotation"

[test.python]
code =  '''
from typing import List

def typed_list(items: List[int]) -> int:
    return len(items)
'''

[test.assertions]
contains = ["Vec<i32>"]
```

### List Methods

```toml
[[test]]
name = "list_append"
description = "List append() method"

[test.python]
code =  '''
def add_item(items: list[int], item: int) -> None:
    items.append(item)
'''

[test.assertions]
contains = ["push("]
```

```toml
[[test]]
name = "list_extend"
description = "List extend() method"

[test.python]
code =  '''
def add_items(items: list[int], more: list[int]) -> None:
    items.extend(more)
'''

[test.assertions]
contains = ["extend("]
```

```toml
[[test]]
name = "list_insert"
description = "List insert() method"

[test.python]
code =  '''
def insert_at(items: list[int], index: int, item: int) -> None:
    items.insert(index, item)
'''

[test.assertions]
contains = ["insert("]
```

```toml
[[test]]
name = "list_pop"
description = "List pop() method"

[test.python]
code =  '''
def pop_last(items: list[int]) -> int:
    return items.pop()
'''

[test.assertions]
contains = ["pop()"]
```

```toml
[[test]]
name = "list_pop_index"
description = "List pop() with index"

[test.python]
code =  '''
def pop_at(items: list[int], index: int) -> int:
    return items.pop(index)
'''

[test.assertions]
contains = ["remove("]
```

```toml
[[test]]
name = "list_remove"
description = "List remove() method (by value)"

[test.python]
code =  '''
def remove_item(items: list[int], item: int) -> None:
    items.remove(item)
'''

[test.assertions]
contains = ["retain("]
any_of = ["retain(", "position(", "remove("]
```

```toml
[[test]]
name = "list_clear"
description = "List clear() method"

[test.python]
code =  '''
def clear_list(items: list[int]) -> None:
    items.clear()
'''

[test.assertions]
contains = ["clear()"]
```

```toml
[[test]]
name = "list_reverse"
description = "List reverse() method"

[test.python]
code =  '''
def reverse_list(items: list[int]) -> None:
    items.reverse()
'''

[test.assertions]
contains = ["reverse()"]
```

```toml
[[test]]
name = "list_sort"
description = "List sort() method"

[test.python]
code =  '''
def sort_list(items: list[int]) -> None:
    items.sort()
'''

[test.assertions]
contains = ["sort()"]
```

```toml
[[test]]
name = "list_sort_reverse"
description = "List sort(reverse=True)"

[test.python]
code =  '''
def sort_descending(items: list[int]) -> None:
    items.sort(reverse=True)
'''

[test.assertions]
contains = ["sort"]
any_of = ["sort_by(|a, b| b.cmp(a))", "reverse()"]
```

```toml
[[test]]
name = "list_index"
description = "List index() method"

[test.python]
code =  '''
def find_index(items: list[int], item: int) -> int:
    return items.index(item)
'''

[test.assertions]
contains = ["position("]
any_of = ["position(", "iter()"]
```

```toml
[[test]]
name = "list_count"
description = "List count() method"

[test.python]
code =  '''
def count_item(items: list[int], item: int) -> int:
    return items.count(item)
'''

[test.assertions]
contains = ["filter(", "count()"]
```

### List Indexing

```toml
[[test]]
name = "list_index_positive"
description = "List positive index access"

[test.python]
code =  '''
def get_first(items: list[int]) -> int:
    return items[0]
'''

[test.assertions]
contains = ["items[0]"]
```

```toml
[[test]]
name = "list_index_negative"
description = "List negative index access"

[test.python]
code =  '''
def get_last(items: list[int]) -> int:
    return items[-1]
'''

[test.assertions]
contains = ["len()", "-1"]
any_of = ["items[items.len() - 1]", "last()"]
```

```toml
[[test]]
name = "list_index_assign"
description = "List index assignment"

[test.python]
code =  '''
def set_first(items: list[int], value: int) -> None:
    items[0] = value
'''

[test.assertions]
contains = ["items[0] = value"]
```

### Built-in Functions

```toml
[[test]]
name = "list_len"
description = "len() on list"

[test.python]
code =  '''
def get_length(items: list[int]) -> int:
    return len(items)
'''

[test.assertions]
contains = ["len()"]
```

```toml
[[test]]
name = "list_min"
description = "min() on list"

[test.python]
code =  '''
def get_min(items: list[int]) -> int:
    return min(items)
'''

[test.assertions]
contains = ["min()"]
any_of = ["iter().min()", "min()"]
```

```toml
[[test]]
name = "list_max"
description = "max() on list"

[test.python]
code =  '''
def get_max(items: list[int]) -> int:
    return max(items)
'''

[test.assertions]
contains = ["max()"]
any_of = ["iter().max()", "max()"]
```

```toml
[[test]]
name = "list_sum"
description = "sum() on list"

[test.python]
code =  '''
def get_sum(items: list[int]) -> int:
    return sum(items)
'''

[test.assertions]
contains = ["sum()"]
any_of = ["iter().sum()", "sum::<i32>()"]
```

```toml
[[test]]
name = "list_sorted"
description = "sorted() on list"

[test.python]
code =  '''
def get_sorted(items: list[int]) -> list[int]:
    return sorted(items)
'''

[test.assertions]
contains = ["sorted", "clone"]
any_of = ["sorted()", "sort()", "clone()"]
```

```toml
[[test]]
name = "list_reversed"
description = "reversed() on list"

[test.python]
code =  '''
def get_reversed(items: list[int]) -> list[int]:
    return list(reversed(items))
'''

[test.assertions]
contains = ["rev()"]
```

### List Membership

```toml
[[test]]
name = "list_in_operator"
description = "in operator for list membership"

[test.python]
code =  '''
def is_in_list(items: list[int], item: int) -> bool:
    return item in items
'''

[test.assertions]
contains = ["contains("]
```

```toml
[[test]]
name = "list_not_in_operator"
description = "not in operator for list"

[test.python]
code =  '''
def is_not_in_list(items: list[int], item: int) -> bool:
    return item not in items
'''

[test.assertions]
contains = ["contains("]
any_of = ["!items.contains(", "!contains"]
```

### List Concatenation

```toml
[[test]]
name = "list_concat"
description = "List concatenation with +"

[test.python]
code =  '''
def concat_lists(a: list[int], b: list[int]) -> list[int]:
    return a + b
'''

[test.assertions]
contains = ["extend", "clone"]
any_of = ["extend(", "chain(", "concat"]
```

```toml
[[test]]
name = "list_repeat"
description = "List repetition with *"

[test.python]
code =  '''
def repeat_list(items: list[int], n: int) -> list[int]:
    return items * n
'''

[test.assertions]
contains = ["repeat(", "flatten"]
any_of = ["repeat(", "cycle(", "take("]
```

## Notes

- Python `list` → Rust `Vec`
- `list.append(x)` → `vec.push(x)`
- `list.pop()` → `vec.pop()`
- `list.remove(x)` → find + remove (more complex)
- `list[-1]` → `vec[vec.len() - 1]` or `vec.last()`
- `sorted(list)` → clone + sort
- `x in list` → `list.contains(&x)`

## Acceptance Criteria

- [ ] List creation tests migrated
- [ ] List method tests migrated
- [ ] List indexing tests migrated
- [ ] Built-in function tests migrated
- [ ] List operator tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Low-Medium
