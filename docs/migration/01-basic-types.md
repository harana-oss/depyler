# Migration Task: Basic Types Tests

## Status: Not Started

## Source Files

| File | Lines | Tests | Pattern |
|------|-------|-------|---------|
| test_basic.rs | 70 | 4 | Pure Rust tests (no transpilation) |
| test_constants.rs | ~80 | ~5 | `transpile_and_check` |
| test_dynamic_type.rs | ~60 | ~3 | `transpile_and_check` |
| test_boolean_conversion.rs | ~100 | ~6 | `transpile_and_check` |

## Target File

`tests/toml/01-basic-types.toml`

## Tests to Migrate

### From test_constants.rs

```toml
[[test]]
name = "constant_string"
description = "String constants should transpile to Rust const"

[test.python]
code =  '''
MESSAGE: str = "Hello, World!"

def get_message() -> str:
    return MESSAGE
'''

[test.assertions]
contains = ["const MESSAGE", "str", "Hello, World!"]
```

```toml
[[test]]
name = "constant_integer"
description = "Integer constants should transpile to Rust const"

[test.python]
code =  '''
MAX_VALUE: int = 100

def get_max() -> int:
    return MAX_VALUE
'''

[test.assertions]
contains = ["const MAX_VALUE", "i32", "100"]
```

### From test_boolean_conversion.rs

```toml
[[test]]
name = "boolean_true"
description = "Python True should become Rust true"

[test.python]
code =  '''
def get_true() -> bool:
    return True
'''

[test.assertions]
contains = ["true"]
not_contains = ["True"]
```

```toml
[[test]]
name = "boolean_false"
description = "Python False should become Rust false"

[test.python]
code =  '''
def get_false() -> bool:
    return False
'''

[test.assertions]
contains = ["false"]
not_contains = ["False"]
```

```toml
[[test]]
name = "boolean_not"
description = "Python not should become Rust !"

[test.python]
code =  '''
def negate(x: bool) -> bool:
    return not x
'''

[test.assertions]
contains = ["!x", "bool"]
not_contains = ["not"]
```

### From test_dynamic_type.rs

```toml
[[test]]
name = "dynamic_type_inference"
description = "Untyped parameters should infer types from usage"

[test.python]
code =  '''
def add_one(x):
    return x + 1
'''

[test.assertions]
contains = ["i32", "i64", "fn add_one"]
not_contains = ["serde_json::Value"]
```

## Notes

- `test_basic.rs` contains pure Rust helper function tests - these are NOT transpilation tests and should remain as Rust tests
- Boolean tests verify Python keyword → Rust keyword mapping
- Constant tests verify module-level constant handling

## Acceptance Criteria

- [ ] All transpilation tests from source files converted to TOML
- [ ] Tags applied consistently
- [ ] Tests pass with new TOML runner
- [ ] Old test functions marked for removal

## Estimated Effort

**Time**: 1-2 hours
**Risk**: Low
