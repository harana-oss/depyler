# 011: Method Chaining on Optional Results

**Priority**: Medium  
**Status**: ⚠️ Partial Implementation  
**Complexity**: High

## Problem

When a method returns Optional and another method is immediately called on the result, the transpiler needs to insert an unwrap between the calls.

## Current Behavior

```python
def find_and_upper(d: dict, key: str) -> str:
    return d.get(key).upper()  # .get() returns Optional[str]
```

**Generated (Incorrect)**:
```rust
pub fn find_and_upper(d: &HashMap<String, String>, key: &str) -> String {
    return d.get(key).upper();  // Error: Option<&String> has no method `upper`
}
```

## Expected Behavior

```rust
pub fn find_and_upper(d: &HashMap<String, String>, key: &str) -> String {
    return d.get(key).unwrap().to_uppercase();
}
```

## Common Patterns

### Dict.get() Chaining

```python
name = data.get("name").strip()
value = config.get("setting").lower()
```

### List Method Chaining

```python
first = items.pop().upper()  # pop() can return Optional
result = values.get(0).process()  # indexing returns Optional
```

### Attribute + Method

```python
result = obj.optional_field.method()  # Field is Optional
```

## Partial Implementation Status

**Current Support:**
- Basic method unwrap for string methods when receiver is Optional (line 9997-10002)
- Attribute access chaining with Optional intermediate fields (line 11514-11517)

**Not Yet Implemented:**
- Explicit tracking of which methods return Optional types
- Automatic unwrapping of `dict.get()` return before chained method calls
- `list.pop()` return type tracking

The issue is that the transpiler doesn't have a comprehensive registry of which standard library methods return `Option<T>` in Rust.

## Remaining Implementation Approach

### 1. Track Method Return Types

Need to add a registry of which methods return Optional:
        // ... more methods
        _ => None,
    }
}
```

### 2. Modify Method Call Chaining

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`**

```rust
fn convert_method_call(&mut self, receiver: &HirExpr, method: &str, args: &[HirExpr]) -> Result<syn::Expr> {
    // First, convert the receiver
    let receiver_expr = self.convert_expr(receiver)?;
    
    // Check if receiver is a method call that returns Optional
    if let HirExpr::Call { func, args: inner_args } = receiver {
        if let HirExpr::Attribute { value, attr } = func.as_ref() {
            let inner_return_type = self.get_method_return_type(/* receiver type */, attr);
            
            if matches!(inner_return_type, Some(Type::Optional(_))) {
                // Insert unwrap before the outer method
                let inner_call = self.convert_method_call(value, attr, inner_args)?;
                let unwrapped = parse_quote! { #inner_call.unwrap() };
                return self.apply_method(unwrapped, method, args);
            }
        }
    }
    
    // Normal method call
    self.apply_method(receiver_expr, method, args)
}
```

### 3. Methods That Return Optional

| Type | Method | Returns |
|------|--------|---------|
| `Dict[K,V]` | `get(key)` | `Optional[V]` |
| `List[T]` | `pop()` | `Optional[T]` |
| `List[T]` | `pop(idx)` | `Optional[T]` |
| `str` | `find(sub)` | `Optional[int]` |
| `Any` | `__getattr__` | Depends |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Add method return type tracking, modify chain handling |
| `crates/depyler-core/src/rust_gen/context.rs` | May need method signature registry |

## Tests to Add

```python
def test_dict_get_chain():
    def process(d: Dict[str, str], key: str) -> str:
        return d.get(key).upper()

def test_list_pop_chain():
    def process(items: List[str]) -> str:
        return items.pop().lower()

def test_double_chain():
    def process(d: Dict[str, str]) -> str:
        return d.get("a").strip().upper()

def test_mixed_chain():
    @dataclass
    class Container:
        data: Dict[str, str]
    
    def process(c: Container, key: str) -> str:
        return c.data.get(key).upper()
```

## Edge Cases

1. **Multiple chained Optional methods**: `d.get(k1).get(k2).value`
2. **Optional field + method returning Optional**: `obj.opt_dict.get(key)`
3. **Chained with non-Optional**: `d.get(key).upper().strip()` - only first needs unwrap
4. **Method on unwrapped value returning Optional**: Recursive problem

## Challenges

- Need comprehensive method signature database
- Complex type inference through chains
- Interaction with field access (004)
- Performance of deep chain analysis

## Alternative: Use `map` Chain

Instead of unwrapping, could use functional style:

```rust
// Instead of: d.get(key).unwrap().to_uppercase()
// Generate:   d.get(key).map(|s| s.to_uppercase())
```

This changes return type to `Option<String>` - may require context awareness.

## Acceptance Criteria

- [ ] Common method return types catalogued
- [ ] Method chains detected during conversion
- [ ] Unwrap inserted between Optional-returning method and next call
- [ ] Works with dict.get(), list.pop(), etc.
- [ ] Multiple chain levels handled
- [ ] Unit tests passing
