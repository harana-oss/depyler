# Depyler Outstanding Issues

## Quick Reference

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| 0346 | Classmethod parameter type inference | HIGH | **Fixed 2026-01-06** |
| 0347 | Walrus operator scope in compound conditions | HIGH | **Fixed 2026-01-05** |
| 0348 | Try-except generates unreachable code / Some() wrapping | HIGH | **Fixed 2026-01-05** |
| 0349 | Dict.get() double unwrap | MEDIUM | **Fixed 2026-01-05** |
| 0354 | ABC missing `impl Trait for Struct` blocks | HIGH | **Partially Fixed 2026-01-06** |
| 0356 | Async/await generates invalid Python API calls | MEDIUM | Open |
| 0359 | Function argument type inference defaults to serde_json::Value | HIGH | Open |

---

## HIGH Priority

### DEPYLER-0346: Classmethod Parameter Type Inference

**Files**: `tests/toml/class-methods.toml`

**Status**: ✅ **FIXED** (2026-01-06)

**Issue**: Classmethod parameters infer as `serde_json::Value` instead of concrete types.

**Example**:
```python
class Calculator:
    @classmethod
    def add(cls, a, b):
        return a + b
```

Generated (BEFORE):
```rust
pub fn add(a: serde_json::Value, b: serde_json::Value) {
    return a + b;
}
```

Generated (AFTER):
```rust
pub fn add(a: i32, b: i32) {
    return a + b;
}
```

**Root Cause**: When method parameters lacked type annotations, `ast_bridge.rs` would set them to `Type::Unknown` (line 948), which the TypeMapper would then convert to `serde_json::Value`. There was no type inference step for method parameters.

**Fix**: Modified `ast_bridge.rs` to add parameter type inference for class methods:
1. Added `infer_method_parameter_types()` function that uses the existing `TypeHintProvider` to analyze method bodies for usage patterns
2. TypeHintProvider detects numeric operations (like `a + b`), string methods, container operations, etc.
3. For simple concrete types (Int, Float, String, Bool), we accept any confidence level since these are common patterns
4. Applied to both regular methods and async methods after HIR conversion

**Limitation**: Parameters used only as pass-through arguments (e.g., passed to another function without direct operations) still default to `serde_json::Value`. This would require inter-procedural type inference to resolve.

**Files Modified**:
- `crates/depyler-core/src/ast_bridge.rs` (added type inference for method parameters, ~40 lines)
- `tests/toml/class-methods.toml` (updated 7 test expectations)

**Tests**: All 19 class-methods tests pass ✅

**Secondary Issue**: Class variables generate as `pub const` which can't be mutated. Need `AtomicI32`, `RwLock`, or `lazy_static` for true class variable mutation.

---

### DEPYLER-0347: Walrus Operator Scope

**Files**: `tests/toml/walrus-operator.toml`

**Status**: ✅ **FIXED** (2026-01-05)

**Issue**: Multiple walrus operators in compound conditions scope variables incorrectly:
```python
if (a := x * 2) > 5 and (b := y * 2) > 15:
    print(f"a={a}, b={b}")
```

Generated (BEFORE):
```rust
let _cse_temp_0 = ({
    let a = x * 2;  // ❌ scoped to block
    a
} > 5) && ({
    let b = y * 2;  // ❌ scoped to block  
    b
} > 15);
if _cse_temp_0 {
    log::info!("{}", format!("a={}, b={}", a, b));  // ❌ a, b not in scope
}
```

Generated (AFTER):
```rust
let a = x * 2;  // ✅ Hoisted before if
let b = y * 2;  // ✅ Hoisted before if
if (a > 5) && (b > 15) {
    log::info!("{}", format!("a={}, b={}", a, b));  // ✅ a, b accessible
}
```

**Root Cause**: The optimizer's Common Subexpression Elimination (CSE) was extracting if-conditions containing `NamedExpr` (walrus operators) into temporary variables before the `codegen_if_stmt` function could properly extract and hoist the walrus assignments. The CSE pass would convert walrus operators into block-scoped expressions `{ let x = expr; x }`, making the variables inaccessible outside those blocks.

**Fix**: Modified `optimizer.rs` to skip CSE extraction for expressions containing `NamedExpr`:
1. Added `contains_named_expr()` helper function that recursively checks if an expression tree contains any walrus operators
2. Updated `should_extract_for_cse()` to return `false` when the expression contains walrus operators, allowing the existing walrus extraction logic in `codegen_if_stmt()` (lines 1720-1731) to handle them correctly

