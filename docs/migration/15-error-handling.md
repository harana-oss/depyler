# Migration Task: Error Handling Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_error_handlings.rs | 317 | ~20 | `transpile_and_check` |
| test_exception_handlings.rs | ~200 | ~12 | `transpile_and_check` |
| test_exception_scope.rs | ~100 | ~5 | `transpile_and_check` |
| test_valueerror.rs | ~80 | ~4 | `transpile_and_check` |
| test_error_coverage.rs | ~150 | ~8 | `transpile_and_check` |
| test_error_path_coverage.rs | ~100 | ~5 | `transpile_and_check` |
| test_error_gen_coverage.rs | ~120 | ~6 | `transpile_and_check` |

## Target File

`tests/toml/15-error-handling.toml`

## Tests to Migrate

### Try-Except Basic

```toml
[[test]]
name = "try_except_simple"
description = "Simple try-except block"

[test.python]
code =  '''
def safe_divide(a: int, b: int) -> int:
    try:
        return a // b
    except ZeroDivisionError:
        return 0
'''

[test.assertions]
any_of = ["Result", "match", "?"]
```

```toml
[[test]]
name = "try_except_generic"
description = "Try-except with generic Exception"

[test.python]
code =  '''
def safe_call(func) -> str:
    try:
        return func()
    except Exception as e:
        return str(e)
'''

[test.assertions]
any_of = ["Result", "Err(", "catch"]
```

```toml
[[test]]
name = "try_except_multiple"
description = "Multiple except clauses"

[test.python]
code =  '''
def parse_safely(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return -1
    except TypeError:
        return -2
'''

[test.assertions]
any_of = ["match", "Err("]
```

### Try-Except-Finally

```toml
[[test]]
name = "try_finally"
description = "Try with finally block"

[test.python]
code =  '''
def with_cleanup():
    try:
        do_work()
    finally:
        cleanup()
'''

[test.assertions]
any_of = ["defer", "Drop", "finally"]
```

```toml
[[test]]
name = "try_except_finally"
description = "Try-except-finally complete"

[test.python]
code =  '''
def safe_file_read(filename: str) -> str:
    f = None
    try:
        f = open(filename)
        return f.read()
    except IOError:
        return ""
    finally:
        if f:
            f.close()
'''

[test.assertions]
any_of = ["Result", "drop", "?"]
```

### Try-Except-Else

```toml
[[test]]
name = "try_except_else"
description = "Try-except with else block"

[test.python]
code =  '''
def safe_parse(s: str) -> str:
    try:
        value = int(s)
    except ValueError:
        return "invalid"
    else:
        return f"value: {value}"
'''

[test.assertions]
any_of = ["Ok(", "match"]
```

### Raise Statements

```toml
[[test]]
name = "raise_valueerror"
description = "Raise ValueError"

[test.python]
code =  '''
def validate_positive(x: int) -> int:
    if x < 0:
        raise ValueError("must be positive")
    return x
'''

[test.assertions]
any_of = ["Err(", "panic!", "return Err"]
```

```toml
[[test]]
name = "raise_typeerror"
description = "Raise TypeError"

[test.python]
code =  '''
def require_int(x) -> int:
    if not isinstance(x, int):
        raise TypeError("expected int")
    return x
'''

[test.assertions]
any_of = ["Err(", "panic!"]
```

```toml
[[test]]
name = "raise_custom"
description = "Raise with custom message"

[test.python]
code =  '''
def check_bounds(x: int, low: int, high: int) -> int:
    if x < low or x > high:
        raise ValueError(f"out of bounds: {x}")
    return x
'''

[test.assertions]
any_of = ["Err(", "format!"]
```

### Exception Chaining

```toml
[[test]]
name = "raise_from"
description = "Raise from another exception"

[test.python]
code =  '''
def wrap_error():
    try:
        risky_call()
    except RuntimeError as e:
        raise ValueError("wrapped") from e
'''

[test.assertions]
any_of = ["cause", "source", "Err("]
```

### Assert Statements

```toml
[[test]]
name = "assert_simple"
description = "Simple assert statement"

[test.python]
code =  '''
def checked_sqrt(x: float) -> float:
    assert x >= 0, "cannot sqrt negative"
    return x ** 0.5
'''

[test.assertions]
any_of = ["assert!", "debug_assert!", "if x < 0"]
```

```toml
[[test]]
name = "assert_no_message"
description = "Assert without message"

[test.python]
code =  '''
def require_nonempty(items: list) -> list:
    assert len(items) > 0
    return items
'''

[test.assertions]
any_of = ["assert!", "debug_assert!"]
```

### Error Propagation

```toml
[[test]]
name = "error_propagate_question"
description = "Error propagation with ?"

[test.python]
code =  '''
def read_and_parse(filename: str) -> int:
    with open(filename) as f:
        return int(f.read())
'''

[test.assertions]
contains = ["?"]
any_of = ["Result<", "-> Result"]
```

### Nested Try-Except

```toml
[[test]]
name = "try_nested"
description = "Nested try-except blocks"

[test.python]
code =  '''
def nested_error_handling(data: str) -> int:
    try:
        try:
            return int(data)
        except ValueError:
            return int(float(data))
    except:
        return 0
'''

[test.assertions]
any_of = ["match", "Ok(", "Err("]
```

### Context Manager Error Handling

```toml
[[test]]
name = "with_error_handling"
description = "Error handling in with statement"

[test.python]
code =  '''
def safe_file_ops(filename: str) -> str:
    try:
        with open(filename) as f:
            return f.read()
    except IOError:
        return ""
'''

[test.assertions]
any_of = ["Result", "?", "match"]
```

### Return in Exception Handler

```toml
[[test]]
name = "except_return"
description = "Return from except block"

[test.python]
code =  '''
def safe_get(items: list, index: int) -> int:
    try:
        return items[index]
    except IndexError:
        return -1
'''

[test.assertions]
any_of = ["get(", "unwrap_or(-1)", "Option"]
```

### Exception Type Matching

```toml
[[test]]
name = "isinstance_exception"
description = "Check exception type"

[test.python]
code =  '''
def handle_by_type(e: Exception) -> str:
    if isinstance(e, ValueError):
        return "value error"
    elif isinstance(e, TypeError):
        return "type error"
    return "other"
'''

[test.assertions]
any_of = ["match", "downcast"]
```

## Notes

- Python `try/except` → Rust `Result<T, E>` or `match`
- `raise` → `return Err(...)` or `panic!`
- `finally` → RAII/Drop patterns or explicit cleanup
- `assert` → `assert!` or `debug_assert!`
- Exception types map to error enums or custom types
- `?` operator propagates errors idiomatically

## Acceptance Criteria

- [ ] Try-except tests migrated
- [ ] Try-finally tests migrated
- [ ] Raise tests migrated
- [ ] Assert tests migrated
- [ ] Error propagation tests migrated
- [ ] Nested error handling tests migrated

## Estimated Effort

**Time**: 4-5 hours
**Risk**: High (complex error model translation)
