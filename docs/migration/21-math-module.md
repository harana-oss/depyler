# Migration Task: Math Module Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_math_unit.rs | 558 | ~35 | `transpile_and_check` |
| test_maths.rs | 713 | ~45 | `transpile_and_check` |
| test_math_additional_unit.rs | ~200 | ~12 | `transpile_and_check` |
| test_math_more_unit.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/21-math-module.toml`

## Tests to Migrate

### Trigonometric Functions

```toml
[[test]]
name = "math_sin"
description = "math.sin() function"

[test.python]
code =  '''
import math

def calculate_sin(x: float) -> float:
    return math.sin(x)
'''

[test.assertions]
any_of = ["x.sin()", "f64::sin"]
```

```toml
[[test]]
name = "math_cos"
description = "math.cos() function"

[test.python]
code =  '''
import math

def calculate_cos(x: float) -> float:
    return math.cos(x)
'''

[test.assertions]
any_of = ["x.cos()", "f64::cos"]
```

```toml
[[test]]
name = "math_tan"
description = "math.tan() function"

[test.python]
code =  '''
import math

def calculate_tan(x: float) -> float:
    return math.tan(x)
'''

[test.assertions]
any_of = ["x.tan()", "f64::tan"]
```

### Exponential and Logarithmic

```toml
[[test]]
name = "math_exp"
description = "math.exp() function"

[test.python]
code =  '''
import math

def calculate_exp(x: float) -> float:
    return math.exp(x)
'''

[test.assertions]
any_of = ["x.exp()", "f64::exp"]
```

```toml
[[test]]
name = "math_log"
description = "math.log() natural logarithm"

[test.python]
code =  '''
import math

def calculate_log(x: float) -> float:
    return math.log(x)
'''

[test.assertions]
any_of = ["x.ln()", "f64::ln"]
```

```toml
[[test]]
name = "math_log10"
description = "math.log10() base-10 logarithm"

[test.python]
code =  '''
import math

def calculate_log10(x: float) -> float:
    return math.log10(x)
'''

[test.assertions]
any_of = ["x.log10()", "f64::log10"]
```

```toml
[[test]]
name = "math_log2"
description = "math.log2() base-2 logarithm"

[test.python]
code =  '''
import math

def calculate_log2(x: float) -> float:
    return math.log2(x)
'''

[test.assertions]
any_of = ["x.log2()", "f64::log2"]
```

### Power and Root Functions

```toml
[[test]]
name = "math_sqrt"
description = "math.sqrt() square root"

[test.python]
code =  '''
import math

def calculate_sqrt(x: float) -> float:
    return math.sqrt(x)
'''

[test.assertions]
any_of = ["x.sqrt()", "f64::sqrt"]
```

```toml
[[test]]
name = "math_pow"
description = "math.pow() power function"

[test.python]
code =  '''
import math

def calculate_pow(x: float, y: float) -> float:
    return math.pow(x, y)
'''

[test.assertions]
any_of = ["powf", "powi"]
```

### Rounding Functions

```toml
[[test]]
name = "math_floor"
description = "math.floor() function"

[test.python]
code =  '''
import math

def calculate_floor(x: float) -> int:
    return math.floor(x)
'''

[test.assertions]
any_of = ["x.floor()", "f64::floor"]
```

```toml
[[test]]
name = "math_ceil"
description = "math.ceil() function"

[test.python]
code =  '''
import math

def calculate_ceil(x: float) -> int:
    return math.ceil(x)
'''

[test.assertions]
any_of = ["x.ceil()", "f64::ceil"]
```

```toml
[[test]]
name = "math_trunc"
description = "math.trunc() truncation"

[test.python]
code =  '''
import math

def calculate_trunc(x: float) -> int:
    return math.trunc(x)
'''

[test.assertions]
any_of = ["x.trunc()", "f64::trunc"]
```

### Built-in Math Functions

```toml
[[test]]
name = "builtin_abs"
description = "abs() builtin function"

[test.python]
code =  '''
def test_abs(value: int) -> int:
    return abs(value)
'''

[test.assertions]
contains = ["value.abs()"]
```

```toml
[[test]]
name = "builtin_round"
description = "round() builtin function"

[test.python]
code =  '''
def round_to_int(x: float) -> int:
    return round(x)
'''

[test.assertions]
contains = ["round()"]
any_of = ["x.round() as i32", "x.round()"]
```

```toml
[[test]]
name = "builtin_pow"
description = "pow() builtin function"

[test.python]
code =  '''
def power(base: int, exp: int) -> int:
    return pow(base, exp)
'''

[test.assertions]
any_of = [".pow(", ".checked_pow("]
```

```toml
[[test]]
name = "builtin_divmod"
description = "divmod() builtin function"

[test.python]
code =  '''
def divide_with_remainder(a: int, b: int) -> tuple[int, int]:
    return divmod(a, b)
'''

[test.assertions]
any_of = ["a / b", "a % b", "(", ")"]
```

### Math Constants

```toml
[[test]]
name = "math_pi"
description = "math.pi constant"

[test.python]
code =  '''
import math

def circle_area(r: float) -> float:
    return math.pi * r * r
'''

[test.assertions]
any_of = ["std::f64::consts::PI", "3.14159"]
```

```toml
[[test]]
name = "math_e"
description = "math.e constant"

[test.python]
code =  '''
import math

def natural_exp(x: float) -> float:
    return math.e ** x
'''

[test.assertions]
any_of = ["std::f64::consts::E", "2.71828"]
```

### Hyperbolic Functions

```toml
[[test]]
name = "math_sinh"
description = "math.sinh() function"

[test.python]
code =  '''
import math

def calculate_sinh(x: float) -> float:
    return math.sinh(x)
'''

[test.assertions]
any_of = ["x.sinh()", "f64::sinh"]
```

### Sum with Float Type Inference

```toml
[[test]]
name = "sum_float_list"
description = "sum() on list of floats from math.exp"

[test.python]
code =  '''
import math

def softmax(logits: list[float]) -> list[float]:
    max_logit = max(logits)
    exps = [math.exp(x - max_logit) for x in logits]
    total = sum(exps)
    return [e / total for e in exps]
'''

[test.assertions]
contains = ["sum::<f64>()"]
not_contains = ["sum::<i32>()"]
```

## Notes

- Python `math` module → Rust `f64` methods or `std::f64::consts`
- `math.sin(x)` → `x.sin()`
- `math.sqrt(x)` → `x.sqrt()`
- `math.pi` → `std::f64::consts::PI`
- `abs(x)` → `x.abs()` (available on numeric types)
- `round(x)` → `x.round()` (returns f64, cast if needed)

## Acceptance Criteria

- [ ] Trigonometric function tests migrated
- [ ] Exponential/log function tests migrated
- [ ] Power/root function tests migrated
- [ ] Rounding function tests migrated
- [ ] Built-in math tests migrated
- [ ] Math constant tests migrated

## Estimated Effort

**Time**: 3-4 hours
**Risk**: Low