**Files Modified**:
- `crates/depyler-core/src/optimizer.rs` (added `contains_named_expr()`, updated `should_extract_for_cse()`)

**Tests**: All 9 walrus operator tests pass ✅

**Note**: Walrus operators in list/generator comprehensions still have scoping issues - this is a separate problem requiring special handling in comprehension codegen. While loops with walrus operators work correctly, but have unrelated mutation tracking issues.

---

### DEPYLER-0348: Try-Except Safe Functions

**Files**: `tests/toml/result-types.toml`, `tests/toml/exceptions.toml`

**Status**: ✅ **FIXED** (2026-01-05)

**Issue 1**: Try-except with IndexError generates unreachable code:
```python
def safe_get(items: list[int], index: int) -> int:
    try:
        return items[index]
    except IndexError:
        return -1
```

Generated (BEFORE):
```rust
return items.get(index as usize).cloned().unwrap();  // Always returns or panics
return -1;  // ❌ Unreachable
```

Generated (AFTER):
```rust
return items.get(index as usize).cloned().unwrap_or(-1);  // ✅ Fixed
```

**Issue 2**: `safe_parse` wraps return in `Some()` when function returns `int`:
```python
def safe_parse(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return 0
```

Generated (BEFORE):
```rust
pub fn safe_parse(s: String) -> i32 {
    match s.parse::<i32>() {
        Ok(__parsed_value) => Some(__parsed_value),  // ❌ Returns Option<i32>, not i32
        Err(_) => Some(0),
    }
}
```

Generated (AFTER):
```rust
pub fn safe_parse(s: String) -> i32 {
    match s.parse::<i32>() {
        Ok(__parsed_value) => __parsed_value,  // ✅ Returns i32
        Err(_) => 0,
    }
}
```

**Root Cause**: 
1. IndexError: The `codegen_try_stmt` function had no special handling for `try { return items[index] } except IndexError { return default }` pattern, falling back to sequential statement generation which created unreachable code.
2. ValueError/Some(): The special case handler for `int()` parse was incorrectly wrapping return values in `Some()`, treating them as Option types when they shouldn't be.

**Fix**: 
1. Added special case handling in `codegen_try_stmt` (stmt_gen.rs ~4820-4838) to detect IndexError handlers and generate `.unwrap_or(default)` instead of sequential statements.
2. Removed `Some()` wrapping in `codegen_try_stmt` (stmt_gen.rs ~4653-4683) - now returns raw values directly in both Ok and Err branches.

**Files Modified**:
- `crates/depyler-core/src/rust_gen/stmt_gen.rs` (lines 4653-4683, 4820-4838)
- `tests/toml/result-types.toml` (2 test expectations updated)
- `tests/toml/exceptions.toml` (3 test expectations updated)

**Tests**: All 25 exception tests pass ✅, all 18 result-types tests pass ✅

---

### DEPYLER-0354: ABC Trait Generation

**Files**: `tests/toml/abc.toml`

**Status**: ✅ **PARTIALLY FIXED** (2026-01-06)

**Issue 1**: ABC classes don't generate `impl Trait for Struct` blocks:
```python
class Base(ABC):
    @abstractmethod
    def method(self) -> int:
        pass

class Concrete(Base):
    def method(self) -> int:
        return 42
```

Generated (BEFORE):
```rust
trait Base {
    fn method(&self) -> i32;
}
#[derive(Debug, Copy, Clone)]
pub struct Concrete {}
impl Concrete {
    pub fn new() -> Self {
        Self {}
    }
    pub fn method(&self) -> i32 {  // ❌ Method in wrong impl block
        return 42;
    }
}
// ❌ Missing: impl Base for Concrete { ... }
```

Generated (AFTER):
```rust
trait Base {
    fn method(&self) -> i32;
}
#[derive(Debug, Copy, Clone)]
pub struct Concrete {}
impl Concrete {
    pub fn new() -> Self {
        Self {}
    }
}
impl Base for Concrete {  // ✅ Trait implementation generated
    fn method(&self) -> i32 {
        return 42;
    }
}
```

**Root Cause**: The `convert_class_to_struct()` function in `direct_rules.rs` was generating all methods in `impl Struct` blocks, with no awareness of trait inheritance. When a class inherited from an ABC, it didn't generate the required `impl Trait for Struct` block.

