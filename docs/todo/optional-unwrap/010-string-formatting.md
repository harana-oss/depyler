# 010: Optional in String Formatting (F-Strings)

**Priority**: Medium  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

F-strings containing Optional fields need special handling. In Python, formatting `None` produces the string `"None"`. The transpiler must decide whether to mimic this behavior or require unwrapping.

## Current Behavior

```python
@dataclass
class Person:
    name: Optional[str]
    age: Optional[int]

def format_person(p: Person) -> str:
    return f"Name: {p.name}, Age: {p.age}"
```

**Generated (Potentially Incorrect)**:
```rust
pub fn format_person(p: &Person) -> String {
    format!("Name: {}, Age: {}", p.name, p.age)
    // Error: Option<String> doesn't implement Display directly
}
```

## Expected Behavior Options

### Option A: Unwrap (panic on None)

```rust
pub fn format_person(p: &Person) -> String {
    format!("Name: {}, Age: {}", p.name.as_ref().unwrap(), p.age.unwrap())
}
```

### Option B: Display "None" (match Python)

```rust
pub fn format_person(p: &Person) -> String {
    format!("Name: {}, Age: {}",
        p.name.as_ref().map(|s| s.as_str()).unwrap_or("None"),
        p.age.map(|n| n.to_string()).unwrap_or_else(|| "None".to_string())
    )
}
```

### Option C: Use Debug formatting

```rust
pub fn format_person(p: &Person) -> String {
    format!("Name: {:?}, Age: {:?}", p.name, p.age)
}
```

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (line 12808-12819)

The `convert_fstring()` function handles Optional expressions by wrapping them in a match:

```rust
let final_arg = if is_option {
    // Option<T> doesn't implement Display, so we need to unwrap it
    // For string-like types, display the value or "None"
    parse_quote! {
        {
            match &#arg_expr {
                Some(v) => format!("{}", v),
                None => "None".to_string(),
            }
        }
    }
} else {
    arg_expr
};
```

This implements Option B (Python-like behavior) where `None` displays as the string "None".

## Original Implementation Approach (Historical)
                #value_expr.map(|n| n.to_string()).unwrap_or_else(|| "None".to_string())
            }),
            _ => Ok(quote! {
                #value_expr.as_ref().map(|v| format!("{:?}", v)).unwrap_or_else(|| "None".to_string())
            }),
        };
    }
    
    Ok(quote! { #value_expr })
}
```

### 2. Handle Format Specifiers

Python f-strings support format specifiers:

```python
f"{p.age:05d}"  # Zero-padded
f"{p.name:>10}"  # Right-aligned
f"{p.value:.2f}" # Float precision
```

These need special handling with Optional:

```rust
// For f"{p.age:05d}" where age is Optional[int]
format!("{:05}", p.age.unwrap())
```

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Modify f-string value conversion |

## Tests to Add

```python
def test_fstring_optional_string():
    @dataclass
    class Person:
        name: Optional[str]
    
    def format_name(p: Person) -> str:
        return f"Name: {p.name}"

def test_fstring_optional_int():
    @dataclass
    class Person:
        age: Optional[int]
    
    def format_age(p: Person) -> str:
        return f"Age: {p.age}"

def test_fstring_with_format_spec():
    @dataclass
    class Data:
        value: Optional[float]
    
    def format_value(d: Data) -> str:
        return f"Value: {d.value:.2f}"  # Needs unwrap for format spec

def test_fstring_multiple_optional():
    @dataclass
    class Person:
        name: Optional[str]
        age: Optional[int]
    
    def format_all(p: Person) -> str:
        return f"{p.name} is {p.age} years old"
```

## Edge Cases

1. **Format specifiers**: `{x:05d}` requires unwrapping
2. **Conversion flags**: `{x!r}`, `{x!s}` - repr vs str
3. **Mixed Optional/non-Optional**: Multiple values in one f-string
4. **Nested expressions**: `f"{p.name.upper()}"` where name is Optional
5. **Expression in f-string**: `f"{p.age + 1}"` where age is Optional

## Design Decision

| Approach | Pros | Cons |
|----------|------|------|
| Always unwrap | Simple, explicit | Panics on None |
| Display "None" | Matches Python | More complex code |
| Use `{:?}` | Simple, shows `Some(x)` | Different output format |

**Recommendation**: Match Python behavior (display "None") for string fields, unwrap for format specifiers.

## Acceptance Criteria

- [ ] Optional string fields in f-strings handled
- [ ] Optional numeric fields in f-strings handled
- [ ] Format specifiers work with unwrapping
- [ ] None displays as "None" (Python behavior)
- [ ] Mixed Optional/non-Optional f-strings work
- [ ] Unit tests passing
