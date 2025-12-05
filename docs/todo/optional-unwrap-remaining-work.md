# Optional Type Auto-Unwrap: Remaining Work

**Date**: December 5, 2025  
**Branch**: `math-fixes`  
**Status**: Partially Complete

## Summary

This document outlines the remaining work needed to fully implement Python-like behavior for `Optional` types in the Depyler transpiler. In Python, you can access Optional fields directly without explicit unwrapping - operations on `None` values fail at runtime. The Rust transpilation should mimic this by adding `.unwrap()` automatically in appropriate contexts.

## Completed Work

### 1. Binary Operations (✅ Complete)

Optional fields are now automatically unwrapped when used as operands in binary operations.

**Location**: `crates/depyler-core/src/rust_gen/expr_gen.rs`

**Changes Made**:
- Added `field_is_optional()` helper (lines 11467-11497)
- Added `convert_with_optional_unwrap()` helper (lines 11499-11513)  
- Modified `convert_binary()` to use the new helper (lines 137-142)

**Example**:
```python
@dataclass
class Person:
    age: Optional[int]

def double_age(p: Person) -> int:
    return p.age * 2  # Now generates: p.age.unwrap() * 2
```

### 2. Return Statements (✅ Already Working)

The return statement handler in `stmt_gen.rs` already correctly handles:
- Unwrapping Optional variables when returning from non-Optional functions
- NOT unwrapping when returning Optional from Optional functions

**Location**: `crates/depyler-core/src/rust_gen/stmt_gen.rs` (lines 570-620)

---

## Remaining Work

### 1. Function Arguments (🔴 Not Implemented)

**Problem**: When an Optional field is passed as an argument to a function that expects the inner type (not Optional), the transpiler should automatically add `.unwrap()`.

**Current Behavior**:
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

**Expected**:
```rust
pub fn use_person(person: &Person) -> i32 {
    return process_age(person.age.unwrap());  // Correct
}
```

**Implementation Approach**:

1. **Track Function Parameter Types**: The `CodeGenContext` needs access to parameter type information for called functions. Currently available:
   - `function_return_types: HashMap<String, Type>` ✅
   - `function_param_borrows: HashMap<String, Vec<bool>>` ✅
   - `function_param_muts: HashMap<String, Vec<bool>>` ✅
   - `function_param_names: HashMap<String, Vec<String>>` ✅
   - **Missing**: `function_param_types: HashMap<String, Vec<Type>>` ❌

2. **Add Parameter Types to Context**:
   ```rust
   // In CodeGenContext (context.rs)
   pub function_param_types: HashMap<String, Vec<Type>>,
   ```

3. **Populate During Function Processing**:
   In `rust_gen.rs` during module processing, collect parameter types for each function.

4. **Modify Argument Conversion**:
   In `expr_gen.rs`, when converting function call arguments:
   ```rust
   fn convert_call_argument(&mut self, arg: &HirExpr, param_type: Option<&Type>) -> Result<syn::Expr> {
       let arg_expr = arg.to_rust_expr(self.ctx)?;
       
       // Check if argument is Optional field and parameter expects inner type
       if let HirExpr::Attribute { value, attr } = arg {
           if self.field_is_optional(value, attr) {
               if let Some(Type::Optional(_)) = param_type {
                   // Parameter also Optional, no unwrap needed
                   return Ok(arg_expr);
               }
               // Parameter expects inner type, unwrap
               return Ok(parse_quote! { #arg_expr.unwrap() });
           }
       }
       Ok(arg_expr)
   }
   ```

5. **Files to Modify**:
   - `crates/depyler-core/src/rust_gen/context.rs` - Add `function_param_types`
   - `crates/depyler-core/src/rust_gen.rs` - Populate parameter types during processing
   - `crates/depyler-core/src/rust_gen/expr_gen.rs` - Use parameter types in `convert_call()`

**Complexity**: Medium-High (requires threading type information through call sites)