**Fix**: Modified `convert_classes_to_rust()` in `rust_gen.rs` and `convert_class_to_struct()` in `direct_rules.rs`:
1. Build a map of ABC classes when processing all classes
2. Pass this map to `convert_class_to_struct()`
3. When a class inherits from an ABC, generate `impl Trait for Struct` blocks with methods that implement trait requirements
4. Skip trait methods when adding to `impl Struct` to avoid duplication

**Files Modified**:
- `crates/depyler-core/src/rust_gen.rs` (added ABC class map building)
- `crates/depyler-core/src/direct_rules.rs` (updated `convert_class_to_struct()` signature and logic, ~100 lines added)

**Tests**: Basic ABC inheritance now compiles and runs correctly ✅

**Issue 2**: Invalid `super()` calls:
```rust
return super().method() * 2;  // ❌ Not valid Rust
```

**Status**: ⚠️ **Known Limitation**

`super().method()` in Python gets converted to `self.method()` which causes infinite recursion in Rust when the method is overridden. Proper fix would require calling the trait's default implementation, but Rust doesn't have a direct equivalent to Python's `super()` for trait methods.

Possible solutions (not yet implemented):
- Wrapper methods that call trait defaults
- Manual code generation for trait delegation
- Use of explicit trait qualification `<Self as Trait>::method(self)`

This is a fundamental difference between Python's inheritance model and Rust's trait system.

**Issue 3**: `ABC.register()` generates undefined function calls.

**Status**: ⚠️ **Not Fixed** - ABC.register() is a runtime registration system that doesn't have a Rust equivalent. Would need compile-time trait bounds instead.

---

### DEPYLER-0359: Function Argument Type Inference

**Files**: `tests/toml/type-guards.toml`

**Issue**: Functions infer arguments as `&serde_json::Value` or `&object` instead of concrete types:
```python
def is_valid(x):
    return x > 0
```

Generates:
```rust
pub fn is_valid(x: &serde_json::Value) -> bool  // ❌ Should be i32
```

**Also**: `isinstance()` checks don't narrow types—always generate `if true`.

**Fix**: Improve type inference from usage context. Default to `i32` for numeric operations. Implement proper type narrowing for isinstance().

---

## MEDIUM Priority

### DEPYLER-0349: Dict.get() Double Unwrap

**Files**: `tests/toml/dictionaries.toml`, `tests/toml/collections.toml`

**Status**: ✅ **FIXED** (2026-01-05)

**Issue**: Extra `.unwrap()` call:
```python
return d.get("key", 0)
```

Generated (BEFORE):
```rust
return *d.get("key").unwrap_or(&0).unwrap();  // ❌ Double unwrap
```

Generated (AFTER):
```rust
return *d.get("key").unwrap_or(&0);  // ✅ Fixed
```

**Root Cause**: The `expr_is_optional` function in `stmt_gen.rs` (line 338-340) treated ALL `.get()` method calls as returning `Option`, even when called with a default value argument which makes the return type non-optional.

**Fix**: Modified `expr_is_optional` in `crates/depyler-core/src/rust_gen/stmt_gen.rs` to check the number of arguments:
- `dict.get(key)` → Returns `Option<V>` → needs unwrap
- `dict.get(key, default)` → Returns `V` → no unwrap needed

**Files Modified**:
- `crates/depyler-core/src/rust_gen/stmt_gen.rs` (lines 338-348)
- `tests/toml/dictionaries.toml` (4 test expectations updated)

**Tests**: All 58 dictionary tests pass ✅

---

### DEPYLER-0356: Async/Await Codegen

**Files**: `tests/toml/async-functions.toml`

**What works**: `pub async fn` signatures, `.await` expressions, async methods.

**Issues**:
- Generates `asyncio.run()` calls (Python API, doesn't exist in Rust)
- Generates `inspect.iscoroutinefunction()` calls (Python API)
- Module-level async code generates invalid `pub const` declarations
- Async closures sometimes lose return type information

**Fix**: Implement tokio runtime setup. Map `asyncio.run()` to `tokio::runtime::Runtime::block_on()`. Remove Python-specific API calls.

---

## Test Commands

```bash
# Run all TOML tests
cargo run -- test -j

# Run specific test file
cargo run -- test -p tests/toml/exceptions.toml

# Filter by name
cargo run -- test -f "classmethod"

# With compilation verification
cargo run -- test -c
```

---

## Architecture Note

Two code generation paths exist:
- `direct_rules.rs` — Older, used for classes
- `rust_gen/` — Newer, context-aware, used for functions

Several issues (0346, 0359) stem from `direct_rules.rs` lacking context that `rust_gen/` has. Long-term fix: migrate class generation to unified `rust_gen` path.