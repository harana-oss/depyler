# Migration Task: Optional Type Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_optional_assign_type_check.rs | ~80 | ~4 | `transpile_and_check` |
| test_optional_augassign.rs | ~80 | ~4 | `transpile_and_check` |
| test_optional_conditional.rs | ~100 | ~5 | `transpile_and_check` |
| test_optional_default_values.rs | ~120 | ~6 | `transpile_and_check` |
| test_optional_index_unwrap.rs | ~80 | ~4 | `transpile_and_check` |
| test_optional_method_unwrap.rs | ~100 | ~5 | `transpile_and_check` |
| test_optional_unary_unwrap.rs | ~60 | ~3 | `transpile_and_check` |
| test_optional_unwrap_args.rs | ~80 | ~4 | `transpile_and_check` |
| test_option_type_mismatch.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/19-optional-types.toml`

## Tests to Migrate

### Basic Optional

```toml
[[test]]
name = "optional_param"
description = "Optional parameter type"

[test.python]
code =  '''
from typing import Optional

def greet(name: Optional[str]) -> str:
    if name is None:
        return "Hello, stranger"
    return f"Hello, {name}"
'''

[test.assertions]
contains = ["Option<String>"]
```

```toml
[[test]]
name = "optional_return"
description = "Optional return type"

[test.python]
code =  '''
from typing import Optional

def find_item(items: list[int], target: int) -> Optional[int]:
    for item in items:
        if item == target:
            return item
    return None
'''

[test.assertions]
contains = ["Option<i32>", "Some(", "None"]
```

### None Checks

```toml
[[test]]
name = "optional_is_none"
description = "Check if value is None"

[test.python]
code =  '''
from typing import Optional

def is_empty(value: Optional[str]) -> bool:
    return value is None
'''

[test.assertions]
contains = [".is_none()"]
```

```toml
[[test]]
name = "optional_is_not_none"
description = "Check if value is not None"

[test.python]
code =  '''
from typing import Optional

def has_value(value: Optional[str]) -> bool:
    return value is not None
'''

[test.assertions]
contains = [".is_some()"]
```

### Optional Unwrapping

```toml
[[test]]
name = "optional_unwrap_or"
description = "Unwrap with default value"

[test.python]
code =  '''
from typing import Optional

def get_or_default(value: Optional[int]) -> int:
    return value if value is not None else 0
'''

[test.assertions]
contains = ["unwrap_or(0)"]
```

```toml
[[test]]
name = "optional_unwrap_or_default"
description = "Unwrap with type default"

[test.python]
code =  '''
from typing import Optional

def safe_string(value: Optional[str]) -> str:
    return value or ""
'''

[test.assertions]
any_of = ["unwrap_or_default()", "unwrap_or(\"\".to_string())"]
```

```toml
[[test]]
name = "optional_if_let"
description = "if let pattern for Optional"

[test.python]
code =  '''
from typing import Optional

def process(value: Optional[int]) -> int:
    if value is not None:
        return value * 2
    return 0
'''

[test.assertions]
any_of = ["if let Some(", "match", "is_some()"]
```

### Optional with Operations

```toml
[[test]]
name = "optional_arithmetic"
description = "Arithmetic on optional value"

[test.python]
code =  '''
from typing import Optional

def double(value: Optional[int]) -> Optional[int]:
    if value is None:
        return None
    return value * 2
'''

[test.assertions]
any_of = [".map(|v| v * 2)", "Some(value * 2)"]
```

```toml
[[test]]
name = "optional_method_call"
description = "Method call on optional value"

[test.python]
code =  '''
from typing import Optional

def upper(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    return value.upper()
'''

[test.assertions]
any_of = [".map(|v| v.to_uppercase())", "Some("]
```

### Optional in Collections

```toml
[[test]]
name = "optional_dict_get"
description = "Dict get returns Optional"

[test.python]
code =  '''
def safe_lookup(d: dict[str, int], key: str) -> int:
    return d.get(key, 0)
'''

[test.assertions]
any_of = ["unwrap_or(0)", "get("].concat(["unwrap_or"])
```

```toml
[[test]]
name = "optional_list_first"
description = "Get first element (may not exist)"

[test.python]
code =  '''
from typing import Optional

def first(items: list[int]) -> Optional[int]:
    if len(items) > 0:
        return items[0]
    return None
'''

[test.assertions]
any_of = [".first()", "get(0)", "Option<"]
```

### Optional Chaining

```toml
[[test]]
name = "optional_chain"
description = "Chained optional operations"

[test.python]
code =  '''
from typing import Optional

def process_chain(value: Optional[str]) -> Optional[int]:
    if value is None:
        return None
    stripped = value.strip()
    if not stripped:
        return None
    return len(stripped)
'''

[test.assertions]
any_of = [".and_then(", ".map(", "?"]
```

### Default Parameter Values

```toml
[[test]]
name = "optional_default_param"
description = "Optional parameter with default None"

[test.python]
code =  '''
def greet(name: str = None) -> str:
    if name is None:
        return "Hello"
    return f"Hello, {name}"
'''

[test.assertions]
contains = ["Option<"]
```

### Augmented Assignment with Optional

```toml
[[test]]
name = "optional_augassign"
description = "Augmented assignment on optional"

[test.python]
code =  '''
from typing import Optional

def increment(value: Optional[int]) -> Optional[int]:
    if value is not None:
        value += 1
    return value
'''

[test.assertions]
any_of = [".map(|v| v + 1)", "Some(value + 1)"]
```

### Index Access with Optional

```toml
[[test]]
name = "optional_index_safe"
description = "Safe index access returning Optional"

[test.python]
code =  '''
from typing import Optional

def safe_get(items: list[int], index: int) -> Optional[int]:
    if 0 <= index < len(items):
        return items[index]
    return None
'''

[test.assertions]
any_of = [".get(index)", "Option<i32>"]
```

## Notes

- Python `None` → Rust `None` (for `Option<T>`)
- `Optional[T]` → `Option<T>`
- `if x is None` → `if x.is_none()`
- `x if x is not None else default` → `x.unwrap_or(default)`
- Optional map: `None if x is None else f(x)` → `x.map(f)`
- Optional chain: multiple checks → `.and_then()`
- Dict `.get()` returns `Option<&V>`

## Acceptance Criteria

- [ ] Basic optional tests migrated
- [ ] None check tests migrated
- [ ] Optional unwrap tests migrated
- [ ] Optional operation tests migrated
- [ ] Optional collection tests migrated
- [ ] Optional chaining tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Medium