---

### 2. Method Calls on Optional Fields (🟡 Partial)

**Problem**: When calling methods on Optional fields, the transpiler should unwrap first.

**Example**:
```python
@dataclass
class Container:
    items: Optional[List[int]]

def count_items(c: Container) -> int:
    return len(c.items)  # c.items is Optional[List[int]]
```

**Current**: May generate incorrect code depending on context.

**Implementation**: Extend `convert_with_optional_unwrap` to handle method receiver expressions.

---

### 3. Index Operations on Optional Fields (🔴 Not Implemented)

**Problem**: Indexing into Optional collections should unwrap first.

**Example**:
```python
@dataclass  
class Data:
    values: Optional[List[int]]

def get_first(d: Data) -> int:
    return d.values[0]  # d.values is Optional[List[int]]
```

**Expected**:
```rust
return d.values.unwrap()[0];
```

**Implementation**: Modify index expression handling in `expr_gen.rs` to check if base is Optional field.

---

### 4. Comparison with Non-Optional Values (🟡 Partial - via binary ops)

Comparisons are handled by binary operations, but special cases may exist:
- `is None` / `is not None` checks
- Comparisons in conditional contexts

---

### 5. Assignment from Optional to Non-Optional (🔴 Not Implemented)

**Problem**: When assigning an Optional field to a variable with non-Optional type annotation.

**Example**:
```python
def extract_age(p: Person) -> None:
    age: int = p.age  # Should unwrap
```

**Implementation**: Check type annotation during assignment statement generation.

---

## Python Patterns Requiring Special Handling

### Pattern: Guard Clauses

Python code often uses early returns to narrow Optional types:

```python
def process(p: Person) -> int:
    if p.age is None:
        return 0
    # After this point, p.age is effectively non-None
    return p.age * 2  # Type narrowing - should still unwrap
```

**Challenge**: The transpiler doesn't perform control flow-based type narrowing.

### Pattern: Truthiness Checks

```python
def process(p: Person) -> str:
    if p.name:  # Optional[str] used as bool
        return p.name.upper()  # name is "narrowed" here
    return "Unknown"
```

### Pattern: Walrus Operator

```python
def process(d: dict) -> str:
    if (val := d.get("key")) is not None:
        return val.upper()  # val is narrowed to non-None
    return "default"
```

### Pattern: or-Chaining for Defaults

```python
def get_display_name(p: Person) -> str:
    return p.nickname or p.name or "Anonymous"  # All are Optional[str]
```

**Rust equivalent**:
```rust
p.nickname.clone()
    .or_else(|| p.name.clone())
    .unwrap_or_else(|| "Anonymous".to_string())
```

### Pattern: Optional with Any/All

```python
def all_present(people: List[Person]) -> bool:
    return all(p.age is not None for p in people)
```

### Pattern: Filter None from Collections

```python
def get_ages(people: List[Person]) -> List[int]:
    return [p.age for p in people if p.age is not None]  # Filter + unwrap
```

**Rust equivalent**:
```rust
people.iter()
    .filter_map(|p| p.age)
    .collect()
```

### Pattern: Optional Callable

```python
from typing import Optional, Callable

def apply_if_present(value: int, func: Optional[Callable[[int], int]]) -> int:
    if func is not None:
        return func(value)
    return value
```

---

## Technical Architecture Analysis

### Type Information Flow

The current architecture has several sources of type information:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Type Information Sources                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  HIR Level:                                                      │
│  ├── HirParam.ty: Type          # Parameter types in functions   │
│  ├── HirFunc.ret_type: Type     # Function return types          │
│  └── HirField.ty: Type          # Class field types              │
│                                                                  │
│  CodeGenContext:                                                 │
│  ├── var_types: HashMap<String, Type>                            │
│  ├── class_field_types: HashMap<String, HashMap<String, Type>>   │
│  ├── function_return_types: HashMap<String, Type>                │
│  ├── function_param_borrows: HashMap<String, Vec<bool>>          │
│  ├── function_param_muts: HashMap<String, Vec<bool>>             │
│  └── function_param_names: HashMap<String, Vec<String>>          │
│      └── MISSING: function_param_types                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Current Limitations

