# Migration Task: Iterator Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_iterator_deref.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/13-iterators.toml`

## Tests to Migrate

### Iterator Methods

```toml
[[test]]
name = "iterator_map"
description = "map() function on iterable"

[test.python]
code =  '''
def double_all(items: list[int]) -> list[int]:
    return list(map(lambda x: x * 2, items))
'''

[test.assertions]
contains = [".map(", "collect"]
```

```toml
[[test]]
name = "iterator_filter"
description = "filter() function on iterable"

[test.python]
code =  '''
def positives(items: list[int]) -> list[int]:
    return list(filter(lambda x: x > 0, items))
'''

[test.assertions]
contains = [".filter(", "collect"]
```

```toml
[[test]]
name = "iterator_zip"
description = "zip() function on iterables"

[test.python]
code =  '''
def pair_up(a: list[int], b: list[str]) -> list[tuple]:
    return list(zip(a, b))
'''

[test.assertions]
contains = [".zip(", "collect"]
```

```toml
[[test]]
name = "iterator_enumerate"
description = "enumerate() function"

[test.python]
code =  '''
def with_index(items: list[str]) -> list[tuple]:
    return list(enumerate(items))
'''

[test.assertions]
contains = [".enumerate(", "collect"]
```

```toml
[[test]]
name = "iterator_reversed"
description = "reversed() function"

[test.python]
code =  '''
def reverse_list(items: list[int]) -> list[int]:
    return list(reversed(items))
'''

[test.assertions]
contains = [".rev()", "collect"]
```

### Reduction Operations

```toml
[[test]]
name = "iterator_sum"
description = "sum() on iterable"

[test.python]
code =  '''
def total(items: list[int]) -> int:
    return sum(items)
'''

[test.assertions]
contains = [".sum()"]
any_of = ["iter().sum()", "sum::<"]
```

```toml
[[test]]
name = "iterator_min"
description = "min() on iterable"

[test.python]
code =  '''
def minimum(items: list[int]) -> int:
    return min(items)
'''

[test.assertions]
contains = [".min()"]
```

```toml
[[test]]
name = "iterator_max"
description = "max() on iterable"

[test.python]
code =  '''
def maximum(items: list[int]) -> int:
    return max(items)
'''

[test.assertions]
contains = [".max()"]
```

```toml
[[test]]
name = "iterator_any"
description = "any() on iterable"

[test.python]
code =  '''
def has_positive(items: list[int]) -> bool:
    return any(x > 0 for x in items)
'''

[test.assertions]
contains = [".any("]
```

```toml
[[test]]
name = "iterator_all"
description = "all() on iterable"

[test.python]
code =  '''
def all_positive(items: list[int]) -> bool:
    return all(x > 0 for x in items)
'''

[test.assertions]
contains = [".all("]
```

### Iterator Chaining

```toml
[[test]]
name = "iterator_chain"
description = "Chaining multiple iterator operations"

[test.python]
code =  '''
def process(items: list[int]) -> list[int]:
    return list(map(lambda x: x * 2, filter(lambda x: x > 0, items)))
'''

[test.assertions]
contains = [".filter(", ".map(", "collect"]
```

```toml
[[test]]
name = "iterator_take"
description = "Taking first n elements"

[test.python]
code =  '''
def first_n(items: list[int], n: int) -> list[int]:
    result = []
    for i, item in enumerate(items):
        if i >= n:
            break
        result.append(item)
    return result
'''

[test.assertions]
contains = [".take("]
```

```toml
[[test]]
name = "iterator_skip"
description = "Skipping first n elements"

[test.python]
code =  '''
def after_n(items: list[int], n: int) -> list[int]:
    return items[n:]
'''

[test.assertions]
contains = [".skip("]
```

### Iterator with Deref

```toml
[[test]]
name = "iterator_deref_map"
description = "Iterator map with dereference"

[test.python]
code =  '''
def squares(items: list[int]) -> list[int]:
    return [x * x for x in items]
'''

[test.assertions]
contains = [".map("]
any_of = ["|&x|", "|x|", "*x"]
```

### Iterator Collection

```toml
[[test]]
name = "iterator_collect_vec"
description = "Collecting iterator to Vec"

[test.python]
code =  '''
def to_list(items) -> list:
    return list(items)
'''

[test.assertions]
contains = ["collect::<Vec<"]
```

```toml
[[test]]
name = "iterator_collect_set"
description = "Collecting iterator to HashSet"

[test.python]
code =  '''
def unique(items: list[int]) -> set[int]:
    return set(items)
'''

[test.assertions]
contains = ["collect::<HashSet<"]
```

## Notes

- Python iterators → Rust `Iterator` trait
- `map(f, iter)` → `iter.map(f)`
- `filter(f, iter)` → `iter.filter(f)`
- `zip(a, b)` → `a.iter().zip(b.iter())`
- `enumerate(iter)` → `iter.enumerate()`
- `sum(iter)` → `iter.sum()`
- Iterator methods are lazy; collect to realize

## Acceptance Criteria

- [ ] Iterator method tests migrated
- [ ] Reduction operation tests migrated
- [ ] Iterator chaining tests migrated
- [ ] Iterator collection tests migrated

## Estimated Effort

**Time**: 2 hours
**Risk**: Low
