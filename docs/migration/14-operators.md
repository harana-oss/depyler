# Migration Task: Operator Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_operators.rs | 552 | ~35 | HIR direct construction |
| test_power_operator.rs | ~100 | ~5 | `transpile_and_check` |
| test_floor_div_zero_handler.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/14-operators.toml`

## Tests to Migrate

### Arithmetic Operators

```toml
[[test]]
name = "operator_add"
description = "Addition operator"

[test.python]
code =  '''
def add(a: int, b: int) -> int:
    return a + b
'''

[test.assertions]
contains = ["a + b"]
```

```toml
[[test]]
name = "operator_subtract"
description = "Subtraction operator"

[test.python]
code =  '''
def subtract(a: int, b: int) -> int:
    return a - b
'''

[test.assertions]
contains = ["a - b"]
```

```toml
[[test]]
name = "operator_multiply"
description = "Multiplication operator"

[test.python]
code =  '''
def multiply(a: int, b: int) -> int:
    return a * b
'''

[test.assertions]
contains = ["a * b"]
```

```toml
[[test]]
name = "operator_divide"
description = "Division operator"

[test.python]
code =  '''
def divide(a: float, b: float) -> float:
    return a / b
'''

[test.assertions]
contains = ["a / b"]
```

```toml
[[test]]
name = "operator_floor_div"
description = "Floor division operator"

[test.python]
code =  '''
def floor_div(a: int, b: int) -> int:
    return a // b
'''

[test.assertions]
any_of = ["a / b", "div_euclid"]
not_contains = ["//"]
```

```toml
[[test]]
name = "operator_modulo"
description = "Modulo operator"

[test.python]
code =  '''
def modulo(a: int, b: int) -> int:
    return a % b
'''

[test.assertions]
contains = ["a % b"]
```

### Power Operator

```toml
[[test]]
name = "operator_power_literal"
description = "Power operator with literals"

[test.python]
code =  '''
def power_literal() -> int:
    return 2 ** 8
'''

[test.assertions]
contains = ["pow(8"]
any_of = ["2_i32.pow(", "2i32.pow("]
```

```toml
[[test]]
name = "operator_power_variable"
description = "Power operator with variables"

[test.python]
code =  '''
def power_var(base: int, exp: int) -> int:
    return base ** exp
'''

[test.assertions]
contains = ["pow(", "checked_pow"]
any_of = ["base.pow(", "base.checked_pow("]
```

```toml
[[test]]
name = "operator_pow_builtin"
description = "pow() builtin function"

[test.python]
code =  '''
def pow_call(base: int, exp: int) -> int:
    return pow(base, exp)
'''

[test.assertions]
contains = ["pow("]
```

### Comparison Operators

```toml
[[test]]
name = "operator_eq"
description = "Equality operator"

[test.python]
code =  '''
def equals(a: int, b: int) -> bool:
    return a == b
'''

[test.assertions]
contains = ["a == b"]
```

```toml
[[test]]
name = "operator_ne"
description = "Not equal operator"

[test.python]
code =  '''
def not_equals(a: int, b: int) -> bool:
    return a != b
'''

[test.assertions]
contains = ["a != b"]
```

```toml
[[test]]
name = "operator_lt"
description = "Less than operator"

[test.python]
code =  '''
def less_than(a: int, b: int) -> bool:
    return a < b
'''

[test.assertions]
contains = ["a < b"]
```

```toml
[[test]]
name = "operator_le"
description = "Less than or equal operator"

[test.python]
code =  '''
def less_equal(a: int, b: int) -> bool:
    return a <= b
'''

[test.assertions]
contains = ["a <= b"]
```

```toml
[[test]]
name = "operator_gt"
description = "Greater than operator"

[test.python]
code =  '''
def greater_than(a: int, b: int) -> bool:
    return a > b
'''

[test.assertions]
contains = ["a > b"]
```

```toml
[[test]]
name = "operator_ge"
description = "Greater than or equal operator"

[test.python]
code =  '''
def greater_equal(a: int, b: int) -> bool:
    return a >= b
'''

