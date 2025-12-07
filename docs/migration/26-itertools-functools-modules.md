# Migration Task: Itertools and Functools Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_itertools_unit.rs | ~200 | ~10 | `transpile_and_check` |
| test_functools_unit.rs | ~150 | ~8 | `transpile_and_check` |
| test_functionals.rs | ~180 | ~9 | `transpile_and_check` |

## Target File

`tests/toml/26-itertools-functools-modules.toml`

## Tests to Migrate

### Itertools - Combinations

```toml
[[test]]
name = "itertools_chain"
description = "itertools.chain() iterator chaining"

[test.python]
code =  '''
from itertools import chain

def combine_lists(a: list[int], b: list[int]) -> list[int]:
    return list(chain(a, b))
'''

[test.assertions]
any_of = [".chain(", ".into_iter"]
```

```toml
[[test]]
name = "itertools_combinations"
description = "itertools.combinations() n-choose-k"

[test.python]
code =  '''
from itertools import combinations

def all_pairs(items: list[int]) -> list[tuple[int, int]]:
    return list(combinations(items, 2))
'''

[test.assertions]
any_of = ["combinations(", "itertools"]
```

```toml
[[test]]
name = "itertools_permutations"
description = "itertools.permutations() all orderings"

[test.python]
code =  '''
from itertools import permutations

def all_orderings(items: list[int]) -> list[tuple[int, ...]]:
    return list(permutations(items))
'''

[test.assertions]
any_of = ["permutations(", "itertools"]
```

### Itertools - Filtering

```toml
[[test]]
name = "itertools_takewhile"
description = "itertools.takewhile() conditional take"

[test.python]
code =  '''
from itertools import takewhile

def take_positives(nums: list[int]) -> list[int]:
    return list(takewhile(lambda x: x > 0, nums))
'''

[test.assertions]
any_of = [".take_while(", "take_while"]
```

```toml
[[test]]
name = "itertools_dropwhile"
description = "itertools.dropwhile() conditional skip"

[test.python]
code =  '''
from itertools import dropwhile

def skip_zeros(nums: list[int]) -> list[int]:
    return list(dropwhile(lambda x: x == 0, nums))
'''

[test.assertions]
any_of = [".skip_while(", "skip_while"]
```

```toml
[[test]]
name = "itertools_filter"
description = "itertools filterfalse and filter"

[test.python]
code =  '''
from itertools import filterfalse

def get_odd(nums: list[int]) -> list[int]:
    return list(filterfalse(lambda x: x % 2 == 0, nums))
'''

[test.assertions]
any_of = [".filter(", "filter"]
```

### Itertools - Grouping

```toml
[[test]]
name = "itertools_groupby"
description = "itertools.groupby() grouping"

[test.python]
code =  '''
from itertools import groupby

def group_by_first_char(words: list[str]) -> dict[str, list[str]]:
    result = {}
    for k, g in groupby(sorted(words), key=lambda x: x[0]):
        result[k] = list(g)
    return result
'''

[test.assertions]
any_of = ["group_by(", "GroupBy", "HashMap"]
```

### Itertools - Accumulation

```toml
[[test]]
name = "itertools_accumulate"
description = "itertools.accumulate() running sum"

[test.python]
code =  '''
from itertools import accumulate

def running_sum(nums: list[int]) -> list[int]:
    return list(accumulate(nums))
'''

[test.assertions]
any_of = [".scan(", "scan"]
```

### Itertools - Infinite

```toml
[[test]]
name = "itertools_count"
description = "itertools.count() infinite counter"

[test.python]
code =  '''
from itertools import count, islice

def first_n_from(start: int, n: int) -> list[int]:
    return list(islice(count(start), n))
'''

[test.assertions]
any_of = ["(start..)", ".take("]
```

