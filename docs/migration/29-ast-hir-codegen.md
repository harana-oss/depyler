# Migration Task: AST, HIR, and Code Generation Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_ast_bridge.rs | ~200 | ~10 | `parse_to_hir` |
| test_hir.rs | ~250 | ~12 | `parse_to_hir` |
| test_codegen.rs | ~180 | ~9 | `transpile_python` |
| test_codegen_edge_cases.rs | ~150 | ~8 | `transpile_and_check` |
| test_transpile_edge_cases.rs | ~120 | ~6 | `transpile_and_check` |

## Target File

`tests/toml/29-ast-hir-codegen.toml`

## Tests to Migrate

### AST Bridge Tests

```toml
[[test]]
name = "ast_function_def"
description = "AST function definition parsing"
category = "ast"

[test.python]
code =  '''
def simple_func():
    pass
'''

[test.assertions]
hir_contains = ["FunctionDef", "simple_func"]
```

```toml
[[test]]
name = "ast_class_def"
description = "AST class definition parsing"
category = "ast"

[test.python]
code =  '''
class SimpleClass:
    pass
'''

[test.assertions]
hir_contains = ["ClassDef", "SimpleClass"]
```

```toml
[[test]]
name = "ast_import"
description = "AST import statement parsing"
category = "ast"

[test.python]
code =  '''
import os
from typing import List
'''

[test.assertions]
hir_contains = ["Import", "os"]
```

```toml
[[test]]
name = "ast_assignment"
description = "AST assignment statement parsing"
category = "ast"

[test.python]
code =  '''
x = 42
y: int = 10
'''

[test.assertions]
hir_contains = ["Assign", "x", "y"]
```

### HIR (High-level IR) Tests

```toml
[[test]]
name = "hir_type_inference_int"
description = "HIR type inference for integers"
category = "hir"

[test.python]
code =  '''
def get_number() -> int:
    return 42
'''

[test.assertions]
hir_type = "i64"
```

```toml
[[test]]
name = "hir_type_inference_string"
description = "HIR type inference for strings"
category = "hir"

[test.python]
code =  '''
def get_message() -> str:
    return "hello"
'''

[test.assertions]
hir_type = "String"
```

```toml
[[test]]
name = "hir_type_inference_list"
description = "HIR type inference for lists"
category = "hir"

[test.python]
code =  '''
def get_numbers() -> list[int]:
    return [1, 2, 3]
'''

[test.assertions]
hir_type = "Vec<i64>"
```

```toml
[[test]]
name = "hir_control_flow"
description = "HIR control flow representation"
category = "hir"

[test.python]
code =  '''
def conditional(x: int) -> int:
    if x > 0:
        return 1
    else:
        return -1
'''

[test.assertions]
hir_contains = ["If", "condition", "then", "else"]
```

```toml
[[test]]
name = "hir_loop_for"
description = "HIR for loop representation"
category = "hir"

[test.python]
code =  '''
def sum_list(nums: list[int]) -> int:
    total = 0
    for n in nums:
        total += n
    return total
'''

[test.assertions]
hir_contains = ["ForLoop", "iter", "body"]
```

```toml
[[test]]
name = "hir_loop_while"
description = "HIR while loop representation"
category = "hir"

[test.python]
code =  '''
def countdown(n: int) -> int:
    while n > 0:
        n -= 1
    return n
'''

[test.assertions]
hir_contains = ["WhileLoop", "condition", "body"]
```

### Code Generation Tests

```toml
[[test]]
name = "codegen_function_signature"
description = "Function signature code generation"
category = "codegen"

[test.python]
code =  '''
def add(a: int, b: int) -> int:
    return a + b
'''

[test.assertions]
any_of = ["fn add(a: i64, b: i64) -> i64"]
```

```toml
[[test]]
name = "codegen_struct_definition"
description = "Struct definition code generation"
category = "codegen"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
'''

[test.assertions]
any_of = ["struct Point", "x:", "y:"]
```

```toml
[[test]]
name = "codegen_impl_block"
description = "Impl block code generation"
category = "codegen"

[test.python]
code =  '''
class Counter:
    def __init__(self):
        self.value = 0
    
    def increment(self) -> int:
        self.value += 1
        return self.value
'''

[test.assertions]
any_of = ["impl Counter", "fn increment"]
```

### Edge Cases

```toml
[[test]]
name = "edge_empty_function"
description = "Empty function body handling"
category = "edge"

[test.python]
code =  '''
def do_nothing():
    pass
'''

[test.assertions]
any_of = ["fn do_nothing", "()"]
compiles = true
```

```toml
[[test]]
name = "edge_nested_functions"
description = "Nested function handling"
category = "edge"

[test.python]
code =  '''
def outer(x: int) -> int:
    def inner(y: int) -> int:
        return y * 2
    return inner(x)
'''

[test.assertions]
any_of = ["fn outer", "closure", "|"]
compiles = true
```

```toml
[[test]]
name = "edge_docstring"
description = "Docstring preservation or comment"
category = "edge"

[test.python]
code =  '''
def documented() -> int:
    """This function returns 42."""
    return 42
'''

[test.assertions]
any_of = ["//", "///", "42"]
```

```toml
[[test]]
name = "edge_decorator"
description = "Decorator handling"
category = "edge"

[test.python]
code =  '''
@staticmethod
def helper() -> int:
    return 0
'''

[test.assertions]
any_of = ["fn helper", "#["]
```

```toml
[[test]]
name = "edge_multiple_return"
description = "Multiple return values as tuple"
category = "edge"

[test.python]
code =  '''
def divmod_impl(a: int, b: int) -> tuple[int, int]:
    return a // b, a % b
'''

[test.assertions]
any_of = ["(i64, i64)", "tuple"]
compiles = true
```

```toml
[[test]]
name = "edge_unicode_identifiers"
description = "Unicode in identifiers or strings"
category = "edge"

[test.python]
code =  '''
def greet() -> str:
    return "Hello, 世界!"
'''

[test.assertions]
any_of = ["世界", "String"]
compiles = true
```

## Notes

- AST tests verify parsing correctness using `parse_to_hir` helper
- HIR tests verify intermediate representation structure and type inference
- Codegen tests verify Rust code output format
- Edge case tests verify handling of unusual Python constructs
- These tests may require custom assertion types:
  - `hir_contains`: Check HIR structure for specific nodes
  - `hir_type`: Check inferred type in HIR
  - `compiles`: Verify generated Rust compiles successfully

## Extended Test Format

For AST/HIR tests, the TOML format may need extension:

```toml
[[test]]
name = "..."
category = "ast" | "hir" | "codegen" | "edge"

[test.assertions]
# Standard assertions
any_of = ["pattern1", "pattern2"]

# HIR-specific assertions
hir_contains = ["NodeType", "name"]
hir_type = "i64"

# Compilation assertion
compiles = true
```

## Acceptance Criteria

- [ ] AST bridge tests migrated (10 tests)
- [ ] HIR tests migrated (12 tests)
- [ ] Codegen tests migrated (9 tests)
- [ ] Edge case tests migrated (14 tests)
- [ ] Custom assertion types documented

## Estimated Effort

**Time**: 4-5 hours
**Risk**: Medium (may need test runner extensions)
