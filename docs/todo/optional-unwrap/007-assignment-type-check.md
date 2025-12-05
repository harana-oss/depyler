# 007: Assignment from Optional to Non-Optional

**Priority**: Low  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

When assigning an Optional field/variable to a variable with a non-Optional type annotation, the transpiler should automatically unwrap the value.

## Current Behavior

```python
@dataclass
class Person:
    age: Optional[int]

def extract_age(p: Person) -> None:
    age: int = p.age  # p.age is Optional[int], age is int
```

**Generated (Incorrect)**:
```rust
pub fn extract_age(p: &Person) {
    let age: i32 = p.age;  // Error: expected i32, found Option<i32>
}
```

## Expected Behavior

```rust
pub fn extract_age(p: &Person) {
    let age: i32 = p.age.unwrap();
}
```

## Implementation Approach

### 1. Check Type Annotation During Assignment

**File: `crates/depyler-core/src/rust_gen/stmt_gen.rs`**

In `convert_assign()` or `convert_ann_assign()`:

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/stmt_gen.rs`** (line 2636-2640)

In `codegen_assign_stmt()`, auto-unwrap logic for annotated assignments:

```rust
// Auto-unwrap Optional values when assigning to non-Optional annotated variables
let value_is_optional = expr_is_optional(value, ctx);
let target_is_optional = matches!(actual_type, Type::Optional(_));
if value_is_optional && !target_is_optional {
    value_expr = parse_quote! { #value_expr.unwrap() };
}
```

## Type Inference for Unannotated Assignments

For assignments without explicit type annotations, the current behavior preserves Optional:

```python
def process(p: Person) -> int:
    age = p.age  # Kept as Optional[int]
    return age * 2  # Binary op auto-unwraps
```

The auto-unwrap in binary operations (implemented separately) handles the use case.
- **Unwrap**: Simpler generated code, fails fast

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Modify `convert_ann_assign()` and `convert_assign()` |

## Tests to Add

```python
def test_assign_optional_to_int():
    @dataclass
    class Person:
        age: Optional[int]
    
    def extract(p: Person) -> None:
        age: int = p.age

def test_assign_optional_to_str():
    @dataclass
    class Person:
        name: Optional[str]
    
    def extract(p: Person) -> None:
        name: str = p.name

def test_assign_optional_to_optional():
    @dataclass
    class Person:
        age: Optional[int]
    
    def copy(p: Person) -> None:
        age: Optional[int] = p.age  # No unwrap needed
```

## Edge Cases

1. **No annotation**: `x = p.optional_field` - should infer type
2. **Nested Optional**: `Optional[Optional[int]]` to `Optional[int]`
3. **Multiple assignment**: `a, b = p.optional_tuple`
4. **With default**: `age: int = p.age or 0` - handled by 009

## Type Compatibility Matrix

| RHS Type | LHS Annotation | Action |
|----------|---------------|--------|
| `Optional[T]` | `T` | Unwrap |
| `Optional[T]` | `Optional[T]` | No change |
| `T` | `T` | No change |
| `T` | `Optional[T]` | Wrap in `Some()` |

## Acceptance Criteria

- [ ] Annotated assignments check type compatibility
- [ ] Optional unwrapped when assigning to non-Optional annotated variable
- [ ] No unwrap when both sides are Optional
- [ ] Handle various base types (int, str, List, etc.)
- [ ] Unit tests passing