1. **No Expression-Level Type Tracking**: The HIR doesn't track inferred types for arbitrary expressions, only for declared variables and fields.

2. **Late Type Resolution**: Type information is gathered during code generation, not in a separate analysis pass.

3. **Incomplete Parameter Type Propagation**: `function_param_types` is missing from `CodeGenContext`, making it impossible to check argument/parameter type compatibility at call sites.

### Recommended Implementation Order

```
Phase 1: Foundation (function arguments)
├── Add function_param_types to CodeGenContext
├── Populate during module processing
├── Modify convert_call() to check arg/param compatibility
└── Add tests for basic Optional → non-Optional argument passing

Phase 2: Collections (indexing)
├── Modify convert_index() to detect Optional base
├── Handle Optional[List[T]], Optional[Dict[K,V]]
└── Add tests for indexed access on Optional collections

Phase 3: Methods (method calls)
├── Extend expr_is_optional() for method receivers
├── Handle Optional receivers before method dispatch
└── Add tests for method calls on Optional values

Phase 4: Complex Patterns
├── Chained attribute access (a.b.c where b is Optional)
├── F-string interpolation
├── Comprehension iterables
└── Edge cases and error handling
```

---

## Additional Considerations & Edge Cases

### 6. Chained Optional Access (🔴 Not Implemented)

**Problem**: Accessing fields through multiple levels of Optional types requires careful unwrap chaining.

**Example**:
```python
@dataclass
class Inner:
    value: int

@dataclass 
class Outer:
    inner: Optional[Inner]

def get_value(o: Outer) -> int:
    return o.inner.value  # inner is Optional[Inner]
```

**Expected**:
```rust
pub fn get_value(o: &Outer) -> i32 {
    return o.inner.as_ref().unwrap().value;
}
```

**Complexity**: High - requires traversing the attribute chain and determining unwrap points.

---

### 7. Optional in Unary Operations (🔴 Not Implemented)

**Problem**: Unary operators (`-`, `not`, `~`) on Optional fields need unwrapping.

**Example**:
```python
@dataclass
class Numbers:
    count: Optional[int]

def negate_count(n: Numbers) -> int:
    return -n.count
```

**Expected**:
```rust
return -n.count.unwrap();
```

---

### 8. Optional in Augmented Assignment (🔴 Not Implemented)

**Problem**: Augmented assignments (`+=`, `-=`, etc.) with Optional fields.

**Example**:
```python
@dataclass
class Counter:
    value: Optional[int]

def increment(c: Counter) -> None:
    c.value += 1  # c.value is Optional[int]
```

**Current Issue**: Need to unwrap for read, then potentially re-wrap for write if the field should remain Optional.

---

### 9. Optional Variables (Not Just Fields) (🟡 Partial)

The `expr_is_optional()` function in `expr_gen.rs` (line 11807) handles variable-level Optional types, but edge cases remain:

**Example**:
```python
def process(items: Optional[List[int]]) -> int:
    return len(items)  # items is Optional[List[int]]
```

**Current behavior**: `expr_is_optional` checks `var_types`, but this may not be populated for all parameters.

---

### 10. Optional in Conditional Context (🔴 Not Implemented)

**Problem**: When Optional values are used in truthiness checks without explicit `is None`.

**Example**:
```python
def check(p: Person) -> bool:
    if p.name:  # name is Optional[str]
        return True
    return False
```

**Expected**: Should use `.is_some()` or `.is_some_and(|s| !s.is_empty())` for strings.

---

### 11. Optional to Optional Parameter Passing (🟡 Edge Case)