[test.assertions]
contains = ["a >= b"]
```

### Logical Operators

```toml
[[test]]
name = "operator_and"
description = "Logical and operator"

[test.python]
code =  '''
def both_true(a: bool, b: bool) -> bool:
    return a and b
'''

[test.assertions]
contains = ["a && b"]
not_contains = [" and "]
```

```toml
[[test]]
name = "operator_or"
description = "Logical or operator"

[test.python]
code =  '''
def either_true(a: bool, b: bool) -> bool:
    return a or b
'''

[test.assertions]
contains = ["a || b"]
not_contains = [" or "]
```

```toml
[[test]]
name = "operator_not"
description = "Logical not operator"

[test.python]
code =  '''
def negate(a: bool) -> bool:
    return not a
'''

[test.assertions]
contains = ["!a"]
not_contains = ["not "]
```

### Bitwise Operators

```toml
[[test]]
name = "operator_bitand"
description = "Bitwise and operator"

[test.python]
code =  '''
def bitand(a: int, b: int) -> int:
    return a & b
'''

[test.assertions]
contains = ["a & b"]
```

```toml
[[test]]
name = "operator_bitor"
description = "Bitwise or operator"

[test.python]
code =  '''
def bitor(a: int, b: int) -> int:
    return a | b
'''

[test.assertions]
contains = ["a | b"]
```

```toml
[[test]]
name = "operator_bitxor"
description = "Bitwise xor operator"

[test.python]
code =  '''
def bitxor(a: int, b: int) -> int:
    return a ^ b
'''

[test.assertions]
contains = ["a ^ b"]
```

```toml
[[test]]
name = "operator_bitnot"
description = "Bitwise not operator"

[test.python]
code =  '''
def bitnot(a: int) -> int:
    return ~a
'''

[test.assertions]
contains = ["!a"]
```

```toml
[[test]]
name = "operator_lshift"
description = "Left shift operator"

[test.python]
code =  '''
def lshift(a: int, b: int) -> int:
    return a << b
'''

[test.assertions]
contains = ["a << b"]
```

```toml
[[test]]
name = "operator_rshift"
description = "Right shift operator"

[test.python]
code =  '''
def rshift(a: int, b: int) -> int:
    return a >> b
'''

[test.assertions]
contains = ["a >> b"]
```

### Membership Operators

```toml
[[test]]
name = "operator_in_list"
description = "in operator for list"

[test.python]
code =  '''
def is_member(items: list[int], x: int) -> bool:
    return x in items
'''

[test.assertions]
contains = ["contains("]
```

```toml
[[test]]
name = "operator_in_dict"
description = "in operator for dict"

[test.python]
code =  '''
def has_key(d: dict, key: str) -> bool:
    return key in d
'''

[test.assertions]
contains = ["contains_key("]
```

```toml
[[test]]
name = "operator_not_in"
description = "not in operator"

[test.python]
code =  '''
def not_member(items: list[int], x: int) -> bool:
    return x not in items
'''

[test.assertions]
contains = ["!"]
any_of = ["!items.contains(", "!contains"]
```

### Augmented Assignment (see also 06-assignment.md)

```toml
[[test]]
name = "operator_augassign_overview"
description = "All augmented assignment operators"

[test.python]
code =  '''
def aug_assign_demo(x: int) -> int:
    x += 1
    x -= 1
    x *= 2
    x //= 2
    x %= 3
    x **= 2
    return x
'''

[test.assertions]
any_of = ["+=", "= x +"]
```

## Notes

- Most operators map directly (arithmetic, comparison, bitwise)
- `and`/`or`/`not` → `&&`/`||`/`!`
- `**` → `.pow()` or `.checked_pow()`
- `//` → integer division (may use `div_euclid`)
- `in` → `.contains()` or `.contains_key()`
- Augmented assignment may expand to full expression

## Acceptance Criteria

- [ ] Arithmetic operator tests migrated
- [ ] Power operator tests migrated
- [ ] Comparison operator tests migrated
- [ ] Logical operator tests migrated
- [ ] Bitwise operator tests migrated
- [ ] Membership operator tests migrated

## Estimated Effort

**Time**: 3 hours
**Risk**: Low