```toml
[[test]]
name = "itertools_cycle"
description = "itertools.cycle() repeating iterator"

[test.python]
code =  '''
from itertools import cycle, islice

def repeat_pattern(items: list[int], n: int) -> list[int]:
    return list(islice(cycle(items), n))
'''

[test.assertions]
any_of = [".cycle(", "cycle"]
```

```toml
[[test]]
name = "itertools_repeat"
description = "itertools.repeat() single value repetition"

[test.python]
code =  '''
from itertools import repeat

def make_n_copies(value: int, n: int) -> list[int]:
    return list(repeat(value, n))
'''

[test.assertions]
any_of = ["std::iter::repeat", ".take("]
```

### Functools

```toml
[[test]]
name = "functools_reduce"
description = "functools.reduce() fold operation"

[test.python]
code =  '''
from functools import reduce

def sum_all(nums: list[int]) -> int:
    return reduce(lambda a, b: a + b, nums, 0)
'''

[test.assertions]
any_of = [".fold(", "fold"]
```

```toml
[[test]]
name = "functools_reduce_product"
description = "functools.reduce() for product"

[test.python]
code =  '''
from functools import reduce

def product(nums: list[int]) -> int:
    return reduce(lambda a, b: a * b, nums, 1)
'''

[test.assertions]
any_of = [".fold(", ".product("]
```

```toml
[[test]]
name = "functools_partial"
description = "functools.partial() partial application"

[test.python]
code =  '''
from functools import partial

def make_adder(n: int):
    return partial(lambda a, b: a + b, n)
'''

[test.assertions]
any_of = ["move |", "|", "closure"]
```

```toml
[[test]]
name = "functools_lru_cache"
description = "functools.lru_cache() memoization"

[test.python]
code =  '''
from functools import lru_cache

@lru_cache(maxsize=128)
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)
'''

[test.assertions]
any_of = ["HashMap", "cache", "memo"]
```

### Map/Filter/Reduce

```toml
[[test]]
name = "builtin_map"
description = "Built-in map() function"

[test.python]
code =  '''
def double_all(nums: list[int]) -> list[int]:
    return list(map(lambda x: x * 2, nums))
'''

[test.assertions]
any_of = [".map(|", ".map("]
```

```toml
[[test]]
name = "builtin_filter"
description = "Built-in filter() function"

[test.python]
code =  '''
def evens_only(nums: list[int]) -> list[int]:
    return list(filter(lambda x: x % 2 == 0, nums))
'''

[test.assertions]
any_of = [".filter(|", ".filter("]
```

```toml
[[test]]
name = "zip_iteration"
description = "Built-in zip() function"

[test.python]
code =  '''
def add_pairwise(a: list[int], b: list[int]) -> list[int]:
    return [x + y for x, y in zip(a, b)]
'''

[test.assertions]
any_of = [".zip(", "zip"]
```

```toml
[[test]]
name = "enumerate_iteration"
description = "Built-in enumerate() function"

[test.python]
code =  '''
def with_indices(items: list[str]) -> list[tuple[int, str]]:
    return list(enumerate(items))
'''

[test.assertions]
any_of = [".enumerate(", "enumerate"]
```

## Notes

- Python `itertools` → Rust `Iterator` methods and `itertools` crate
- `chain()` → `.chain()`
- `takewhile()` → `.take_while()`
- `dropwhile()` → `.skip_while()`
- `accumulate()` → `.scan()`
- `count()` → `(start..)`
- `cycle()` → `.cycle()`
- Python `functools.reduce()` → Rust `.fold()`
- `map()` → `.map()`
- `filter()` → `.filter()`
- `zip()` → `.zip()`
- `enumerate()` → `.enumerate()`

## Acceptance Criteria

- [ ] Itertools combination tests migrated
- [ ] Itertools filtering tests migrated
- [ ] Itertools grouping tests migrated
- [ ] Itertools infinite iterator tests migrated
- [ ] Functools tests migrated
- [ ] Map/filter/reduce tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Low-Medium