**Problem**: When passing Optional to a function that expects Optional, no unwrap needed, but type compatibility must be verified.

**Example**:
```python
def store_age(age: Optional[int]) -> None:
    pass

def use_person(p: Person) -> None:
    store_age(p.age)  # Both are Optional[int] - no unwrap needed
```

---

### 12. Optional with Default Values (🔴 Not Implemented)

**Problem**: Python's `or` pattern for default values with Optional.

**Example**:
```python
def get_name(p: Person) -> str:
    return p.name or "Unknown"  # p.name is Optional[str]
```

**Expected**:
```rust
return p.name.clone().unwrap_or_else(|| "Unknown".to_string());
```

---

### 13. Method Chaining on Optional Results (🔴 Not Implemented)

**Problem**: When a method returns Optional and another method is called on the result.

**Example**:
```python
def find_and_upper(d: dict, key: str) -> str:
    return d.get(key).upper()  # .get() returns Optional
```

**Current**: `convert_method_call` has partial handling for `get()` (line 9780), but chaining is not fully supported.

---

### 14. Optional in String Formatting (🔴 Not Implemented)

**Problem**: F-strings with Optional fields.

**Example**:
```python
def format_person(p: Person) -> str:
    return f"Name: {p.name}"  # p.name is Optional[str]
```

**Expected**: Should handle None gracefully or unwrap with format display.

---

### 15. Generic Functions with Optional (🔴 Not Implemented)

**Problem**: When Optional is used with generic type parameters.

**Example**:
```python
from typing import TypeVar

T = TypeVar('T')

def unwrap_or(value: Optional[T], default: T) -> T:
    if value is not None:
        return value
    return default
```

---

### 16. Optional in Collection Operations (🔴 Not Implemented)

**Problem**: When Optional collections are used in operations like `extend`, `update`, etc.

**Example**:
```python
@dataclass
class Container:
    items: Optional[List[int]]
    
def extend_items(c: Container, more: List[int]) -> None:
    c.items.extend(more)  # items is Optional[List[int]]
```

---

### 17. Reference Parameter Unwrapping (✅ Handled)

The `stmt_gen.rs` already handles reference parameters specially (line 584-593):

```rust
if is_ref_param {
    // For &Option<T> parameters, use as_ref().unwrap().clone()
    expr_tokens = parse_quote! { #expr_tokens.as_ref().unwrap().clone() };
}
```

---

### 18. Double-Unwrap Prevention (✅ Handled)

The code already prevents double-wrapping in Some() (line 611):

```rust
let expr_already_optional = expr_is_optional(e, ctx);
if expr_already_optional {
    // Expression is already Option<T>, don't wrap in Some()
}
```

Similar logic needed for unwrapping to prevent double unwrap.

---

## Implementation Notes

### Current Helper Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `field_is_optional()` | `expr_gen.rs:11469` | Check if attribute access is Optional |
| `expr_is_optional()` | `expr_gen.rs:11807` | Check if any expression is Optional |
| `expr_is_optional()` | `stmt_gen.rs:222` | Duplicate for statement context |
| `convert_with_optional_unwrap()` | `expr_gen.rs:11504` | Auto-unwrap for binary ops |

### Suggested Helper Functions to Add

1. **`unwrap_optional_if_needed(expr, target_type)`**: Unified unwrap logic that checks if target expects non-Optional.

2. **`is_deeply_optional(expr)`**: For chained access, determine if any link in the chain is Optional.

3. **`get_expr_type(expr)`**: Full type inference for expressions (currently partial).

---

## Safety Considerations

### Panic vs. Graceful Handling

Using `.unwrap()` mirrors Python's runtime error behavior, but consider:

1. **`.expect("message")`**: Better error messages in production
2. **`.unwrap_or_default()`**: Silent recovery (changes semantics)
3. **`match` expressions**: Explicit handling with custom logic
4. **Annotation-based control**: Let users choose via `#[depyler(safe_unwrap)]`

