# 009: Optional with Default Values (or Pattern)

**Priority**: Medium  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

Python's `or` operator is commonly used to provide default values for Optional types. When the left operand is `None` (or falsy), the right operand is returned. This pattern needs special handling in Rust.

## Current Behavior

```python
@dataclass
class Person:
    name: Optional[str]

def get_name(p: Person) -> str:
    return p.name or "Unknown"  # p.name is Optional[str]
```

**Generated (Incorrect)**:
```rust
pub fn get_name(p: &Person) -> String {
    return p.name || "Unknown".to_string();  // Error: cannot apply `||` to Option<String>
}
```

## Expected Behavior

```rust
pub fn get_name(p: &Person) -> String {
    return p.name.clone().unwrap_or_else(|| "Unknown".to_string());
}
```

## Python `or` Semantics

Python's `or` returns the first truthy value or the last value:

```python
None or "default"      # → "default"
"" or "default"        # → "default" (empty string is falsy)
"value" or "default"   # → "value"
0 or 42                # → 42 (0 is falsy)
[] or [1,2,3]          # → [1,2,3] (empty list is falsy)
```

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (line 726-781)

The `convert_binary()` function handles `BinOp::Or` with Optional types:

```rust
BinOp::Or => {
    // Check if left operand is Optional - use unwrap_or_else pattern
    if let Some(inner_type) = self.ctx.get_optional_inner_type(left) {
        let left_no_unwrap = self.convert_expr_no_unwrap(left)?;
        
        // Check if we need to handle chained optionals: opt1 or opt2 or default
        let right_is_optional = self.ctx.get_optional_inner_type(right).is_some();
        
        if right_is_optional {
            // Chained optionals: opt1 or opt2 -> opt1.or_else(|| opt2)
            return Ok(parse_quote! { #left_no_unwrap.clone().or_else(|| #right_no_unwrap.clone()) });
        }
        
        // For string default values, add .to_string()
        // Other types: use unwrap_or_else directly
        return Ok(parse_quote! { #left_no_unwrap.clone().unwrap_or_else(|| #right_expr) });
    }
    // ...
}
```

Also handles chained `or` expressions like `opt1 or opt2 or "default"`.

## Original Implementation Approach (Historical)
        let left_expr = self.convert_expr(left)?;
        let right_expr = self.convert_expr(right)?;
        
        let inner_type = self.get_optional_inner_type(left);
        
        // For strings: handle empty string as falsy
        if matches!(inner_type, Some(Type::String)) {
            return Ok(parse_quote! {
                #left_expr.clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| #right_expr)
            });
        }
        
        // For numbers: handle 0 as falsy (if needed)
        // For general case: just use unwrap_or_else
        return Ok(parse_quote! {
            #left_expr.clone().unwrap_or_else(|| #right_expr)
        });
    }
    
    // Normal boolean or
    let left_expr = self.convert_expr(left)?;
    let right_expr = self.convert_expr(right)?;
    Ok(parse_quote! { #left_expr || #right_expr })
}
```

### 2. Handle Chained `or`

```python
def get_display_name(p: Person) -> str:
    return p.nickname or p.name or "Anonymous"
```

**Expected**:
```rust
p.nickname.clone()
    .or_else(|| p.name.clone())
    .unwrap_or_else(|| "Anonymous".to_string())
```

### 3. Type-Specific Handling

| Type | Python Falsy Values | Rust Equivalent |
|------|--------------------|-----------------| 
| `Optional[str]` | `None`, `""` | `.filter(\|s\| !s.is_empty()).unwrap_or(...)` |
| `Optional[int]` | `None`, `0` | `.filter(\|n\| *n != 0).unwrap_or(...)` |
| `Optional[List]` | `None`, `[]` | `.filter(\|v\| !v.is_empty()).unwrap_or(...)` |
| `Optional[T]` | `None` | `.unwrap_or(...)` |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Modify boolean `or` handling for Optional |

## Tests to Add

```python
def test_optional_or_default():
    @dataclass
    class Person:
        name: Optional[str]
    
    def get_name(p: Person) -> str:
        return p.name or "Unknown"

def test_chained_or():
    @dataclass
    class Person:
        nickname: Optional[str]
        name: Optional[str]
    
    def get_display(p: Person) -> str:
        return p.nickname or p.name or "Anonymous"

def test_optional_int_or():
    @dataclass
    class Counter:
        value: Optional[int]
    
    def get_value(c: Counter) -> int:
        return c.value or 0

def test_empty_string_or():
    def get_name(name: Optional[str]) -> str:
        return name or "default"  # Empty string should trigger default
```

## Edge Cases

1. **Type mismatch**: `p.optional_int or "default"` - type error
2. **Both Optional**: `p.name or p.backup_name` - keep as Optional
3. **Non-Optional or**: `"a" or "b"` - standard boolean logic
4. **Side effects**: Right side should only evaluate if left is falsy

## Decision: Match Python Truthiness?

**Option A: Simple unwrap_or** (recommended for MVP)
- Treats only `None` as needing default
- Simpler code generation
- Different from Python for empty strings/zero

**Option B: Full Python truthiness**
- Filter empty strings, zero, empty lists
- More complex code generation  
- Exact Python semantics

## Acceptance Criteria

- [ ] `Optional[T] or default` generates `unwrap_or_else`
- [ ] Chained `or` expressions handled
- [ ] Return type inferred correctly (inner type, not Optional)
- [ ] String empty check if matching full Python semantics
- [ ] Unit tests passing
