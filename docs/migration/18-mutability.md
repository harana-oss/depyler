# Migration Task: Mutability Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_mutability.rs | ~200 | ~12 | `transpile_and_check` |
| test_clone.rs | ~100 | ~5 | `transpile_and_check` |
| test_mutationing.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/18-mutability.toml`

## Tests to Migrate

### Variable Mutability

```toml
[[test]]
name = "mut_reassign_variable"
description = "Variable requiring mut for reassignment"

[test.python]
code =  '''
def counter() -> int:
    x = 0
    x = 1
    x = 2
    return x
'''

[test.assertions]
contains = ["let mut x"]
```

```toml
[[test]]
name = "immutable_no_reassign"
description = "Variable not reassigned is immutable"

[test.python]
code =  '''
def constant() -> int:
    x = 42
    return x
'''

[test.assertions]
contains = ["let x"]
not_contains = ["let mut x"]
```

```toml
[[test]]
name = "mut_augmented_assign"
description = "Augmented assignment requires mut"

[test.python]
code =  '''
def accumulate(items: list[int]) -> int:
    total = 0
    for item in items:
        total += item
    return total
'''

[test.assertions]
contains = ["let mut total"]
```

### Collection Mutability

```toml
[[test]]
name = "mut_list_append"
description = "List append requires mut"

[test.python]
code =  '''
def build_list() -> list[int]:
    items = []
    items.append(1)
    items.append(2)
    return items
'''

[test.assertions]
contains = ["let mut items"]
```

```toml
[[test]]
name = "mut_dict_insert"
description = "Dict insert requires mut"

[test.python]
code =  '''
def build_dict() -> dict:
    d = {}
    d["a"] = 1
    d["b"] = 2
    return d
'''

[test.assertions]
contains = ["let mut d"]
```

```toml
[[test]]
name = "mut_list_sort"
description = "In-place sort requires mut"

[test.python]
code =  '''
def sort_in_place(items: list[int]) -> list[int]:
    items.sort()
    return items
'''

[test.assertions]
any_of = ["&mut", "mut items"]
```

### Parameter Mutability

```toml
[[test]]
name = "param_read_only"
description = "Read-only parameter is immutable"

[test.python]
code =  '''
def sum_list(items: list[int]) -> int:
    return sum(items)
'''

[test.assertions]
contains = ["&"]
not_contains = ["&mut"]
```

```toml
[[test]]
name = "param_mutating"
description = "Mutating parameter needs &mut"

[test.python]
code =  '''
def clear_list(items: list[int]) -> None:
    items.clear()
'''

[test.assertions]
contains = ["&mut"]
```

### Clone for Immutability

```toml
[[test]]
name = "clone_to_avoid_mut"
description = "Clone to work with copy instead of mutating"

[test.python]
code =  '''
def sorted_copy(items: list[int]) -> list[int]:
    return sorted(items)
'''

[test.assertions]
contains = ["clone()"]
any_of = ["clone().sort", "sorted"]
```

```toml
[[test]]
name = "clone_list_copy"
description = "list() creates a clone"

[test.python]
code =  '''
def copy_list(items: list[int]) -> list[int]:
    return list(items)
'''

[test.assertions]
contains = ["clone()"]
```

### Loop Variable Mutability

```toml
[[test]]
name = "loop_var_immutable"
description = "Loop variable typically immutable"

[test.python]
code =  '''
def print_all(items: list[int]) -> None:
    for item in items:
        print(item)
'''

[test.assertions]
contains = ["for item in"]
not_contains = ["mut item"]
```

```toml
[[test]]
name = "loop_var_mutating"
description = "Loop variable mutated in body"

[test.python]
code =  '''
def process(items: list[int]) -> list[int]:
    result = []
    for item in items:
        item = item * 2
        result.append(item)
    return result
'''

[test.assertions]
any_of = ["let mut item", "item * 2"]
```

### Self Mutability

```toml
[[test]]
name = "self_immutable_method"
description = "Method not mutating self"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
    
    def magnitude(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5
'''

[test.assertions]
contains = ["&self"]
not_contains = ["&mut self"]
```

```toml
[[test]]
name = "self_mutable_method"
description = "Method mutating self"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
    
    def translate(self, dx: int, dy: int):
        self.x += dx
        self.y += dy
'''

[test.assertions]
contains = ["&mut self"]
```

### Interior Mutability

```toml
[[test]]
name = "cell_pattern"
description = "Cell for interior mutability"

[test.python]
code =  '''
class Counter:
    def __init__(self):
        self._count = 0
    
    def count(self) -> int:
        self._count += 1
        return self._count
'''

[test.assertions]
any_of = ["Cell", "RefCell", "&mut self"]
```

## Notes

- Python variables are always mutable
- Rust requires explicit `mut` for mutability
- Transpiler analyzes usage to determine mutability
- Parameters mutated → `&mut`
- In-place operations (sort, append) → `&mut`
- Creating copies avoids mutation
- Method mutating `self` → `&mut self`

## Acceptance Criteria

- [ ] Variable mutability tests migrated
- [ ] Collection mutability tests migrated
- [ ] Parameter mutability tests migrated
- [ ] Clone pattern tests migrated
- [ ] Loop variable tests migrated
- [ ] Self mutability tests migrated

## Estimated Effort

**Time**: 2-3 hours
**Risk**: Medium
