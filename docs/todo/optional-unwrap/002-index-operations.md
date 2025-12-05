# 002: Optional Auto-Unwrap for Index Operations

**Priority**: High  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

Indexing into Optional collections should unwrap the collection first before applying the index operation.

## Current Behavior

```python
@dataclass  
class Data:
    values: Optional[List[int]]

def get_first(d: Data) -> int:
    return d.values[0]  # d.values is Optional[List[int]]
```

**Generated (Incorrect)**:
```rust
pub fn get_first(d: &Data) -> i32 {
    return d.values[0];  // Error: cannot index into Option<Vec<i32>>
}
```

## Expected Behavior

```rust
pub fn get_first(d: &Data) -> i32 {
    return d.values.as_ref().unwrap()[0];
}
```

## Implementation Approach

### Implementation Location

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (line 10260-10267)

The `convert_index()` function checks if base is Optional and adds `.as_ref().unwrap()`:

```rust
// Check if base is Optional - if so, we need to unwrap it before indexing
let base_is_optional = self.ctx.get_optional_inner_type(base).is_some();

let mut base_expr = base.to_rust_expr(self.ctx)?;

// If base is Optional, add .as_ref().unwrap() to unwrap the Option before indexing
if base_is_optional {
    base_expr = parse_quote! { #base_expr.as_ref().unwrap() };
}
```

### 2. Handle Different Collection Types

Different Optional collection types may need slightly different handling:

| Type | Unwrap Pattern |
|------|----------------|
| `Optional[List[T]]` | `.as_ref().unwrap()[idx]` |
| `Optional[Dict[K,V]]` | `.as_ref().unwrap().get(&key)` or `[&key]` |
| `Optional[str]` | `.as_ref().unwrap().chars().nth(idx)` |

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Modify `convert_subscript()` to check if base is Optional |

## Tests to Add

```python
def test_optional_list_indexing():
    @dataclass
    class Container:
        items: Optional[List[int]]
    
    def get_item(c: Container, idx: int) -> int:
        return c.items[idx]  # Should generate: c.items.as_ref().unwrap()[idx]

def test_optional_dict_indexing():
    @dataclass
    class Config:
        settings: Optional[Dict[str, int]]
    
    def get_setting(c: Config, key: str) -> int:
        return c.settings[key]  # Should generate appropriate unwrap
```

## Edge Cases

1. **Nested indexing**: `container.matrix[i][j]` where `matrix` is Optional
2. **Slice operations**: `container.items[1:3]` on Optional list
3. **Negative indexing**: Python's negative index semantics
4. **Dict with `.get()`**: Already handled separately - ensure no double handling

## Acceptance Criteria

- [ ] `convert_subscript()` checks if base expression is Optional
- [ ] Appropriate `.as_ref().unwrap()` added before indexing
- [ ] Works for `List`, `Dict`, and string indexing
- [ ] Slice operations handled correctly
- [ ] Unit tests passing
- [ ] No double-unwrap when already handled
