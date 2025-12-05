# 001: Optional Auto-Unwrap for Function Arguments

**Priority**: High  
**Status**: ✅ Implemented  
**Complexity**: Medium-High

## Problem

When an Optional field is passed as an argument to a function that expects the inner type (not Optional), the transpiler should automatically add `.unwrap()`.

## Current Behavior

```python
def process_age(age: int) -> int:
    return age + 10

def use_person(person: Person) -> int:
    return process_age(person.age)  # person.age is Optional[int]
```

**Generated (Incorrect)**:
```rust
pub fn use_person(person: &Person) -> i32 {
    return process_age(person.age);  // Error: expected i32, found Option<i32>
}
```

## Expected Behavior

```rust
pub fn use_person(person: &Person) -> i32 {
    return process_age(person.age.unwrap());  // Correct
}
```

## Implementation Approach

### 1. Add Parameter Types to Context

The `CodeGenContext` needs access to parameter type information for called functions. Currently available:
- `function_return_types: HashMap<String, Type>` ✅
- `function_param_borrows: HashMap<String, Vec<bool>>` ✅
- `function_param_muts: HashMap<String, Vec<bool>>` ✅
- `function_param_names: HashMap<String, Vec<String>>` ✅
- `function_param_types: HashMap<String, Vec<Type>>` ✅ **Added**

### 2. Implementation Location

**File: `crates/depyler-core/src/rust_gen/context.rs`** (line 121)
```rust
pub function_param_types: HashMap<String, Vec<Type>>,
```

**File: `crates/depyler-core/src/rust_gen/expr_gen.rs`** (around line 2896)
- The `convert_call()` function looks up parameter types
- Adds `.unwrap()` when Optional argument is passed to non-Optional parameter
                return Ok(arg_expr);
            }
            // Parameter expects inner type, unwrap
            return Ok(parse_quote! { #arg_expr.unwrap() });
        }
    }
    Ok(arg_expr)
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/context.rs` | Add `function_param_types` field |
| `crates/depyler-core/src/rust_gen.rs` | Populate parameter types during module processing |
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Use parameter types in `convert_call()` |

## Tests to Add

```python
def test_optional_to_function():
    @dataclass
    class Person:
        age: Optional[int]
    
    def process(age: int) -> int:
        return age + 10
    
    def use_person(p: Person) -> int:
        return process(p.age)  # Should generate: process(p.age.unwrap())
```

## Edge Cases

1. **Optional to Optional**: No unwrap needed when both arg and param are Optional
2. **Variable vs Field**: Handle Optional variables, not just fields
3. **Multiple arguments**: Handle functions with multiple arguments where only some need unwrapping
4. **Nested calls**: `process(other_func(p.age))` - need to handle intermediate types

## Acceptance Criteria

- [ ] `function_param_types` added to `CodeGenContext`
- [ ] Parameter types populated during module processing
- [ ] `convert_call()` checks arg/param type compatibility
- [ ] Optional arguments unwrapped when passed to non-Optional parameters
- [ ] No unwrap when both arg and param are Optional
- [ ] Unit tests passing
- [ ] Integration tests with end-to-end transpilation
