# Migration Task: Comprehension Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_list_comprehension.rs | 220 | ~12 | `transpile_and_check` |

## Target File

`tests/toml/10-comprehensions.toml`

## Tests to Migrate

### Basic List Comprehension

```toml
[[test]]
name = "comprehension_simple"
description = "Simple list comprehension"

[test.python]
code =  '''
def squares(n: int) -> list[int]:
    return [x * x for x in range(n)]
'''

[test.assertions]
contains = ["map(", "collect"]
any_of = ["iter()", "into_iter()"]
```

```toml
[[test]]
name = "comprehension_with_filter"
description = "List comprehension with if condition"

[test.python]
code =  '''
def evens(n: int) -> list[int]:
    return [x for x in range(n) if x % 2 == 0]
'''

[test.assertions]
contains = ["filter(", "collect"]
```

```toml
[[test]]
name = "comprehension_transform_filter"
description = "List comprehension with transform and filter"

[test.python]
code =  '''
def even_squares(n: int) -> list[int]:
    return [x * x for x in range(n) if x % 2 == 0]
'''

[test.assertions]
contains = ["filter(", "map(", "collect"]
```

### String Comprehension

```toml
[[test]]
name = "comprehension_string_transform"
description = "List comprehension with string transformation"

[test.python]
code =  '''
def uppercase_words(words: list[str]) -> list[str]:
    return [word.upper() for word in words]
'''

[test.assertions]
contains = ["map(", "to_uppercase", "collect"]
```

```toml
[[test]]
name = "comprehension_string_filter"
description = "List comprehension filtering strings"

[test.python]
code =  '''
def long_words(words: list[str]) -> list[str]:
    return [word for word in words if len(word) > 4]
'''

[test.assertions]
contains = ["filter(", "len()", "collect"]
```

### Nested Comprehension

```toml
[[test]]
name = "comprehension_nested"
description = "Nested list comprehension"

[test.python]
code =  '''
def matrix(n: int) -> list[list[int]]:
    return [[i * j for j in range(n)] for i in range(n)]
'''

[test.assertions]
contains = ["map(", "collect"]
```

```toml
[[test]]
name = "comprehension_flatten"
description = "Flattening nested lists"

[test.python]
code =  '''
def flatten(matrix: list[list[int]]) -> list[int]:
    return [item for row in matrix for item in row]
'''

[test.assertions]
contains = ["flat_map", "flatten"]
any_of = ["flat_map(", "flatten()"]
```

### Dict Comprehension

```toml
[[test]]
name = "dict_comprehension_basic"
description = "Basic dict comprehension"

[test.python]
code =  '''
def squares_dict(n: int) -> dict[int, int]:
    return {x: x * x for x in range(n)}
'''

[test.assertions]
contains = ["HashMap", "collect"]
```

```toml
[[test]]
name = "dict_comprehension_filter"
description = "Dict comprehension with filter"

[test.python]
code =  '''
def even_squares_dict(n: int) -> dict[int, int]:
    return {x: x * x for x in range(n) if x % 2 == 0}
'''

[test.assertions]
contains = ["filter(", "HashMap", "collect"]
```

```toml
[[test]]
name = "dict_comprehension_from_list"
description = "Dict comprehension from list with enumerate"

[test.python]
code =  '''
def index_dict(items: list[str]) -> dict[int, str]:
    return {i: item for i, item in enumerate(items)}
'''

[test.assertions]
contains = ["enumerate()", "collect"]
```

### Set Comprehension

```toml
[[test]]
name = "set_comprehension_basic"
description = "Basic set comprehension"

[test.python]
code =  '''
def unique_squares(items: list[int]) -> set[int]:
    return {x * x for x in items}
'''

[test.assertions]
contains = ["HashSet", "collect"]
```

```toml
[[test]]
name = "set_comprehension_filter"
description = "Set comprehension with filter"

[test.python]
code =  '''
def positive_unique(items: list[int]) -> set[int]:
    return {x for x in items if x > 0}
'''

[test.assertions]
contains = ["filter(", "HashSet", "collect"]
```

### Generator Expression to Iterator

```toml
[[test]]
name = "generator_expression_sum"
description = "Generator expression in sum()"

[test.python]
code =  '''
def sum_squares(n: int) -> int:
    return sum(x * x for x in range(n))
'''

[test.assertions]
contains = ["map(", "sum()"]
```

```toml
[[test]]
name = "generator_expression_any"
description = "Generator expression in any()"

[test.python]
code =  '''
def has_positive(items: list[int]) -> bool:
    return any(x > 0 for x in items)
'''

[test.assertions]
contains = ["any("]
```

```toml
[[test]]
name = "generator_expression_all"
description = "Generator expression in all()"

[test.python]
code =  '''
def all_positive(items: list[int]) -> bool:
    return all(x > 0 for x in items)
'''

[test.assertions]
contains = ["all("]
```

### Complex Expressions

```toml
[[test]]
name = "comprehension_complex_expr"
description = "Comprehension with complex expression"

[test.python]
code =  '''
def process(items: list[int]) -> list[int]:
    return [x * 2 + 1 if x > 0 else 0 for x in items]
'''

[test.assertions]
contains = ["map(", "if", "else"]
```

```toml
[[test]]
name = "comprehension_method_call"
description = "Comprehension with method calls"

[test.python]
code =  '''
def lengths(words: list[str]) -> list[int]:
    return [len(word) for word in words]
'''

[test.assertions]
contains = ["map(", "len()"]
```

### Multiple Conditions

```toml
[[test]]
name = "comprehension_multiple_conditions"
description = "Comprehension with multiple if conditions"

[test.python]
code =  '''
def filter_range(items: list[int], low: int, high: int) -> list[int]:
    return [x for x in items if x >= low if x <= high]
'''

[test.assertions]
contains = ["filter("]
any_of = ["&&", ".filter("]
```

## Notes

- List comprehension `[expr for x in iter]` → `.map(|x| expr).collect()`
- Filter `[x for x in iter if cond]` → `.filter(|x| cond).collect()`
- Combined `[expr for x in iter if cond]` → `.filter().map().collect()`
- Dict comprehension → `.collect::<HashMap<_, _>>()`
- Set comprehension → `.collect::<HashSet<_>>()`
- Nested loops → `.flat_map()` or nested `.map()`
- Generator expressions in `sum/any/all` don't need `.collect()`

## Acceptance Criteria

- [ ] Basic list comprehension tests migrated
- [ ] Filter comprehension tests migrated
- [ ] Nested comprehension tests migrated
- [ ] Dict comprehension tests migrated
- [ ] Set comprehension tests migrated
- [ ] Generator expression tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Low-Medium
