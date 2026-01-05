# Depyler Outstanding Issues

## Quick Reference

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| 0346 | Classmethod parameter type inference | HIGH | Open |
| 0347 | Walrus operator scope in compound conditions | HIGH | Open |
| 0348 | Try-except generates unreachable code / Some() wrapping | HIGH | Open |
| 0349 | Dict.get() double unwrap | MEDIUM | **Fixed 2026-01-05** |
| 0354 | ABC missing `impl Trait for Struct` blocks | HIGH | Open |
| 0356 | Async/await generates invalid Python API calls | MEDIUM | Open |
| 0359 | Function argument type inference defaults to serde_json::Value | HIGH | Open |

---

## HIGH Priority

### DEPYLER-0346: Classmethod Parameter Type Inference

**Files**: `tests/toml/class-methods.toml`

**Issue**: Classmethod parameters infer as `serde_json::Value` instead of concrete types.

**Secondary Issue**: Class variables generate as `pub const` which can't be mutated. Need `AtomicI32`, `RwLock`, or `lazy_static` for true class variable mutation.

---

### DEPYLER-0347: Walrus Operator Scope

**Files**: `tests/toml/walrus-operator.toml`

**Issue**: Multiple walrus operators in compound conditions scope variables incorrectly:
```python
if (a := x * 2) > 5 and (b := y * 2) > 15:
    print(f"a={a}, b={b}")
```

Generates:
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

**Fix**: Hoist walrus-assigned variables BEFORE the condition block.

**Also**: Walrus in list/generator comprehensions scope variables incorrectly.

---

### DEPYLER-0348: Try-Except Safe Functions

**Files**: `tests/toml/result-types.toml`

**Issue 1**: Try-except with IndexError generates unreachable code:
```python
def safe_get(items: list[int], index: int) -> int:
    try:
        return items[index]
    except IndexError:
        return -1
```

Generates:
```rust
return items.get(index as usize).cloned().unwrap();  // Always returns or panics
return -1;  // ❌ Unreachable
```

**Issue 2**: `safe_parse` wraps return in `Some()` when function returns `int`:
```python
def safe_parse(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return 0
```

Generates:
```rust
pub fn safe_parse(s: String) -> i32 {
    match s.parse::<i32>() {
        Ok(__parsed_value) => Some(__parsed_value),  // ❌ Returns Option<i32>, not i32
        Err(_) => Some(0),
    }
}
```

**Fix**: Use `.get()` with proper fallback for IndexError. Return values directly, not wrapped in Some().

---

### DEPYLER-0354: ABC Trait Generation

**Files**: `tests/toml/abc.toml`

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

Generates struct and trait separately but missing:
```rust
impl Base for Concrete { ... }  // ❌ Never generated
```

**Issue 2**: Invalid `super()` calls:
```rust
return super().method() * 2;  // ❌ Not valid Rust
```

**Issue 3**: `ABC.register()` generates undefined function calls.

**Fix**: Generate proper `impl Trait for Struct` blocks. Map `super()` to trait default implementations.

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