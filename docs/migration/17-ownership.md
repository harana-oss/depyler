# Migration Task: Ownership and Borrowing Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_ownership_patterns.rs | ~200 | ~12 | `transpile_and_check` |
| test_function_borrowing.rs | ~150 | ~8 | `transpile_and_check` |
| test_method_ownership.rs | ~120 | ~6 | `transpile_and_check` |
| test_lifetimes.rs | ~180 | ~10 | `transpile_and_check` |
| test_lifetime_analysis_coverage.rs | ~200 | ~12 | `transpile_and_check` |
| test_lifetime_analysis_integration.rs | ~150 | ~8 | `transpile_and_check` |
| test_struct_field_borrow.rs | ~100 | ~5 | `transpile_and_check` |

## Target File

`tests/toml/17-ownership.toml`

## Tests to Migrate

### Basic Ownership

```toml
[[test]]
name = "ownership_move"
description = "Value moves when assigned"

[test.python]
code =  '''
def transfer_ownership():
    original = [1, 2, 3]
    new_owner = original
    return new_owner
'''

[test.assertions]
any_of = ["clone()", "let new_owner = original"]
```

```toml
[[test]]
name = "ownership_clone"
description = "Explicit clone for copy"

[test.python]
code =  '''
def copy_list(items: list[int]) -> list[int]:
    copy = list(items)  # Python creates a shallow copy
    return copy
'''

[test.assertions]
contains = ["clone()"]
```

### Borrowing Patterns

```toml
[[test]]
name = "borrow_immutable_param"
description = "Immutable borrow for read-only parameter"

[test.python]
code =  '''
def read_only(items: list[int]) -> int:
    return sum(items)
'''

[test.assertions]
contains = ["&"]
not_contains = ["&mut"]
```

```toml
[[test]]
name = "borrow_mutable_param"
description = "Mutable borrow for modifying parameter"

[test.python]
code =  '''
def add_item(items: list[int], item: int) -> None:
    items.append(item)
'''

[test.assertions]
contains = ["&mut"]
```

```toml
[[test]]
name = "borrow_multiple_immutable"
description = "Multiple immutable borrows allowed"

[test.python]
code =  '''
def compare_lists(a: list[int], b: list[int]) -> bool:
    return len(a) == len(b)
'''

[test.assertions]
contains = ["&"]
count = { "&" = { min = 2 } }
```

### Return Value Ownership

```toml
[[test]]
name = "return_owned"
description = "Return owned value"

[test.python]
code =  '''
def create_list() -> list[int]:
    return [1, 2, 3]
'''

[test.assertions]
contains = ["Vec<i32>"]
not_contains = ["&Vec"]
```

```toml
[[test]]
name = "return_reference"
description = "Return reference (slice)"

[test.python]
code =  '''
def get_first_half(items: list[int]) -> list[int]:
    mid = len(items) // 2
    return items[:mid]
'''

[test.assertions]
any_of = ["&[", "Vec<", "clone()"]
```

### Struct Field Ownership

```toml
[[test]]
name = "struct_owned_field"
description = "Struct with owned field"

[test.python]
code =  '''
class Container:
    def __init__(self, items: list[int]):
        self.items = items
'''

[test.assertions]
contains = ["Vec<i32>"]
not_contains = ["&Vec"]
```

```toml
[[test]]
name = "struct_field_access"
description = "Accessing struct field"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def get_x(p: Point) -> int:
    return p.x
'''

[test.assertions]
contains = ["p.x"]
any_of = ["&Point", "Point"]
```

### Method Borrowing

```toml
[[test]]
name = "method_self_immutable"
description = "Method with immutable self"

[test.python]
code =  '''
class Counter:
    def __init__(self, value: int):
        self.value = value
    
    def get_value(self) -> int:
        return self.value
'''

[test.assertions]
contains = ["&self"]
not_contains = ["&mut self"]
```

```toml
[[test]]
name = "method_self_mutable"
description = "Method with mutable self"

[test.python]
code =  '''
class Counter:
    def __init__(self):
        self.count = 0
    
    def increment(self):
        self.count += 1
'''

[test.assertions]
contains = ["&mut self"]
```

### Lifetime Annotations

```toml
[[test]]
name = "lifetime_return_reference"
description = "Lifetime for returned reference"

[test.python]
code =  '''
def longest(a: str, b: str) -> str:
    if len(a) > len(b):
        return a
    return b
'''

[test.assertions]
any_of = ["<'a>", "String", "clone()"]
```

```toml
[[test]]
name = "lifetime_struct_reference"
description = "Lifetime for struct with reference"

[test.python]
code =  '''
class Parser:
    def __init__(self, input: str):
        self.input = input
        self.pos = 0
'''

[test.assertions]
any_of = ["<'a>", "String"]
```

### Copy Types

```toml
[[test]]
name = "copy_primitives"
description = "Primitive types are Copy"

[test.python]
code =  '''
def swap_values(a: int, b: int) -> tuple[int, int]:
    return (b, a)
'''

[test.assertions]
contains = ["i32"]
not_contains = ["clone()"]
```

```toml
[[test]]
name = "copy_after_use"
description = "Copy type can be used after move"

[test.python]
code =  '''
def double_use(x: int) -> int:
    a = x
    b = x  # Still valid for Copy types
    return a + b
'''

[test.assertions]
contains = ["a + b"]
not_contains = ["clone()"]
```

### Ownership Transfer in Functions

```toml
[[test]]
name = "ownership_transfer_in"
description = "Transfer ownership into function"

[test.python]
code =  '''
def consume(items: list[int]) -> int:
    # Takes ownership
    items.append(999)
    return sum(items)
'''

[test.assertions]
any_of = ["mut items: Vec", "&mut Vec"]
```

```toml
[[test]]
name = "ownership_transfer_out"
description = "Transfer ownership out of function"

[test.python]
code =  '''
def produce() -> list[int]:
    items = [1, 2, 3]
    return items  # Ownership transferred out
'''

[test.assertions]
contains = ["Vec<i32>"]
```

### Collection Iteration Ownership

```toml
[[test]]
name = "iterate_borrow"
description = "Iterate without consuming"

[test.python]
code =  '''
def sum_items(items: list[int]) -> int:
    total = 0
    for item in items:
        total += item
    return total
'''

[test.assertions]
contains = [".iter()"]
```

```toml
[[test]]
name = "iterate_consume"
description = "Iterate and consume"

[test.python]
code =  '''
def drain_items(items: list[int]) -> int:
    total = 0
    while items:
        total += items.pop()
    return total
'''

[test.assertions]
any_of = [".into_iter()", ".drain(", "pop()"]
```

## Notes

- Python has no ownership; all references
- Rust requires ownership/borrowing analysis
- Read-only params → `&T`
- Mutating params → `&mut T`
- Primitives (int, float, bool) are `Copy`
- Collections (list, dict) are not `Copy`
- Return ownership → owned type
- Return borrow → lifetime annotation
- Method on `self` → `&self` or `&mut self`

## Acceptance Criteria

- [ ] Basic ownership tests migrated
- [ ] Borrowing pattern tests migrated
- [ ] Return ownership tests migrated
- [ ] Struct field tests migrated
- [ ] Method borrowing tests migrated
- [ ] Lifetime tests migrated

## Estimated Effort

**Time**: 4-5 hours
**Risk**: High (ownership is Rust-specific concept)
