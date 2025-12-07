# Migration Task: Type Inference Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_type_inference.rs | 364 | ~25 | `transpile_and_check` |
| test_type_system.rs | ~200 | ~12 | `transpile_and_check` |
| test_type_mapper_coverage.rs | ~300 | ~18 | `transpile_and_check` |
| test_type_mapper_extended_coverage.rs | ~200 | ~12 | `transpile_and_check` |
| test_type_mapper_propertys.rs | ~150 | ~8 | Property-based |
| test_type_gen_coverage.rs | ~180 | ~10 | `transpile_and_check` |
| test_type_annotation.rs | ~120 | ~6 | `transpile_and_check` |
| test_generic_inference.rs | ~100 | ~5 | `transpile_and_check` |
| test_return_type_inference.rs | ~150 | ~8 | `transpile_and_check` |

## Target File

`tests/toml/16-type-inference.toml`

## Tests to Migrate

### Basic Type Inference

```toml
[[test]]
name = "infer_int_from_literal"
description = "Infer int type from literal assignment"

[test.python]
code =  '''
def get_number():
    x = 42
    return x
'''

[test.assertions]
any_of = ["i32", "i64"]
not_contains = ["serde_json::Value"]
```

```toml
[[test]]
name = "infer_float_from_literal"
description = "Infer float type from literal"

[test.python]
code =  '''
def get_pi():
    x = 3.14
    return x
'''

[test.assertions]
contains = ["f64"]
```

```toml
[[test]]
name = "infer_string_from_literal"
description = "Infer string type from literal"

[test.python]
code =  '''
def get_greeting():
    s = "hello"
    return s
'''

[test.assertions]
any_of = ["String", "&str"]
```

```toml
[[test]]
name = "infer_bool_from_literal"
description = "Infer bool type from literal"

[test.python]
code =  '''
def get_flag():
    b = True
    return b
'''

[test.assertions]
contains = ["bool"]
```

### Inference from Operations

```toml
[[test]]
name = "infer_int_from_arithmetic"
description = "Infer int type from arithmetic"

[test.python]
code =  '''
def increment(x):
    return x + 1
'''

[test.assertions]
any_of = ["i32", "i64", "isize"]
not_contains = ["serde_json::Value"]
```

```toml
[[test]]
name = "infer_float_from_division"
description = "Infer float from true division"

[test.python]
code =  '''
def half(x):
    return x / 2
'''

[test.assertions]
contains = ["f64"]
```

```toml
[[test]]
name = "infer_string_from_concat"
description = "Infer string from concatenation"

[test.python]
code =  '''
def greet(name):
    return "Hello, " + name
'''

[test.assertions]
any_of = ["String", "&str"]
```

### Inference from Method Calls

```toml
[[test]]
name = "infer_string_from_upper"
description = "Infer string from .upper() call"

[test.python]
code =  '''
def shout(text):
    return text.upper()
'''

[test.assertions]
any_of = ["String", "&str"]
contains = ["to_uppercase"]
```

```toml
[[test]]
name = "infer_list_from_append"
description = "Infer list from .append() call"

[test.python]
code =  '''
def add_item(items, item):
    items.append(item)
    return items
'''

[test.assertions]
contains = ["Vec"]
```

### Inference from Builtins

```toml
[[test]]
name = "infer_int_from_len"
description = "Infer int from len()"

[test.python]
code =  '''
def count(items):
    return len(items)
'''

[test.assertions]
any_of = ["usize", "i32"]
```

```toml
[[test]]
name = "infer_string_from_str"
description = "Infer string from str()"

[test.python]
code =  '''
def to_string(x):
    return str(x)
'''

[test.assertions]
contains = ["String", "to_string"]
```

### Inference from Context

```toml
[[test]]
name = "infer_file_from_open"
description = "Infer file type from open()"

[test.python]
code =  '''
def read_file(filepath):
    with open(filepath) as f:
        return f.read()
'''

[test.assertions]
any_of = ["&str", "String", "Path"]
not_contains = ["serde_json::Value"]
```

```toml
[[test]]
name = "infer_from_comparison"
description = "Infer comparable types"

[test.python]
code =  '''
def is_positive(x):
    return x > 0
'''

[test.assertions]
any_of = ["i32", "i64", "f64"]
contains = ["bool"]
```

### Return Type Inference

```toml
[[test]]
name = "infer_return_int"
description = "Infer return type as int"

[test.python]
code =  '''
def add(a: int, b: int):
    return a + b
'''

[test.assertions]
contains = ["-> i32"]
```

```toml
[[test]]
name = "infer_return_optional"
description = "Infer Optional return type"

[test.python]
code =  '''
def find_first(items: list[int], target: int):
    for item in items:
        if item == target:
            return item
    return None
'''

[test.assertions]
contains = ["Option<"]
```

### Generic Type Inference

```toml
[[test]]
name = "infer_list_element_type"
description = "Infer list element type"

[test.python]
code =  '''
def first_element(items):
    return items[0]
'''

[test.assertions]
any_of = ["Vec<", "[T]"]
```

```toml
[[test]]
name = "infer_dict_types"
description = "Infer dict key and value types"

[test.python]
code =  '''
def get_value(d, key):
    return d[key]
'''

[test.assertions]
contains = ["HashMap"]
```

### Type Annotations

```toml
[[test]]
name = "annotated_param_int"
description = "Explicit int annotation"

[test.python]
code =  '''
def square(x: int) -> int:
    return x * x
'''

[test.assertions]
contains = ["x: i32", "-> i32"]
```

```toml
[[test]]
name = "annotated_param_list"
description = "Explicit list annotation"

[test.python]
code =  '''
from typing import List

def sum_list(items: List[int]) -> int:
    return sum(items)
'''

[test.assertions]
contains = ["Vec<i32>", "-> i32"]
```

```toml
[[test]]
name = "annotated_param_dict"
description = "Explicit dict annotation"

[test.python]
code =  '''
from typing import Dict

def count_words(text: str) -> Dict[str, int]:
    result = {}
    for word in text.split():
        result[word] = result.get(word, 0) + 1
    return result
'''

[test.assertions]
contains = ["HashMap<String, i32>"]
```

### Union Types

```toml
[[test]]
name = "union_int_str"
description = "Union of int and str"

[test.python]
code =  '''
from typing import Union

def process(value: Union[int, str]) -> str:
    return str(value)
'''

[test.assertions]
any_of = ["enum", "Either", "union"]
```

```toml
[[test]]
name = "union_optional"
description = "Optional (Union with None)"

[test.python]
code =  '''
from typing import Optional

def maybe_int(x: Optional[int]) -> int:
    return x if x is not None else 0
'''

[test.assertions]
contains = ["Option<i32>"]
```

## Notes

- Python is dynamically typed; inference fills in types
- Type hints are authoritative when present
- Operations provide type constraints
- Method calls indicate receiver type
- Builtins have known return types
- `None` indicates `Option<T>`
- Unions may become enums or `Either`

## Acceptance Criteria

- [ ] Basic type inference tests migrated
- [ ] Operation-based inference tests migrated
- [ ] Method call inference tests migrated
- [ ] Return type inference tests migrated
- [ ] Type annotation tests migrated
- [ ] Union type tests migrated

## Estimated Effort

**Time**: 4-5 hours
**Risk**: Medium (type inference is complex)
