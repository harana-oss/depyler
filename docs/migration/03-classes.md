# Migration Task: Class Tests

## Status: Not Started

## Source Files

| File | Lines | Tests (est) | Pattern |
|------|-------|-------------|---------|
| test_classs.rs | 765 | ~40 | `transpile_and_check` |
| test_property.rs | ~150 | ~8 | `transpile_and_check` |
| test_converters_propertys.rs | ~100 | ~5 | `transpile_and_check` |
| test_setattr.rs | ~80 | ~4 | `transpile_and_check` |

## Target File

`tests/toml/03-classes.toml`

## Tests to Migrate

### Phase 1: Basic Struct Generation

```toml
[[test]]
name = "class_simple_struct"
description = "Simple class should generate Rust struct"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y
'''

[test.assertions]
contains = ["struct Point", "x:", "y:", "impl Point", "fn new"]
```

```toml
[[test]]
name = "class_multiple_fields"
description = "Class with multiple typed fields"

[test.python]
code =  '''
class Student:
    def __init__(self, name: str, age: int, grade: float, active: bool):
        self.name = name
        self.age = age
        self.grade = grade
        self.active = active
'''

[test.assertions]
contains = ["struct Student", "name", "age", "grade", "active", "String", "i32", "f64", "bool"]
```

```toml
[[test]]
name = "class_empty"
description = "Empty class with pass"

[test.python]
code =  '''
class Empty:
    pass
'''

[test.assertions]
contains = ["struct Empty"]
```

### Phase 2: Instance Methods

```toml
[[test]]
name = "class_instance_method"
description = "Class with instance method"

[test.python]
code =  '''
class Rectangle:
    def __init__(self, width: int, height: int):
        self.width = width
        self.height = height
    
    def area(self) -> int:
        return self.width * self.height
'''

[test.assertions]
contains = ["struct Rectangle", "impl Rectangle", "fn area", "&self"]
```

```toml
[[test]]
name = "class_method_with_params"
description = "Instance method with additional parameters"

[test.python]
code =  '''
class Calculator:
    def __init__(self, value: int):
        self.value = value
    
    def add(self, n: int) -> int:
        return self.value + n
'''

[test.assertions]
contains = ["fn add", "&self", "n: i32"]
```

```toml
[[test]]
name = "class_mutating_method"
description = "Method that mutates self"

[test.python]
code =  '''
class Counter:
    def __init__(self):
        self.count = 0
    
    def increment(self):
        self.count += 1
'''

[test.assertions]
contains = ["&mut self", "fn increment"]
```

### Phase 3: Static and Class Methods

```toml
[[test]]
name = "class_staticmethod"
description = "@staticmethod decorator"

[test.python]
code =  '''
class MathUtils:
    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b
'''

[test.assertions]
contains = ["fn add", "a: i32", "b: i32"]
not_contains = ["&self", "&mut self"]
```

```toml
[[test]]
name = "class_classmethod"
description = "@classmethod decorator"

[test.python]
code =  '''
class Factory:
    @classmethod
    def create(cls, value: int):
        return cls(value)
'''

[test.assertions]
contains = ["fn create"]
```

### Phase 4: Properties

```toml
[[test]]
name = "class_property_getter"
description = "@property decorator for getter"

[test.python]
code =  '''
class Circle:
    def __init__(self, radius: float):
        self._radius = radius
    
    @property
    def radius(self) -> float:
        return self._radius
'''

[test.assertions]
contains = ["fn radius", "&self", "f64"]
```

```toml
[[test]]
name = "class_property_setter"
description = "@property.setter decorator"

[test.python]
code =  '''
class Circle:
    def __init__(self, radius: float):
        self._radius = radius
    
    @property
    def radius(self) -> float:
        return self._radius
    
    @radius.setter
    def radius(self, value: float):
        self._radius = value
'''

[test.assertions]
contains = ["fn radius", "fn set_radius", "&mut self"]
```

### Phase 5: Class Instantiation

```toml
[[test]]
name = "class_instantiation"
description = "Creating class instance"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

def create_point() -> Point:
    p = Point(10, 20)
    return p
'''

[test.assertions]
contains = ["Point::new", "10", "20"]
any_of = ["Point::new(", "Point {"]
```

```toml
[[test]]
name = "class_field_access"
description = "Accessing instance fields"

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
```

### Phase 6: Default Constructor Parameters

```toml
[[test]]
name = "class_default_param"
description = "__init__ with default parameter"

[test.python]
code =  '''
class Counter:
    def __init__(self, start: int = 0):
        self.value = start
'''

[test.assertions]
contains = ["struct Counter", "value"]
```

### Phase 7: Multiple Classes

```toml
[[test]]
name = "multiple_classes"
description = "Multiple class definitions in one module"

[test.python]
code =  '''
class Point:
    def __init__(self, x: int, y: int):
        self.x = x
        self.y = y

class Rectangle:
    def __init__(self, origin: Point, width: int, height: int):
        self.origin = origin
        self.width = width
        self.height = height
'''

[test.assertions]
contains = ["struct Point", "struct Rectangle", "origin: Point"]
```

## Notes

- Class methods translate to `impl` blocks
- `self` → `&self` or `&mut self` based on mutation analysis
- `__init__` → `fn new()`
- Private fields (`_name`) may get special handling
- @property creates getter methods
- @staticmethod removes self parameter

## Acceptance Criteria

- [ ] Basic struct generation tests migrated
- [ ] Instance method tests migrated  
- [ ] Static/class method tests migrated
- [ ] Property decorator tests migrated
- [ ] Instantiation and field access tests migrated
- [ ] Multiple class tests migrated

## Estimated Effort

**Time**: 4-5 hours
**Risk**: Medium (many edge cases in class handling)
