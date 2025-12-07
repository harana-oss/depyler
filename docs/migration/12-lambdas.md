# Migration Task: Lambda Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_lambdas.rs | 211 | ~15 | `transpile_and_check` |
| test_lambda_integration.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/12-lambdas.toml`

## Tests to Migrate

### Basic Lambdas

```toml
[[test]]
name = "lambda_simple"
description = "Simple single-parameter lambda"

[test.python]
code =  '''
def double_numbers(numbers: list) -> list:
    return list(map(lambda x: x * 2, numbers))
'''

[test.assertions]
contains = ["fn double_numbers"]
any_of = ["|x|", "| x |"]
contains = [".map"]
```

```toml
[[test]]
name = "lambda_identity"
description = "Identity lambda"

[test.python]
code =  '''
def identity_map(items: list) -> list:
    return list(map(lambda x: x, items))
'''

[test.assertions]
any_of = ["|x| x", "| x | x"]
```

### Lambda with filter

```toml
[[test]]
name = "lambda_filter_positive"
description = "Lambda in filter for positive numbers"

[test.python]
code =  '''
def filter_positive(numbers: list) -> list:
    return list(filter(lambda x: x > 0, numbers))
'''

[test.assertions]
contains = ["fn filter_positive", ".filter"]
any_of = ["|x|", "| x |"]
```

```toml
[[test]]
name = "lambda_filter_predicate"
description = "Lambda filter with complex predicate"

[test.python]
code =  '''
def filter_range(items: list, low: int, high: int) -> list:
    return list(filter(lambda x: low <= x <= high, items))
'''

[test.assertions]
contains = [".filter"]
any_of = ["|x|", "&&"]
```

### Lambda with sorted

```toml
[[test]]
name = "lambda_sorted_key"
description = "Lambda as key function for sorted"

[test.python]
code =  '''
def sort_by_length(words: list) -> list:
    return sorted(words, key=lambda x: len(x))
'''

[test.assertions]
contains = ["fn sort_by_length", ".sort"]
any_of = ["|x|", "| x |", "len()"]
```

```toml
[[test]]
name = "lambda_sorted_reverse"
description = "Lambda for reverse sorting"

[test.python]
code =  '''
def sort_descending(numbers: list) -> list:
    return sorted(numbers, key=lambda x: -x)
'''

[test.assertions]
contains = [".sort"]
any_of = ["|x| -x", "cmp", "reverse"]
```

### Multi-parameter Lambda

```toml
[[test]]
name = "lambda_two_params"
description = "Lambda with two parameters"

[test.python]
code =  '''
def add_pairs(pairs: list) -> list:
    return list(map(lambda x, y: x + y, pairs[0], pairs[1]))
'''

[test.assertions]
contains = ["fn add_pairs"]
any_of = ["|x, y|", "| x, y |", "|(x, y)|"]
```

```toml
[[test]]
name = "lambda_zip_combine"
description = "Lambda to combine zipped lists"

[test.python]
code =  '''
def combine_lists(list1: list, list2: list) -> list:
    return list(map(lambda x, y: x + y, list1, list2))
'''

[test.assertions]
contains = ["fn combine_lists"]
any_of = ["|x, y|", "zip("]
```

### Lambda with reduce

```toml
[[test]]
name = "lambda_reduce_sum"
description = "Lambda in reduce for sum"

[test.python]
code =  '''
from functools import reduce

def sum_all(numbers: list) -> int:
    return reduce(lambda acc, x: acc + x, numbers, 0)
'''

[test.assertions]
contains = ["fold("]
any_of = ["|acc, x|", "|acc,x|"]
```

```toml
[[test]]
name = "lambda_reduce_product"
description = "Lambda in reduce for product"

[test.python]
code =  '''
from functools import reduce

def product_all(numbers: list) -> int:
    return reduce(lambda acc, x: acc * x, numbers, 1)
'''

[test.assertions]
contains = ["fold("]
any_of = ["|acc, x|", "acc * x"]
```

### Lambda with Complex Expressions

```toml
[[test]]
name = "lambda_conditional"
description = "Lambda with conditional expression"

[test.python]
code =  '''
def process(numbers: list) -> list:
    return list(map(lambda x: x * 2 if x > 0 else 0, numbers))
'''

[test.assertions]
contains = [".map"]
any_of = ["|x|", "if", "else"]
```

```toml
[[test]]
name = "lambda_method_call"
description = "Lambda calling method on parameter"

[test.python]
code =  '''
def uppercase_all(words: list) -> list:
    return list(map(lambda s: s.upper(), words))
'''

[test.assertions]
contains = [".map", "to_uppercase"]
any_of = ["|s|", "| s |"]
```

### Lambda as Argument

```toml
[[test]]
name = "lambda_as_callback"
description = "Lambda passed as callback"

[test.python]
code =  '''
def apply_func(items: list, func) -> list:
    return list(map(func, items))

def double_items(items: list) -> list:
    return apply_func(items, lambda x: x * 2)
'''

[test.assertions]
contains = ["fn apply_func", "fn double_items"]
any_of = ["|x|", "Fn("]
```

### Lambda in Comprehension

```toml
[[test]]
name = "lambda_in_list_comp"
description = "Lambda used inside list comprehension"

[test.python]
code =  '''
def apply_all(items: list) -> list:
    funcs = [lambda x: x * 2, lambda x: x + 1]
    return [f(item) for f in funcs for item in items]
'''

[test.assertions]
contains = ["Vec"]
any_of = ["|x|", "closure"]
```

### Lambda Capture

```toml
[[test]]
name = "lambda_capture_variable"
description = "Lambda capturing outer variable"

[test.python]
code =  '''
def make_adder(n: int):
    return lambda x: x + n
'''

[test.assertions]
any_of = ["|x|", "move |x|"]
contains = ["n"]
```

```toml
[[test]]
name = "lambda_capture_multiple"
description = "Lambda capturing multiple variables"

[test.python]
code =  '''
def make_range_checker(low: int, high: int):
    return lambda x: low <= x <= high
'''

[test.assertions]
any_of = ["|x|", "move"]
contains = ["low", "high"]
```

## Notes

- Python `lambda x: expr` → Rust `|x| expr`
- Multi-param `lambda x, y: expr` → `|(x, y)| expr` or `|x, y| expr`
- Lambdas capturing variables need `move` or careful borrowing
- Lambda in `map()` → `.map(|x| ...)`
- Lambda in `filter()` → `.filter(|x| ...)`
- Lambda in `reduce()` → `.fold(init, |acc, x| ...)`
- Lambda with conditional → same ternary/if expression

## Acceptance Criteria

- [ ] Basic lambda tests migrated
- [ ] Lambda with filter tests migrated
- [ ] Lambda with sorted tests migrated
- [ ] Multi-parameter lambda tests migrated
- [ ] Lambda reduce tests migrated
- [ ] Lambda capture tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Low-Medium
