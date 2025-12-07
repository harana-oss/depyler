# Migration Task: Control Flow Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_if_elif_variable_shadowing.rs | ~120 | ~6 | `transpile_and_check` |
| test_ternary_expression.rs | ~100 | ~5 | `transpile_and_check` |
| test_try_block_analysis.rs | ~150 | ~8 | `transpile_and_check` |
| test_try_except_control_flow.rs | ~200 | ~10 | `transpile_and_check` |
| test_range_step.rs | ~80 | ~4 | `transpile_and_check` |
| test_range_v1.rs | ~100 | ~5 | `transpile_and_check` |
| test_unused_loop_vars.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/04-control-flow.toml`

## Tests to Migrate

### If/Elif/Else Statements

```toml
[[test]]
name = "if_simple"
description = "Simple if statement"

[test.python]
code =  '''
def check_positive(x: int) -> bool:
    if x > 0:
        return True
    return False
'''

[test.assertions]
contains = ["if x > 0", "true", "false"]
```

```toml
[[test]]
name = "if_else"
description = "If-else statement"

[test.python]
code =  '''
def sign(x: int) -> str:
    if x >= 0:
        return "non-negative"
    else:
        return "negative"
'''

[test.assertions]
contains = ["if", "else", "non-negative", "negative"]
```

```toml
[[test]]
name = "if_elif_else"
description = "If-elif-else chain"

[test.python]
code =  '''
def classify(x: int) -> str:
    if x > 0:
        return "positive"
    elif x < 0:
        return "negative"
    else:
        return "zero"
'''

[test.assertions]
contains = ["if x > 0", "else if x < 0", "else"]
not_contains = ["elif"]
```

### Ternary Expressions

```toml
[[test]]
name = "ternary_simple"
description = "Simple ternary expression"

[test.python]
code =  '''
def abs_value(x: int) -> int:
    return x if x >= 0 else -x
'''

[test.assertions]
contains = ["if", "else", "-x"]
```

```toml
[[test]]
name = "ternary_nested"
description = "Nested ternary expression"

[test.python]
code =  '''
def classify(x: int) -> str:
    return "positive" if x > 0 else "zero" if x == 0 else "negative"
'''

[test.assertions]
contains = ["if", "else"]
```

### For Loops

```toml
[[test]]
name = "for_range"
description = "For loop with range"

[test.python]
code =  '''
def sum_to_n(n: int) -> int:
    total = 0
    for i in range(n):
        total += i
    return total
'''

[test.assertions]
contains = ["for i in 0..n", "total"]
any_of = ["0..n", "0..=n-1", "(0..n)"]
```

```toml
[[test]]
name = "for_range_with_start"
description = "For loop with range(start, stop)"

[test.python]
code =  '''
def sum_range(start: int, stop: int) -> int:
    total = 0
    for i in range(start, stop):
        total += i
    return total
'''

[test.assertions]
contains = ["for i in", "start", "stop"]
any_of = ["start..stop", "(start..stop)"]
```

```toml
[[test]]
name = "for_range_with_step"
description = "For loop with range(start, stop, step)"

[test.python]
code =  '''
def sum_evens(n: int) -> int:
    total = 0
    for i in range(0, n, 2):
        total += i
    return total
'''

[test.assertions]
contains = ["for i in", "step_by"]
```

```toml
[[test]]
name = "for_iterate_list"
description = "For loop iterating over list"

[test.python]
code =  '''
def sum_list(items: list[int]) -> int:
    total = 0
    for item in items:
        total += item
    return total
'''

[test.assertions]
contains = ["for item in", "iter()"]
```

```toml
[[test]]
name = "for_enumerate"
description = "For loop with enumerate"

[test.python]
code =  '''
def indexed_items(items: list[str]) -> list[str]:
    result = []
    for i, item in enumerate(items):
        result.append(f"{i}: {item}")
    return result
'''

[test.assertions]
contains = ["enumerate()"]
```

### While Loops

```toml
[[test]]
name = "while_simple"
description = "Simple while loop"

[test.python]
code =  '''
def count_down(n: int) -> int:
    count = n
    while count > 0:
        count -= 1
    return count
'''

[test.assertions]
contains = ["while count > 0"]
```

```toml
[[test]]
name = "while_with_break"
description = "While loop with break"

[test.python]
code =  '''
def find_first(items: list[int], target: int) -> int:
    i = 0
    while i < len(items):
        if items[i] == target:
            break
        i += 1
    return i
'''

[test.assertions]
contains = ["while", "break"]
```

```toml
[[test]]
name = "while_with_continue"
description = "While loop with continue"

[test.python]
code =  '''
def sum_positive(items: list[int]) -> int:
    total = 0
    i = 0
    while i < len(items):
        if items[i] < 0:
            i += 1
            continue
        total += items[i]
        i += 1
    return total
'''

[test.assertions]
contains = ["while", "continue"]
```

### Try/Except

```toml
[[test]]
name = "try_except_basic"
description = "Basic try-except block"

[test.python]
code =  '''
def safe_divide(a: int, b: int) -> int:
    try:
        return a // b
    except ZeroDivisionError:
        return 0
'''

[test.assertions]
contains = ["match", "Ok", "Err"]
any_of = ["Result", "?", "unwrap_or"]
```

```toml
[[test]]
name = "try_except_finally"
description = "Try-except with finally"

[test.python]
code =  '''
def safe_read(filename: str) -> str:
    try:
        with open(filename) as f:
            return f.read()
    except IOError:
        return ""
    finally:
        print("Done")
'''

[test.assertions]
contains = ["Result", "println"]
```

### Match/Case (Python 3.10+)

```toml
[[test]]
name = "match_simple"
description = "Simple match statement"

[test.python]
code =  '''
def describe(x: int) -> str:
    match x:
        case 0:
            return "zero"
        case 1:
            return "one"
        case _:
            return "other"
'''

[test.assertions]
contains = ["match x", "0 =>", "1 =>", "_ =>"]
```

### Variable Shadowing in Control Flow

```toml
[[test]]
name = "if_variable_shadowing"
description = "Variable defined in if branch"

[test.python]
code =  '''
def process(condition: bool) -> int:
    if condition:
        result = 10
    else:
        result = 20
    return result
'''

[test.assertions]
contains = ["let result", "if condition"]
```

### Unused Loop Variables

```toml
[[test]]
name = "unused_loop_var"
description = "Loop variable not used in body"

[test.python]
code =  '''
def repeat(n: int) -> int:
    count = 0
    for _ in range(n):
        count += 1
    return count
'''

[test.assertions]
contains = ["for _"]
not_contains = ["for i in"]
```

## Notes

- Python `elif` → Rust `else if`
- Python ternary `a if cond else b` → Rust `if cond { a } else { b }`
- `range(n)` → `0..n`
- `range(start, stop)` → `start..stop`
- `range(start, stop, step)` → `(start..stop).step_by(step)`
- `try/except` → `match` on `Result` or `?` operator
- Python 3.10 `match` → Rust `match` (mostly direct)

## Acceptance Criteria

- [ ] If/elif/else tests migrated
- [ ] Ternary expression tests migrated
- [ ] For loop tests migrated (all range variants)
- [ ] While loop tests migrated
- [ ] Try/except tests migrated
- [ ] Match statement tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Medium (try/except has complex semantics)