### Alternative: Generate `Option<T>` Propagation

Instead of unwrapping, could generate code that propagates Options:

```rust
// Instead of: p.age.unwrap() * 2
// Generate:   p.age.map(|age| age * 2)
```

This maintains Rust's safety but changes return type to `Option<T>`.

---

## Testing Strategy

### Unit Tests to Add

1. **Optional field in arithmetic** (✅ Covered):
   ```python
   def test_optional_arithmetic():
       # person.age * 2, person.age + 5, etc.
   ```

2. **Optional field as function argument** (❌ Needs test):
   ```python
   def test_optional_to_function():
       # process(person.optional_field)
   ```

3. **Optional field indexing** (❌ Needs test):
   ```python
   def test_optional_indexing():
       # container.optional_list[0]
   ```

4. **Nested Optional access** (❌ Needs test):
   ```python
   def test_nested_optional():
       # obj.optional_inner.optional_field
   ```

5. **Optional in comprehensions** (❌ Needs test):
   ```python
   def test_optional_in_comprehension():
       # [x for x in obj.optional_list]
   ```

6. **Chained method calls on Optional** (❌ Needs test):
   ```python
   def test_chained_optional():
       # d.get(key).upper()
   ```

7. **Optional in f-strings** (❌ Needs test):
   ```python
   def test_optional_fstring():
       # f"{person.optional_name}"
   ```

8. **Optional with default (or pattern)** (❌ Needs test):
   ```python
   def test_optional_or_default():
       # person.name or "default"
   ```

9. **Unary operations on Optional** (❌ Needs test):
   ```python
   def test_optional_unary():
       # -person.optional_count, not person.optional_flag
   ```

10. **Augmented assignment with Optional** (❌ Needs test):
    ```python
    def test_optional_augmented():
        # counter.optional_value += 1
    ```

### Integration Tests

Add to `tests/test_clone.rs` or create `tests/test_optional_unwrap.rs`:
- End-to-end transpilation tests
- Rust compilation verification
- Runtime behavior verification

---

## Priority Order

1. **High Priority**: Function arguments - most common use case
2. **High Priority**: Index operations - common for Optional collections
3. **Medium Priority**: Method calls on Optional fields
4. **Medium Priority**: Chained Optional access
5. **Medium Priority**: Optional in f-strings
6. **Low Priority**: Assignment type checking
7. **Low Priority**: Unary operations
8. **Low Priority**: Edge cases (comprehensions, generics)

---

## Alternative Approaches Considered

### 1. Always Unwrap on Field Access
**Rejected**: Breaks when returning Optional from Optional function.

### 2. Use `.unwrap_or_default()`
**Consideration**: Safer but changes semantics - Python would raise, this silently defaults.

### 3. Generate `match` Expressions
**Consideration**: More verbose but explicit error handling. Could be opt-in via annotation.

### 4. Full Type Flow Analysis
**Consideration**: Build complete type inference pass before codegen. Higher complexity but more accurate.

### 5. Use `?` Operator with Result Conversion  
**Consideration**: Convert `Option<T>` to `Result<T, NoneError>` and use `?`. Requires custom error type.

---

## Related Files

| File | Purpose |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Expression generation, binary ops |
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Statement generation, returns |
| `crates/depyler-core/src/rust_gen/context.rs` | Code generation context |
| `crates/depyler-core/src/rust_gen/func_gen.rs` | Function generation |
| `crates/depyler-core/src/hir.rs` | HIR types including `Type::Optional` |
| `tests/test_clone.rs` | Tests for clone and Optional behavior |

---

## References

- Python typing: https://docs.python.org/3/library/typing.html#typing.Optional
- Rust Option: https://doc.rust-lang.org/std/option/enum.Option.html
- `FunctionSignatureRegistry` in `crates/depyler-core/src/interprocedural/signature_registry.rs`
