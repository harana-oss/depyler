# Migration Task: Slicing Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_slice_operations.rs | 184 | ~10 | `transpile_and_check` |
| test_slice_assignment_mutability.rs | ~120 | ~6 | `transpile_and_check` |
| test_indexings.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/09-slicing.toml`

## Tests to Migrate

### Basic Slicing

```toml
[[test]]
name = "slice_start_only"
description = "Slice with start index only (items[n:])"

[test.python]
code =  '''
def skip_first_n(items: list[int], n: int) -> list[int]:
    return items[n:]
'''

[test.assertions]
contains = ["skip(", "collect"]
any_of = ["&items[n..]", "skip(n)"]
```

```toml
[[test]]
name = "slice_end_only"
description = "Slice with end index only (items[:n])"

[test.python]
code =  '''
def first_n(items: list[int], n: int) -> list[int]:
    return items[:n]
'''

[test.assertions]
contains = ["take(", "collect"]
any_of = ["&items[..n]", "take(n)"]
```

```toml
[[test]]
name = "slice_start_end"
description = "Slice with both start and end (items[a:b])"

[test.python]
code =  '''
def middle(items: list[int], start: int, end: int) -> list[int]:
    return items[start:end]
'''

[test.assertions]
contains = ["skip(", "take("]
any_of = ["&items[start..end]", "skip(start).take("]
```

```toml
[[test]]
name = "slice_full_copy"
description = "Full slice copy (items[:])"

[test.python]
code =  '''
def copy_list(items: list[int]) -> list[int]:
    return items[:]
'''

[test.assertions]
contains = ["clone()"]
```

### Negative Indexing

```toml
[[test]]
name = "index_negative_one"
description = "Access last element with -1"

[test.python]
code =  '''
def get_last(items: list[int]) -> int:
    return items[-1]
'''

[test.assertions]
any_of = ["items[items.len() - 1]", "last()", "len() - 1"]
```

```toml
[[test]]
name = "index_negative_n"
description = "Access nth from end with -n"

[test.python]
code =  '''
def get_nth_from_end(items: list[int], n: int) -> int:
    return items[-n]
'''

[test.assertions]
contains = ["len()", "-"]
```

```toml
[[test]]
name = "slice_negative_start"
description = "Slice with negative start (items[-n:])"

[test.python]
code =  '''
def last_n(items: list[int], n: int) -> list[int]:
    return items[-n:]
'''

[test.assertions]
contains = ["len()", "-"]
any_of = ["skip(", "&items["]
```

```toml
[[test]]
name = "slice_negative_end"
description = "Slice with negative end (items[:-n])"

[test.python]
code =  '''
def all_but_last_n(items: list[int], n: int) -> list[int]:
    return items[:-n]
'''

[test.assertions]
contains = ["len()", "-"]
any_of = ["take(", "&items["]
```

### Step Slicing

```toml
[[test]]
name = "slice_step"
description = "Slice with step (items[::n])"

[test.python]
code =  '''
def every_nth(items: list[int], n: int) -> list[int]:
    return items[::n]
'''

[test.assertions]
contains = ["step_by("]
```

```toml
[[test]]
name = "slice_step_with_range"
description = "Slice with start, end, and step"

[test.python]
code =  '''
def range_step(items: list[int], start: int, end: int, step: int) -> list[int]:
    return items[start:end:step]
'''

[test.assertions]
contains = ["step_by("]
any_of = ["skip(", "take("]
```

### Reverse Slicing

```toml
[[test]]
name = "slice_reverse"
description = "Reverse with [::-1]"

[test.python]
code =  '''
def reverse_list(items: list[int]) -> list[int]:
    return items[::-1]
'''

[test.assertions]
contains = ["rev()", "collect"]
```

```toml
[[test]]
name = "slice_reverse_step"
description = "Reverse with step ([::-n])"

[test.python]
code =  '''
def reverse_every_nth(items: list[int], n: int) -> list[int]:
    return items[::-n]
'''

[test.assertions]
contains = ["rev()", "step_by("]
```

### Slice Assignment

```toml
[[test]]
name = "slice_assign_basic"
description = "Replace slice with new values"

[test.python]
code =  '''
def replace_middle(items: list[int]) -> list[int]:
    items[1:3] = [20, 30]
    return items
'''

[test.assertions]
any_of = ["splice(", "drain(", "extend("]
```

```toml
[[test]]
name = "slice_assign_start"
description = "Replace prefix with slice assignment"

[test.python]
code =  '''
def replace_start(items: list[int]) -> list[int]:
    items[:2] = [10, 15]
    return items
'''

[test.assertions]
any_of = ["splice(", "drain("]
```

```toml
[[test]]
name = "slice_assign_end"
description = "Replace suffix with slice assignment"

[test.python]
code =  '''
def replace_end(items: list[int]) -> list[int]:
    items[-2:] = [40, 50]
    return items
'''

[test.assertions]
any_of = ["splice(", "truncate("]
```

```toml
[[test]]
name = "slice_assign_grow"
description = "Slice assignment that grows list"

[test.python]
code =  '''
def grow_list(items: list[int]) -> list[int]:
    items[1:2] = [10, 20, 30, 40]
    return items
'''

[test.assertions]
any_of = ["splice(", "drain("]
```

```toml
[[test]]
name = "slice_assign_shrink"
description = "Slice assignment that shrinks list"

[test.python]
code =  '''
def shrink_list(items: list[int]) -> list[int]:
    items[1:4] = [99]
    return items
'''

[test.assertions]
any_of = ["splice(", "drain("]
```

### Slice Delete

```toml
[[test]]
name = "slice_delete"
description = "Delete slice with del"

[test.python]
code =  '''
def delete_middle(items: list[int]) -> list[int]:
    del items[1:3]
    return items
'''

[test.assertions]
any_of = ["drain(", "splice("]
```

### String Slicing (see also 05-string-operations.md)

```toml
[[test]]
name = "string_slice_basic"
description = "String slicing with range"

[test.python]
code =  '''
def substring(s: str, start: int, end: int) -> str:
    return s[start:end]
'''

[test.assertions]
contains = ["chars()", "skip(", "take("]
```

```toml
[[test]]
name = "string_slice_negative"
description = "String slicing with negative index"

[test.python]
code =  '''
def last_n_chars(s: str, n: int) -> str:
    return s[-n:]
'''

[test.assertions]
contains = ["chars()"]
any_of = ["skip(", "len()"]
```

## Notes

- Python slice syntax `[start:stop:step]` has multiple Rust translations
- Simple slices → Rust range indexing `&vec[a..b]`
- Complex slices → iterator chains with `skip()`, `take()`, `step_by()`
- Negative indices require length calculation
- `[::-1]` reverse → `.iter().rev().collect()`
- Slice assignment → `.splice()` or `.drain()` + extend
- String slicing requires `.chars()` iterator (not byte indexing)

## Acceptance Criteria

- [ ] Basic slice tests migrated
- [ ] Negative index tests migrated
- [ ] Step slice tests migrated
- [ ] Reverse slice tests migrated
- [ ] Slice assignment tests migrated
- [ ] String slice tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Medium (slice assignment is complex)
