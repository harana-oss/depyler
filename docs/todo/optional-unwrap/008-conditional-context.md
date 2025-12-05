# 008: Optional in Conditional Context

**Priority**: Low  
**Status**: ✅ Implemented  
**Complexity**: Medium

## Problem

When Optional values are used in boolean/truthiness contexts (if statements, while conditions, etc.), they need special handling. Python treats `None` as falsy, while Rust's `Option<T>` isn't directly usable as a boolean.

## Current Behavior

```python
@dataclass
class Person:
    name: Optional[str]

def check_name(p: Person) -> bool:
    if p.name:  # p.name is Optional[str]
        return True
    return False
```

**Generated (Incorrect)**:
```rust
pub fn check_name(p: &Person) -> bool {
    if p.name {  // Error: expected `bool`, found `Option<String>`
        return true;
    }
    false
}
```

## Expected Behavior

For simple presence check:
```rust
pub fn check_name(p: &Person) -> bool {
    if p.name.is_some() {
        return true;
    }
    false
}
```

For strings (checking non-empty):
```rust
pub fn check_name(p: &Person) -> bool {
    if p.name.as_ref().is_some_and(|s| !s.is_empty()) {
        return true;
    }
    false
}
```

## Python Truthiness vs Rust

| Python Expression | True When | Rust Equivalent |
|------------------|-----------|-----------------|
| `if optional_val:` | Not None | `if opt.is_some()` |
| `if optional_str:` | Not None and not empty | `opt.as_ref().is_some_and(\|s\| !s.is_empty())` |
| `if optional_list:` | Not None and not empty | `opt.as_ref().is_some_and(\|v\| !v.is_empty())` |
| `if optional_int:` | Not None and not 0 | `opt.is_some_and(\|n\| n != 0)` |
| `if x is None:` | x is None | `if x.is_none()` |
| `if x is not None:` | x is not None | `if x.is_some()` |

## Implementation Location

**File: `crates/depyler-core/src/rust_gen/stmt_gen.rs`** (line 917-940)

The `apply_optional_truthiness()` function handles all Optional types in conditional contexts:

```rust
fn apply_optional_truthiness(field_type: &Type, cond_expr: syn::Expr) -> syn::Expr {
    match field_type {
        Type::Optional(inner) => match inner.as_ref() {
            Type::String => parse_quote! { #cond_expr.as_ref().is_some_and(|s| !s.is_empty()) },
            Type::List(_) | Type::Dict(_, _) | Type::Set(_) => {
                parse_quote! { #cond_expr.as_ref().is_some_and(|v| !v.is_empty()) }
            }
            Type::Int => parse_quote! { #cond_expr.is_some_and(|n| n != 0) },
            Type::Float => parse_quote! { #cond_expr.is_some_and(|n| n != 0.0) },
            _ => parse_quote! { #cond_expr.is_some() },
        },
        // ...
    }
}
```

This is called from `apply_truthiness_conversion()` which is used in `generate_if_block()`.

## Original Implementation Approach (Historical)

### 1. Detect Conditional Context

```rust
fn convert_condition(&mut self, cond: &HirExpr) -> Result<syn::Expr> {
    // Check for `is None` / `is not None` patterns
    if let HirExpr::Compare { left, ops, comparators } = cond {
        if ops.contains(&CmpOp::Is) || ops.contains(&CmpOp::IsNot) {
            return self.convert_none_check(left, ops, comparators);
        }
    }
    
    // Check if condition is Optional
    if self.expr_is_optional(cond) {
        let cond_expr = self.convert_expr(cond)?;
        let inner_type = self.get_optional_inner_type(cond);
        
        // String/List: check non-empty
        if matches!(inner_type, Some(Type::String) | Some(Type::List(_))) {
            return Ok(parse_quote! { #cond_expr.as_ref().is_some_and(|v| !v.is_empty()) });
        }
        
        // Int/Float: check non-zero
        if matches!(inner_type, Some(Type::Int) | Some(Type::Float)) {
            return Ok(parse_quote! { #cond_expr.is_some_and(|n| n != 0) });
        }
        
        // Default: just check presence
        return Ok(parse_quote! { #cond_expr.is_some() });
    }
    
    self.convert_expr(cond)
}
```

### 2. Handle `is None` / `is not None`

```rust
fn convert_none_check(&mut self, left: &HirExpr, ops: &[CmpOp], comparators: &[HirExpr]) -> Result<syn::Expr> {
    let left_expr = self.convert_expr(left)?;
    
    // x is None
    if ops.contains(&CmpOp::Is) {
        return Ok(parse_quote! { #left_expr.is_none() });
    }
    
    // x is not None
    if ops.contains(&CmpOp::IsNot) {
        return Ok(parse_quote! { #left_expr.is_some() });
    }
    
    // Fallback
    self.convert_expr(left)
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Add condition conversion for Optional |
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Handle `is None` comparisons |

## Tests to Add

```python
def test_optional_truthiness():
    @dataclass
    class Person:
        name: Optional[str]
    
    def check(p: Person) -> bool:
        if p.name:
            return True
        return False

def test_is_none():
    @dataclass
    class Person:
        age: Optional[int]
    
    def is_missing(p: Person) -> bool:
        return p.age is None

def test_is_not_none():
    @dataclass
    class Person:
        age: Optional[int]
    
    def has_age(p: Person) -> bool:
        return p.age is not None

def test_optional_string_truthiness():
    # Empty string should be falsy
    def check(s: Optional[str]) -> bool:
        if s:
            return True
        return False
```

## Edge Cases

1. **Empty string**: `""` is falsy in Python but `Some("")` is truthy in naive Rust check
2. **Zero**: `0` is falsy in Python
3. **Empty list**: `[]` is falsy in Python
4. **Combined conditions**: `if x and y:` where one is Optional
5. **Negation**: `if not optional_val:`

## Semantic Difference Warning

Python's truthiness is more nuanced than Rust's `is_some()`:

```python
name = ""
if name:  # False in Python (empty string)
    print("has name")
```

The implementation must decide whether to:
1. Match Python semantics exactly (more complex code generation)
2. Use simple `is_some()` (simpler but different semantics)

**Recommendation**: Match Python semantics for correctness.

## Acceptance Criteria

- [ ] Optional in `if` condition converted to `.is_some()` or equivalent
- [ ] `is None` converted to `.is_none()`
- [ ] `is not None` converted to `.is_some()`
- [ ] Empty string/list/zero handling matches Python truthiness
- [ ] Combined boolean expressions handled
- [ ] Unit tests passing
