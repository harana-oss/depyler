# Migration Task: Generator Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_generators.rs | 471 | ~30 | `transpile_and_check` |
| test_generator_compilations.rs | ~150 | ~8 | `transpile_and_check` |
| test_generator_yield_analysis_coverage.rs | ~200 | ~12 | `transpile_and_check` |

## Target File

`tests/toml/11-generators.toml`

## Tests to Migrate

### Basic Generators

```toml
[[test]]
name = "generator_simple_yield"
description = "Simple generator with single yield"

[test.python]
code =  '''
def simple_generator():
    yield 1
'''

[test.assertions]
any_of = ["fn simple_generator", "struct SimpleGenerator"]
contains = ["Iterator"]
```

```toml
[[test]]
name = "generator_multiple_yields"
description = "Generator with multiple yield statements"

[test.python]
code =  '''
def count_to_three():
    yield 1
    yield 2
    yield 3
'''

[test.assertions]
any_of = ["count_to_three", "CountToThree"]
```

```toml
[[test]]
name = "generator_with_loop"
description = "Generator using while loop"

[test.python]
code =  '''
def count_up(n: int):
    i = 0
    while i < n:
        yield i
        i = i + 1
'''

[test.assertions]
any_of = ["count_up", "CountUp"]
```

```toml
[[test]]
name = "generator_with_for"
description = "Generator using for loop with range"

[test.python]
code =  '''
def range_generator(n: int):
    for i in range(n):
        yield i
'''

[test.assertions]
any_of = ["range_generator", "RangeGenerator"]
```

```toml
[[test]]
name = "generator_with_condition"
description = "Generator with conditional yield"

[test.python]
code =  '''
def even_numbers(n: int):
    for i in range(n):
        if i % 2 == 0:
            yield i
'''

[test.assertions]
any_of = ["even_numbers", "EvenNumbers"]
```

### Generator Parameters

```toml
[[test]]
name = "generator_with_params"
description = "Generator with parameters"

[test.python]
code =  '''
def repeat_value(value: int, times: int):
    for _ in range(times):
        yield value
'''

[test.assertions]
contains = ["value", "times"]
```

```toml
[[test]]
name = "generator_with_start_stop"
description = "Generator with start and stop parameters"

[test.python]
code =  '''
def range_gen(start: int, stop: int):
    i = start
    while i < stop:
        yield i
        i += 1
'''

[test.assertions]
contains = ["start", "stop"]
```

### Generator Expressions

```toml
[[test]]
name = "generator_expression_basic"
description = "Generator expression syntax"

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
name = "generator_expression_filter"
description = "Generator expression with filter"

[test.python]
code =  '''
def sum_even_squares(n: int) -> int:
    return sum(x * x for x in range(n) if x % 2 == 0)
'''

[test.assertions]
contains = ["filter(", "map(", "sum()"]
```

### Stateful Generators

```toml
[[test]]
name = "generator_stateful"
description = "Stateful generator tracking count"

[test.python]
code =  '''
def counter(start: int):
    count = start
    while True:
        yield count
        count += 1
'''

[test.assertions]
contains = ["count"]
any_of = ["struct", "state"]
```

```toml
[[test]]
name = "generator_fibonacci"
description = "Fibonacci generator"

[test.python]
code =  '''
def fibonacci():
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b
'''

[test.assertions]
contains = ["yield", "Iterator"]
any_of = ["fibonacci", "Fibonacci"]
```

### Generator Consumption

```toml
[[test]]
name = "generator_to_list"
description = "Converting generator to list"

[test.python]
code =  '''
def make_list(n: int) -> list[int]:
    return list(x * x for x in range(n))
'''

[test.assertions]
contains = ["collect"]
```

```toml
[[test]]
name = "generator_next"
description = "Using next() on generator"

[test.python]
code =  '''
def first_even(items: list[int]) -> int:
    return next(x for x in items if x % 2 == 0)
'''

[test.assertions]
contains = ["filter(", "next()"]
any_of = ["find(", "next()"]
```

### Yield From (Python 3.3+)

```toml
[[test]]
name = "generator_yield_from"
description = "yield from delegation"

[test.python]
code =  '''
def flatten_gen(lists):
    for lst in lists:
        yield from lst
'''

[test.assertions]
contains = ["flatten"]
any_of = ["flat_map(", "flatten()"]
```

### Generator with Return

```toml
[[test]]
name = "generator_with_return"
description = "Generator with return value"

[test.python]
code =  '''
def limited_counter(limit: int):
    count = 0
    while count < limit:
        yield count
        count += 1
    return count
'''

[test.assertions]
any_of = ["StopIteration", "return", "None"]
```

### Generator Iterator Protocol

```toml
[[test]]
name = "generator_iterator_trait"
description = "Generator implementing Iterator trait"

[test.python]
code =  '''
def squares_gen(n: int):
    for i in range(n):
        yield i * i
'''

[test.assertions]
contains = ["Iterator"]
any_of = ["impl Iterator", "Item ="]
```

```toml
[[test]]
name = "generator_for_loop"
description = "Using generator in for loop"

[test.python]
code =  '''
def sum_generator(gen) -> int:
    total = 0
    for value in gen:
        total += value
    return total
'''

[test.assertions]
contains = ["for", "iter()"]
```

## Notes

- Python generators → Rust Iterator implementations
- Simple generators may use `std::iter::once` or `std::iter::from_fn`
- Complex generators become state machine structs implementing `Iterator`
- `yield` becomes `Some(value)` in `next()` method
- Generator expressions → iterator chains
- `yield from` → `flat_map()` or manual delegation
- Infinite generators need careful handling (take, etc.)

## Acceptance Criteria

- [ ] Basic generator tests migrated
- [ ] Generator parameter tests migrated
- [ ] Generator expression tests migrated
- [ ] Stateful generator tests migrated
- [ ] Generator consumption tests migrated
- [ ] yield from tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: High (generators have complex state machine translation)
