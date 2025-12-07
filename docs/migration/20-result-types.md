# Migration Task: Result Type Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_result_return_wrapping.rs | ~150 | ~8 | `transpile_and_check` |
| test_result_unwrapping.rs | ~120 | ~6 | `transpile_and_check` |
| test_validator_return_type.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/20-result-types.toml`

## Tests to Migrate

### Basic Result Return

```toml
[[test]]
name = "result_ok"
description = "Function returning Ok result"

[test.python]
code =  '''
def parse_int(s: str) -> int:
    return int(s)
'''

[test.assertions]
any_of = ["Result<", "Ok(", "?"]
```

```toml
[[test]]
name = "result_err"
description = "Function returning Err result"

[test.python]
code =  '''
def validate_positive(x: int) -> int:
    if x < 0:
        raise ValueError("must be positive")
    return x
'''

[test.assertions]
any_of = ["Result<", "Err(", "return Err"]
```

### Result Wrapping

```toml
[[test]]
name = "result_wrap_ok"
description = "Wrap success value in Ok"

[test.python]
code =  '''
def success(value: int) -> int:
    return value
'''

[test.assertions]
any_of = ["Ok(value)", "Ok(", "-> i32"]
```

```toml
[[test]]
name = "result_wrap_err"
description = "Wrap error in Err"

[test.python]
code =  '''
def fail(message: str):
    raise RuntimeError(message)
'''

[test.assertions]
any_of = ["Err(", "panic!", "return Err"]
```

### Result Propagation

```toml
[[test]]
name = "result_question_mark"
description = "Propagate error with ?"

[test.python]
code =  '''
def read_and_parse(filename: str) -> int:
    with open(filename) as f:
        content = f.read()
    return int(content.strip())
'''

[test.assertions]
contains = ["?"]
any_of = ["Result<", "-> Result"]
```

```toml
[[test]]
name = "result_chain_propagation"
description = "Chain multiple fallible operations"

[test.python]
code =  '''
def process_file(filename: str) -> int:
    content = read_file(filename)  # may fail
    value = parse_int(content)      # may fail
    return validate(value)          # may fail
'''

[test.assertions]
contains = ["?"]
count = { "?" = { min = 2 } }
```

### Result Unwrapping

```toml
[[test]]
name = "result_unwrap"
description = "Unwrap Result (may panic)"

[test.python]
code =  '''
def force_parse(s: str) -> int:
    return int(s)  # raises on invalid input
'''

[test.assertions]
any_of = ["unwrap()", "expect(", "?"]
```

```toml
[[test]]
name = "result_unwrap_or"
description = "Unwrap with default on error"

[test.python]
code =  '''
def safe_parse(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return 0
'''

[test.assertions]
any_of = ["unwrap_or(0)", "unwrap_or_default()", "match"]
```

```toml
[[test]]
name = "result_unwrap_or_else"
description = "Unwrap with computed default"

[test.python]
code =  '''
def parse_with_fallback(s: str, fallback: int) -> int:
    try:
        return int(s)
    except ValueError:
        return fallback
'''

[test.assertions]
any_of = ["unwrap_or(fallback)", "unwrap_or_else(", "match"]
```

### Result Pattern Matching

```toml
[[test]]
name = "result_match"
description = "Match on Result"

[test.python]
code =  '''
def handle_result(s: str) -> str:
    try:
        value = int(s)
        return f"Got: {value}"
    except ValueError as e:
        return f"Error: {e}"
'''

[test.assertions]
any_of = ["match", "Ok(", "Err("]
```

### Result Map Operations

```toml
[[test]]
name = "result_map"
description = "Map over Ok value"

[test.python]
code =  '''
def double_parsed(s: str) -> int:
    try:
        value = int(s)
        return value * 2
    except ValueError:
        raise
'''

[test.assertions]
any_of = [".map(|v| v * 2)", "Ok(value * 2)", "?"]
```

```toml
[[test]]
name = "result_map_err"
description = "Map over Err value"

[test.python]
code =  '''
def parse_with_context(s: str, context: str) -> int:
    try:
        return int(s)
    except ValueError:
        raise ValueError(f"{context}: invalid number")
'''

[test.assertions]
any_of = [".map_err(", "Err(format!"]
```

### Result And Then

```toml
[[test]]
name = "result_and_then"
description = "Chain fallible operations"

[test.python]
code =  '''
def parse_and_validate(s: str) -> int:
    try:
        value = int(s)
        if value < 0:
            raise ValueError("must be non-negative")
        return value
    except ValueError:
        raise
'''

[test.assertions]
any_of = [".and_then(", "?", "match"]
```

### Result Conversion

```toml
[[test]]
name = "result_to_option"
description = "Convert Result to Option"

[test.python]
code =  '''
def try_parse(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return None
'''

[test.assertions]
any_of = [".ok()", "Option<", "Some(", "None"]
```

```toml
[[test]]
name = "option_to_result"
description = "Convert Option to Result"

[test.python]
code =  '''
def require_value(value) -> int:
    if value is None:
        raise ValueError("value required")
    return value
'''

[test.assertions]
any_of = [".ok_or(", "Err(", "Option"]
```

### Custom Error Types

```toml
[[test]]
name = "result_custom_error"
description = "Result with custom error type"

[test.python]
code =  '''
class ValidationError(Exception):
    pass

def validate(x: int) -> int:
    if x < 0:
        raise ValidationError("negative value")
    if x > 100:
        raise ValidationError("too large")
    return x
'''

[test.assertions]
any_of = ["enum", "struct", "Error", "Result<"]
```

## Notes

- Python exceptions → Rust `Result<T, E>`
- `raise` → `return Err(...)` or `Err(...)?`
- `try/except` → `match` on Result or `?` with handling
- `.unwrap()` is like ignoring possible exception
- `.unwrap_or(default)` is like `try: ... except: default`
- Result can be chained with `?`, `.map()`, `.and_then()`
- Custom exceptions → custom error enums/structs

## Acceptance Criteria

- [ ] Basic Result tests migrated
- [ ] Result propagation tests migrated
- [ ] Result unwrap tests migrated
- [ ] Result pattern match tests migrated
- [ ] Result map tests migrated
- [ ] Result conversion tests migrated

## Estimated Effort

**Time**: 3 hours
**Risk**: Medium
