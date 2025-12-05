# 003: Optional Auto-Unwrap for Method Calls

**Priority**: Medium  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

When calling methods on Optional fields, the transpiler should unwrap the field first before dispatching the method.

## Current Behavior

```python
@dataclass
class Container:
    items: Optional[List[int]]

def count_items(c: Container) -> int:
    return len(c.items)  # c.items is Optional[List[int]]
```

**Generated (Potentially Incorrect)**:
```rust
pub fn count_items(c: &Container) -> i32 {
    return c.items.len();  // Error: Option<Vec<i32>> has no method `len`
}
```

## Expected Behavior

```rust
pub fn count_items(c: &Container) -> i32 {
    return c.items.as_ref().unwrap().len() as i32;
}
```

## Implementation Approach

### 1. Extend `convert_with_optional_unwrap` 

The existing helper at `expr_gen.rs:11504` handles binary operations. Extend or create a similar helper for method receivers.

### Implementation Location

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (line 9997-10002)

The `convert_method_call()` function checks if the object is Optional and unwraps it for string methods:

```rust
// Check if the object is an Optional type that needs unwrapping
let object_is_optional = self.expr_is_optional(object);
let mut object_expr = object.to_rust_expr(self.ctx)?;
if object_is_optional {
    // Unwrap Optional before calling string method
    object_expr = parse_quote! { #object_expr.as_ref().unwrap() };
}
```

        let base = self.convert_expr(receiver)?;
        parse_quote! { #base.as_ref().unwrap() }
    } else {
        self.convert_expr(receiver)?
    };
    
    let converted_args = self.convert_args(args)?;
    Ok(parse_quote! { #receiver_expr.#method(#(#converted_args),*) })
}
```

### 3. Built-in Functions on Optional

Handle Python built-ins called on Optional values:

| Python | Rust with Optional |
|--------|-------------------|
| `len(opt_list)` | `opt_list.as_ref().unwrap().len()` |
| `str(opt_val)` | `opt_val.as_ref().unwrap().to_string()` |
| `int(opt_str)` | `opt_str.as_ref().unwrap().parse::<i32>()` |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Modify method call handling to check Optional receivers |

## Tests to Add

```python
def test_len_on_optional_list():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def count(c: Container) -> int:
        return len(c.items)

def test_method_on_optional_string():
    @dataclass
    class Person:
        name: Optional[str]
    
    def upper_name(p: Person) -> str:
        return p.name.upper()

def test_append_on_optional_list():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def add_item(c: Container, item: int) -> None:
        c.items.append(item)
```

## Edge Cases

1. **Option-native methods**: Don't unwrap for `is_some()`, `is_none()`, `map()`, etc.
2. **Chained methods**: `p.name.upper().strip()` where `name` is Optional
3. **Mutable methods**: `list.append()` needs `as_mut().unwrap()`
4. **Built-in functions**: `len()`, `str()`, `int()` called on Optional values

## Acceptance Criteria

- [ ] Method calls on Optional receivers are unwrapped
- [ ] Option-native methods (`is_some`, etc.) NOT unwrapped
- [ ] Built-in functions (`len`, `str`) handle Optional correctly
- [ ] Mutable methods use `as_mut().unwrap()` 
- [ ] Chained methods work correctly
- [ ] Unit tests passing
