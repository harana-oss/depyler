# Depyler Transpilation Issues

This document catalogs the **3,207 test failures** found in the `failures/` directory, grouped by category and root cause.

*Last updated: 2026-01-09 (Pass 9)*

## Summary Statistics

| Category | Failure Count | Priority |
|----------|--------------|----------|
| Standard Library (examples-stdlib) | 72 | High |
| Control Flow | 55 | High |
| String Methods | 51 | Medium |
| Error Handling | 50 | High |
| Type Inference | 47 | Critical |
| Optional Types | 46 | Critical |
| Set Operations | 40 | Medium |
| Dictionaries | 37 | High |
| Functions | 32 | High |
| Deque | 31 | Medium |
| Classes | 31 | High |
| Operators | 30 | Medium |
| F-Strings | 28 | Medium |
| Dataclasses (all) | 101 | Medium |
| Counter | 28 | Low |
| ChainMap | 27 | Low |
| Bytes/ByteStrings | 51 | Medium |
| AsyncIO Tasks | 26 | Medium |
| Enums (all) | 71 | High |
| Dunder Methods (all) | ~200 | High |
| Complex Numbers | 24 | Low |
| Collections | 24 | Medium |
| Global/Nonlocal | 19 | High |
| Template Strings | 22 | Medium |
| Binary Literals | 22 | Medium |
| Functools (all) | 66 | Medium |
| Slots | 17 | Medium |
| Protocols | 11 | High |
| Descriptors | 17 | Low |
| Concurrency | 15 | Medium |
| Performance Patterns | 16 | Medium |
| Verification Contracts | 14 | Medium |
| Positional-Only Args | 14 | High |
| Async Generators | 22 | High |
| Walrus Operator | 3 | Medium |
| Generators/Liveness | 10 | High |
| Import Deduplication | 5 | Low |
| Multi-Module | 12 | Medium |
| Resource Management | 10 | Medium |
| Argparse/CLI | 25 | Medium |

**Total Failures: 3,207**
**Total Issues Documented: 189**

---

## Critical Issues

### Issue 1: Type Inference Falls Back to `serde_json::Value`

**Affected Categories:** type-inference, basic-types, lambdas, dunder-*, many others  
**Failure Count:** ~500+

**Problem:** When type information cannot be inferred, the transpiler defaults to `serde_json::Value` instead of generating proper Rust types.

**Examples:**
```rust
// Expected:
pub fn lambda_handler(event: HashMap<String, i32>, context: HashMap<String, i32>) -> HashMap<String, i32>

// Actual:
pub fn lambda_handler(event: HashMap<serde_json::Value, serde_json::Value>, context: HashMap<serde_json::Value, serde_json::Value>) -> HashMap<serde_json::Value, serde_json::Value>
```

**Root Cause:** Type inference system doesn't propagate types from:
- Function return type hints
- Dict/list literal contents
- Method call return types

**Fix:** Enhance type inference to:
1. Use Python type hints when available
2. Infer from literal values in collections
3. Track type flow through assignments

---

### Issue 2: Module-Level Statements Generate Invalid Code

**Affected Categories:** fstrings, dataclasses-basic, enum-basic, many examples  
**Failure Count:** ~300+

**Problem:** Module-level Python statements are incorrectly transpiled as `pub const` with incorrect types. Additionally, `fn main()` is sometimes generated when the Python code has no `main()` function.

**Examples:**
```rust
// Expected (module-level code without main):
let x = 42;
let result = format!("Value: {}", x);

// Actual:
pub const x: i32 = 42;
pub const result: serde_json::Value = format!("Value: {}", x);
```

**Root Cause:** Module-level code detection and wrapping logic is incomplete.

**Fix:** 
1. Only generate `fn main()` when Python code explicitly defines a `main()` function
2. Use `let` bindings instead of `const` for runtime values
3. Never use `serde_json::Value` as const type

---

### Issue 3: ABC/Protocol/Trait Transpilation

**Affected Categories:** abc, protocols, multiple-inheritance  
**Failure Count:** ~100+

**Problem:** Abstract base classes and protocols are transpiled as structs instead of traits.

**Examples:**
```rust
// Expected:
trait MyABC {}

// Actual:
pub struct MyABC {}
```

**Root Cause:** Missing detection and handling for:
- `ABC` base class
- `@abstractmethod` decorator
- `Protocol` classes
- Trait-like inheritance patterns

**Fix:**
1. Detect `ABC` inheritance → generate `trait`
2. Handle `@abstractmethod` → generate trait method signatures
3. Map `Protocol` to Rust traits with `dyn` support

---

### Issue 4: Ownership/Borrowing Analysis

**Affected Categories:** ownership, lifetimes, parameter-borrowing  
**Failure Count:** ~80+

**Problem:** Incorrect ownership analysis leads to missing `.clone()` calls or wrong borrow types.

**Examples:**
```rust
// Expected:
pub fn use_string_twice(s: String) -> String {
    let a = s.clone();
    let b = s;
    return format!("{}{}", a, b);
}

// Actual:
pub fn use_string_twice(s: String) -> String {
    let a = s;  // Missing clone!
    let b = s;  // Use after move
    return format!("{}{}", a, b);
}
```

**Root Cause:** Use-after-move analysis doesn't track:
- Multiple uses of the same variable
- Variable usage in expressions vs. moves

**Fix:** Implement liveness analysis that:
1. Tracks all uses of each variable
2. Inserts `.clone()` when a value is used multiple times before its last use
3. Only moves on final use

---

### Issue 5: Decorator Transpilation

**Affected Categories:** decorators-basic, decorators-class, decorators-with-args  
**Failure Count:** ~80+

**Problem:** Decorators are not properly expanded, resulting in incorrect function signatures.

**Examples:**
```rust
// Expected:
fn identity<F>(f: F) -> F where F: Fn() -> String { f }
fn greet_impl() -> String { "hello".to_string() }
let greet = identity(greet_impl);

// Actual:
pub fn identity(f: serde_json::Value) { return f; }
pub fn greet() -> String { return "hello".to_string(); }
```

**Root Cause:** Decorator handling doesn't:
- Preserve function type signatures through decoration
- Generate proper generic wrapper types
- Handle decorator arguments

**Fix:**
1. Implement decorator expansion during AST transformation
2. Generate proper `Fn`/`FnMut`/`FnOnce` trait bounds
3. Handle common decorators (`@staticmethod`, `@classmethod`, `@property`)

---

## High Priority Issues

### Issue 6: Enum Transpilation

**Affected Categories:** enum-basic, enum-advanced, enum-methods, enum-int  
**Failure Count:** ~95+

**Problem:** Python enums are transpiled as structs with constants instead of proper Rust enums.

**Examples:**
```rust
// Expected:
#[derive(Debug, PartialEq)]
enum Color {
    RED = 1,
    GREEN = 2,
}

// Actual:
pub struct Color {}
impl Color {
    pub const RED: i32 = 1;
    pub const GREEN: i32 = 2;
}
```

**Fix:**
1. Detect `Enum` inheritance
2. Generate proper `enum` with values
3. Handle `IntEnum`, `Flag`, `auto()` patterns

---

### Issue 7: Async/Await Transpilation

**Affected Categories:** async-functions, async-for, async-generators, asyncio-tasks, await-expressions  
**Failure Count:** ~150+

**Problem:** Async code generation is incomplete, missing main loop setup and proper awaiting.

**Examples:**
```rust
// Expected:
pub async fn f() -> i32 { return 42; }
fn main() {
    let coro = f();
    let result = futures::executor::block_on(coro);
}

// Actual:
pub async fn f() -> i32 { return 42; }
// Missing runtime setup and await
```

**Fix:**
1. Generate proper async runtime setup
2. Handle `await` expressions correctly
3. Transpile `asyncio.run()` to appropriate executor

---

### Issue 8: Context Manager (`with` statement)

**Affected Categories:** context-managers, nested-context-managers, async-with  
**Failure Count:** ~60+

**Problem:** Context managers don't generate proper `Drop` trait implementation or RAII patterns.

**Examples:**
```rust
// Expected:
impl Drop for File {
    fn drop(&mut self) { self.closed = true; }
}
{
    let f = File::new("data.txt");
    // auto-closes when scope ends
}

// Actual:
pub fn __enter__(&self) -> &Self { ... }  // Python-style, not Rust-idiomatic
```

**Fix:**
1. Implement `Drop` trait for context managers
2. Use scope-based resource management
3. Handle `__enter__`/`__exit__` conversion to RAII

---

### Issue 9: Exception Handling → Result Types

**Affected Categories:** error-handling, exceptions, result-types, result-error-propagation  
**Failure Count:** ~100+

**Problem:** Try/except doesn't consistently map to `Result<T, E>` types.

**Issues observed:**
- Missing `?` operator usage
- Inconsistent error type wrapping
- `panic!` used instead of `Result::Err`

**Fix:**
1. Consistent `Result<T, E>` return type generation
2. Proper error type hierarchy
3. Use `?` operator for error propagation

---

### Issue 10: Dict/HashMap Key-Value Type Inference

**Affected Categories:** dictionaries, comprehensions, modules-json  
**Failure Count:** ~100+

**Problem:** Dictionary key/value types default to `serde_json::Value`.

**Examples:**
```rust
// Expected:
pub fn make_dict_comp() -> HashMap<String, i32>

// Actual:
pub fn make_dict_comp() -> HashMap<serde_json::Value, i32>
```

**Fix:** Infer key/value types from:
1. Type annotations
2. Dictionary literals
3. Comprehension expressions

---

## Medium Priority Issues

### Issue 11: String Method Mapping

**Affected Categories:** string-methods-all, string-operations  
**Failure Count:** ~60+

**Problem:** Some Python string methods don't map correctly to Rust equivalents.

**Fix:** Complete the string method mapping table.

---

### Issue 12: Multiple Inheritance

**Affected Categories:** multiple-inheritance  
**Failure Count:** ~20+

**Problem:** Diamond inheritance and MRO not handled.

**Fix:** Use composition or trait objects to simulate multiple inheritance.

---

### Issue 13: Metaclasses

**Affected Categories:** metaclasses  
**Failure Count:** ~15+

**Problem:** Metaclasses have no direct Rust equivalent.

**Fix:** 
1. Document as unsupported feature
2. Use derive macros for common patterns
3. Generate compile-time alternatives where possible

---

### Issue 14: Generator State Machine

**Affected Categories:** generators, generator-liveness  
**Failure Count:** ~7+

**Problem:** Generator state variables not always initialized correctly.

**Examples:**
```rust
// Expected:
0 => {
    self.x = 1;
    self.state = 1usize;
    return Some(self.x);
}

// Actual:
0 => {
    // Missing: self.x = 1;
    self.state = 1usize;
    return Some(self.x);
}
```

**Fix:** Ensure all state variables are initialized before use.

---

### Issue 15: Walrus Operator (`:=`)

**Affected Categories:** walrus-operator  
**Failure Count:** ~3

**Problem:** Assignment expressions not transpiled.

**Fix:** Convert `:=` to let binding before condition.

---

### Issue 16: Type Cast Redundancy

**Affected Categories:** lifetimes, cast-expressions  
**Failure Count:** ~20+

**Problem:** Redundant type casts generated.

**Examples:**
```rust
// Expected:
return s.len() as i32;

// Actual:
return s.len() as i32 as i32;  // Double cast
```

**Fix:** Remove duplicate casts during code generation.

---

### Issue 17: Standard Library Module Mapping

**Affected Categories:** examples-stdlib, imports-basic, modules-*  
**Failure Count:** ~150+

**Problem:** Many stdlib modules have incomplete or incorrect Rust equivalents.

**Key Missing Mappings:**
- `math` → `std::f64::consts`
- `json` → `serde_json` (mostly working)
- `csv` → `csv` crate
- `datetime` → `chrono` crate
- `re` → `regex` crate
- `random` → `rand` crate
- `asyncio` → `tokio` or `async-std`

**Fix:** Complete module mapping configuration.

---

### Issue 18: Property Decorator

**Affected Categories:** properties, classes  
**Failure Count:** ~20+

**Problem:** `@property` not correctly generating getter methods.

**Fix:** Generate proper accessor methods without `serde_json::Value`.

---

### Issue 19: Slice Assignment

**Affected Categories:** slicing, assignment  
**Failure Count:** ~30+

**Problem:** Slice assignment syntax not correctly handled.

**Examples:**
```rust
// Expected:
pub fn replace_middle(mut items: Vec<i32>, ...) { ... }

// Actual:  
pub fn replace_middle(items: &mut Vec<i32>, ...) { ... }
```

**Fix:** Correct mutability inference for function parameters.

---

### Issue 20: `__dunder__` Methods

**Affected Categories:** dunder-*  
**Failure Count:** ~200+

**Problem:** Python dunder methods not consistently mapped to Rust traits.

**Key Mappings Needed:**
| Python | Rust |
|--------|------|
| `__add__` | `impl Add` |
| `__eq__` | `impl PartialEq` |
| `__hash__` | `impl Hash` |
| `__iter__` | `impl Iterator` |
| `__str__` | `impl Display` |
| `__repr__` | `impl Debug` |
| `__enter__/__exit__` | `impl Drop` + RAII |

**Fix:** Implement dunder method → trait mapping.

---

## Low Priority Issues

### Issue 21: Exception Groups (Python 3.11+)

**Affected Categories:** exception-groups  
**Failure Count:** ~20+

**Problem:** `except*` syntax not supported.

**Fix:** Document as unsupported or generate `Result` with multiple error handling.

---

### Issue 22: Complex Numbers

**Affected Categories:** complex-numbers  
**Failure Count:** ~25+

**Problem:** Complex number literals and operations not supported.

**Fix:** Use `num-complex` crate mapping.

---

### Issue 23: Binary/Hex/Octal Literals

**Affected Categories:** binary-literals  
**Failure Count:** ~22+

**Problem:** Numeric literal formats sometimes mishandled.

**Fix:** Preserve literal format in code generation.

---

### Issue 24: F-String Debug (`f"{x=}"`)

**Affected Categories:** fstrings-debug  
**Failure Count:** ~15+

**Problem:** Debug f-string syntax not supported.

**Fix:** Expand `{x=}` to `"x = {x}"` pattern.

---

### Issue 25: Formatting Minor Issues

**Affected Categories:** pattern-matching, various  
**Failure Count:** ~50+

**Problem:** Minor formatting differences (line breaks, closing braces).

**Fix:** Run `rustfmt` on expected output or normalize comparison.

---

### Issue 26: Match Pattern Binding (`as` patterns)

**Affected Categories:** match-as-patterns, parser-pattern-matching  
**Failure Count:** ~40+

**Problem:** Match patterns with `as` binding and complex pattern matching not transpiled.

**Examples:**
```python
# Python
match value:
    case [x, y] as point:
        return point
```
```rust
// Expected:
match &value[..] {
    [x, y] => {
        let point = value.clone();
        (*x, *y, point)
    }
    _ => (0, 0, vec![]),
}

// Actual: Empty output
```

**Fix:** Implement `as` pattern binding in match expression transpilation.

---

### Issue 27: Unpacking/Destructuring

**Affected Categories:** unpacking  
**Failure Count:** ~17+

**Problem:** List/tuple unpacking to multiple variables not transpiled.

**Examples:**
```python
a, b, c = [1, 2, 3]
```
```rust
// Expected:
let (a, b, c) = vec![1, 2, 3];

// Actual: Empty output
```

**Fix:** Implement destructuring assignment transpilation.

---

### Issue 28: CSE (Common Subexpression Elimination) Optimization

**Affected Categories:** cse-optimization  
**Failure Count:** ~25

**Problem:** CSE optimization generates extra temporary variables or misses opportunities.

**Examples:**
```rust
// Expected:
temp = value * 2;

// Actual:
let _cse_temp_0 = value.clone() * 2;
temp = _cse_temp_0;
```

**Fix:** Refine CSE optimization to avoid unnecessary variables and clones.

---

### Issue 29: Optional Clone Redundancy

**Affected Categories:** optional-types, lifetimes  
**Failure Count:** ~30+

**Problem:** Unnecessary `.clone()` calls on Option types.

**Examples:**
```rust
// Expected:
return value.unwrap() * 2;

// Actual:
return value.clone().unwrap() * 2;
```

**Fix:** Don't clone Options when the borrow is sufficient.

---

### Issue 30: Module-Level `lazy_static!` Patterns

**Affected Categories:** set-operations, collections  
**Failure Count:** ~50+

**Problem:** Module-level mutable collections incorrectly wrapped in `lazy_static!`.

**Examples:**
```rust
// Actual (incorrect):
pub const result: i32 = s.len() as i32;
lazy_static! {
    pub static ref s: HashSet<i32> = { ... };
}

// Expected: Proper scoping or test function
```

**Fix:** Module-level collection code should be wrapped in test functions, not lazy_static.

---

### Issue 31: Global/Nonlocal Variable Handling

**Affected Categories:** global-nonlocal  
**Failure Count:** ~19

**Problem:** Python `global` and `nonlocal` keywords are not properly transpiled to Rust's thread-local or Cell-based patterns.

**Examples:**
```python
# Python
x = 10
def modify():
    global x
    x = 20
```
```rust
// Expected:
use std::cell::Cell;
thread_local! {
    static X: Cell<i32> = Cell::new(10);
}
fn modify() {
    X.with(|x| x.set(20));
}

// Actual:
pub fn modify() {
    let x = 20;  // Local variable, doesn't modify global
}
```

**Fix:** Implement proper global/nonlocal transpilation using `thread_local!` and `Cell/RefCell`.

---

### Issue 32: `__slots__` Handling

**Affected Categories:** slots  
**Failure Count:** ~17

**Problem:** Python `__slots__` is incorrectly transpiled as a const Vec instead of being used to inform struct field generation.

**Examples:**
```rust
// Expected:
struct Point {
    x: i32,
    y: i32,
}

// Actual:
pub struct Point {
    pub x: serde_json::Value,
    pub y: serde_json::Value,
}
impl Point {
    pub const __slots__: Vec<String> = vec!["x".to_string(), "y".to_string()];  // Wrong!
}
```

**Fix:** Use `__slots__` to determine struct fields, not as a runtime constant.

---

### Issue 33: Protocol → Trait with `impl Trait` Syntax

**Affected Categories:** protocols  
**Failure Count:** ~11

**Problem:** Protocol classes should generate traits, and functions using protocols should use `impl Trait` syntax.

**Examples:**
```rust
// Expected:
pub trait Drawable {
    fn draw(&self) -> String;
}
pub fn render(shape: &impl Drawable) -> String { ... }

// Actual:
pub fn render(shape: &Drawable) -> String { ... }  // Missing trait definition
```

**Fix:** Generate trait definitions and use `&impl Trait` for protocol parameters.

---

### Issue 34: Descriptor Protocol

**Affected Categories:** descriptors  
**Failure Count:** ~17

**Problem:** Python descriptors (`__get__`, `__set__`, `__delete__`) have no direct Rust equivalent and are incorrectly transpiled.

**Fix:** Document as advanced feature; consider using procedural macros or custom accessor methods.

---

### Issue 35: `__init_subclass__` Hook

**Affected Categories:** init-subclass  
**Failure Count:** ~15

**Problem:** Dynamic subclass registration hooks don't exist in Rust.

**Fix:** Document as unsupported; suggest macro-based alternatives for compile-time registration.

---

### Issue 36: String Parameter Ownership (`&str` vs `String`)

**Affected Categories:** concurrency, string-operations, many others  
**Failure Count:** ~100+

**Problem:** String parameters are always transpiled as `String` instead of `&str` where appropriate.

**Examples:**
```rust
// Expected:
pub fn worker(name: &str) -> String { ... }

// Actual:
pub fn worker(name: String) -> String { ... }
```

**Fix:** Prefer `&str` for read-only string parameters; use `String` only when ownership is needed.

---

### Issue 37: Float Type Precision (`f32` vs `f64`)

**Affected Categories:** ffi-ctypes, numeric-precision  
**Failure Count:** ~20+

**Problem:** Python floats map to both `f32` and `f64` inconsistently; C `float` should map to `f32`.

**Examples:**
```rust
// Expected:
pub fn c_float_to_float(value: f32) -> f32 { ... }

// Actual:
pub fn c_float_to_float(value: f64) -> f64 { ... }
```

**Fix:** Use context to determine float precision; `ctypes.c_float` → `f32`.

---

### Issue 38: Return Type Consistency (with `Result`)

**Affected Categories:** error-handling, io-streams, verification-contracts  
**Failure Count:** ~50+

**Problem:** Functions that can fail have inconsistent return types—sometimes returning plain type, sometimes `Result<T, E>`.

**Examples:**
```rust
// Expected (when function doesn't use try/except):
pub fn safe_access(items: &Vec<i32>, idx: i32) -> i32 { ... }

// Actual (unnecessary Result):
pub fn safe_access(items: &Vec<i32>, idx: i32) -> Result<i32, IndexError> { ... }
```

**Fix:** Only wrap in `Result` when the function actually handles or propagates errors.

---

### Issue 39: Dead Code Elimination Not Applied

**Affected Categories:** dead-code-elimination, optimizer-inlining  
**Failure Count:** ~9

**Problem:** Dead code elimination optimization not removing unused variables.

**Examples:**
```rust
// Expected:
pub fn use_only_y(x: i32, y: i32) -> i32 {
    return y;  // unused `x * 2` eliminated
}

// Actual:
pub fn use_only_y(x: i32, y: i32) -> i32 {
    let unused = x * 2;  // Should be eliminated
    return y;
}
```

**Fix:** Implement proper dead code elimination pass.

---

### Issue 40: Double Type Cast

**Affected Categories:** string-encoding, cast-expressions  
**Failure Count:** ~10+

**Problem:** Redundant type casts are generated (e.g., `as i32 as i32`).

**Examples:**
```rust
// Expected:
return s.len() as i32;

// Actual:
return s.encode("utf-8").len() as i32 as i32;  // Double cast
```

**Fix:** Deduplicate consecutive identical casts.

---

### Issue 41: Missing Return Type on Functions

**Affected Categories:** borrow-checker-violations, functions  
**Failure Count:** ~15+

**Problem:** Functions with return statements sometimes generate `fn name() { ... }` without return type.

**Examples:**
```rust
// Expected:
pub fn caller() -> i32 { ... }

// Actual:
pub fn caller() { ... }  // Missing return type
```

**Fix:** Infer return type from return statements.

---

### Issue 42: Ellipsis (`...`) Not Transpiled

**Affected Categories:** ellipsis  
**Failure Count:** ~15

**Problem:** Python `...` (Ellipsis) has no direct Rust equivalent; module-level usage generates empty output.

**Fix:** Map to `()` for type placeholders; use `todo!()` or `unimplemented!()` for function bodies.

---

### Issue 43: Python 3.11+ `Self` Type

**Affected Categories:** version-features  
**Failure Count:** ~2

**Problem:** Python 3.11's `Self` type hint in classes fails to transpile.

**Error:** `Statement type not yet supported: ClassDef (classes)`

**Fix:** Handle `Self` type annotation → map to `Self` in Rust impl blocks.

---

### Issue 44: Default Parameter Values Not Properly Handled

**Affected Categories:** functions  
**Failure Count:** ~20+

**Problem:** Python default parameter values are not properly transpiled to `Option<T>` with `unwrap_or()`.

**Examples:**
```python
# Python
def greet_with_default(name: str, greeting: str = "Hello") -> str:
    return f"{greeting} {name}"
```
```rust
// Expected:
pub fn greet_with_default(name: String, greeting: Option<String>) -> String {
    let greeting = greeting.unwrap_or("Hello".to_string());
    return format!("{} {}", greeting, name);
}

// Actual:
pub fn greet_with_default(name: String, greeting: String) -> String {
    return format!("{}{}", format!("{}{}", greeting, " "), name);
}
```

**Fix:** Transpile default parameters as `Option<T>` with proper unwrapping.

---

### Issue 45: Function Callback/Closure Parameters as `serde_json::Value`

**Affected Categories:** ffi-ctypes, decorators-basic, functools-*  
**Failure Count:** ~50+

**Problem:** Function parameters that accept callbacks/closures are typed as `serde_json::Value` instead of generic `Fn` traits.

**Examples:**
```rust
// Expected:
pub fn apply_callback<F>(value: i32, callback: F) -> i32
where
    F: Fn(i32) -> i32,
{ ... }

// Actual:
pub fn apply_callback(value: i32, callback: &serde_json::Value) -> i32 { ... }
```

**Fix:** Detect callable parameters and generate generic `Fn`/`FnMut`/`FnOnce` bounds.

---

### Issue 46: Operator Trait Implementations Missing

**Affected Categories:** dunder-arithmetic, dunder-comparison  
**Failure Count:** ~50+

**Problem:** Python dunder methods for operators (`__add__`, `__mul__`, etc.) don't generate corresponding Rust `impl` blocks for `std::ops` traits.

**Examples:**
```rust
// Expected:
impl std::ops::Add for Number {
    type Output = Number;
    fn add(self, other: Number) -> Number {
        Number { value: self.value + other.value }
    }
}

// Actual: (missing entirely, only struct definition)
pub struct Number {
    pub value: serde_json::Value,
}
```

**Fix:** Map dunder operators to `std::ops::*` trait implementations.

---

### Issue 47: `Display` and `Debug` Trait Implementations Missing

**Affected Categories:** dunder-lifecycle  
**Failure Count:** ~20+

**Problem:** Python `__str__` and `__repr__` methods don't generate `impl fmt::Display` and `impl fmt::Debug`.

**Examples:**
```rust
// Expected:
impl fmt::Display for Product {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - ${:.2}", self.name, self.price)
    }
}
impl fmt::Debug for Product {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Product(name='{}', price={})", self.name, self.price)
    }
}

// Actual: Missing, just #[derive(Debug)] without custom formatting
```

**Fix:** Map `__str__` → `impl Display`, `__repr__` → `impl Debug`.

---

### Issue 48: Template String Module Not Supported

**Affected Categories:** template-strings  
**Failure Count:** ~22

**Problem:** Python `string.Template` class and its substitution methods generate empty output.

**Examples:**
```python
from string import Template
t = Template("Hello $name")
result = t.substitute(name="World")
```
```rust
// Expected:
let name = "World";
let result = format!("Hello {}", name);

// Actual: Empty output
```

**Fix:** Detect `string.Template` usage and convert to `format!()` macro.

---

### Issue 49: `unwrap_or_default()` vs `unwrap()` Inconsistency

**Affected Categories:** examples-stdlib, optional-types  
**Failure Count:** ~30+

**Problem:** Inconsistent use of `unwrap_or_default()` vs `unwrap()` for Option unwrapping.

**Examples:**
```rust
// Expected (when value cannot be None):
.get("name").cloned().unwrap()

// Actual:
.get("name").cloned().unwrap_or_default()
```

**Fix:** Use `unwrap_or_default()` only when a default makes semantic sense.

---

### Issue 50: Missing `return` Keyword in Expression-Body Functions

**Affected Categories:** examples-stdlib, examples-tooling  
**Failure Count:** ~40+

**Problem:** Some functions that should use implicit return (expression-body) incorrectly have explicit `return`, or vice versa.

**Examples:**
```rust
// Expected (Rust idiom):
pub fn factorial(n: i32) -> i32 {
    let mut result = 1;
    for i in 2..n + 1 { result *= i; }
    result  // implicit return
}

// Actual:
pub fn factorial(n: i32) -> i32 {
    let mut result = 1;
    for i in 2..n + 1 { result *= i; }
    return result;  // explicit return (less idiomatic)
}
```

**Fix:** Use implicit returns for simple expression-body functions.

---

### Issue 51: Binary Literal Format Not Preserved

**Affected Categories:** binary-literals  
**Failure Count:** ~22

**Problem:** Binary (`0b`), octal (`0o`), and hex (`0x`) literals are converted to decimal.

**Examples:**
```rust
// Expected:
let result = (0b0 == 0) && (0o0 == 0) && (0x0 == 0);

// Actual:
pub const result: serde_json::Value = ((0 == 0) && (0 == 0)) && (0 == 0);
```

**Fix:** Preserve numeric literal format in code generation.

---

### Issue 52: Missing Semicolon/Closing Brace Issues

**Affected Categories:** modules-math, modules-os-sys, type-inference  
**Failure Count:** ~30+

**Problem:** Generated code sometimes has missing semicolons or trailing closing braces, or extra semicolons after function definitions.

**Examples:**
```rust
// Expected:
pub fn add_numbers(a: i32, b: i32) -> i32 {
    return a + b;
}

// Actual (various issues):
pub fn add_numbers(a: i32, b: i32) -> i32 {
    return a + b;
};  // Extra semicolon

// Or:
    return path_obj.strip_prefix(start_obj)...
}  // Missing code after this
```

**Fix:** Clean up syntax generation in code emitter.

---

### Issue 53: IndexError/Exception Types Generated Unnecessarily

**Affected Categories:** copy-type-semantics, return-statements  
**Failure Count:** ~20+

**Problem:** Error type definitions are generated even when not used by the function.

**Examples:**
```rust
// Actual (unnecessary error type):
#[derive(Debug, Clone)]
pub struct IndexError { ... }
impl std::fmt::Display for IndexError { ... }
impl std::error::Error for IndexError {}

pub fn swap_tuple(pair: (i32, i32)) -> (i32, i32) {
    return (pair.1, pair.0);  // Never throws IndexError
}
```

**Fix:** Only generate error types when they're actually used.

---

### Issue 54: Option/Result Wrapping for Simple Functions

**Affected Categories:** return-statements  
**Failure Count:** ~15+

**Problem:** Functions returning `Option<T>` are incorrectly wrapped in `Result<Option<T>, E>`.

**Examples:**
```rust
// Expected:
pub fn find_or_none(items: &Vec<i32>, target: i32) -> Option<i32> {
    if items.is_empty() { return None; }
    return Some(items[0]);
}

// Actual:
pub fn find_or_none(items: &Vec<i32>, target: i32) -> Result<Option<i32>, IndexError> {
    if items.is_empty() { return Ok(None); }
    return Ok(Some(items[0]));
}
```

**Fix:** Don't double-wrap Optional returns in Result unless error handling is present.

---

### Issue 55: Positional-Only Parameters Not Captured

**Affected Categories:** positional-only-args  
**Failure Count:** ~14

**Problem:** Python positional-only parameters (before `/`) are not being captured in the function signature.

**Examples:**
```python
# Python
def calculate(x, /):
    return x * 2
```
```rust
// Expected:
fn calculate(x: i32) -> i32 {
    x * 2
}

// Actual:
pub fn calculate() -> i32 {  // Missing parameter!
    return x * 2;
}
```

**Fix:** Parse positional-only parameter syntax and include parameters in function signature.

---

### Issue 56: Varargs (`*args`) Not Handled

**Affected Categories:** positional-only-args, functions  
**Failure Count:** ~20+

**Problem:** Python `*args` variadic parameters generate functions with missing parameters.

**Examples:**
```python
# Python
def with_varargs(a, /, *args):
    return f"a={a}, args={args}"
```
```rust
// Expected:
fn with_varargs(a: i32, args: &[i32]) -> String {
    format!("a={}, args={:?}", a, args)
}

// Actual:
pub fn with_varargs() -> String {  // All parameters missing!
    return format!("a={}, args={}", a, args);
}
```

**Fix:** Map `*args` to slice parameter `&[T]` or `Vec<T>`.

---

### Issue 57: Type Parameter Order Inconsistency

**Affected Categories:** type-inference  
**Failure Count:** ~5+

**Problem:** Generic type parameters are sometimes reordered in the output.

**Examples:**
```rust
// Expected:
pub fn pair<T: Clone, U: Clone>(a: &T, b: &U) -> (T, U) { ... }

// Actual:
pub fn pair<U: Clone, T: Clone>(a: &T, b: &U) -> (T, U) { ... }
```

**Fix:** Preserve type parameter declaration order from Python.

---

### Issue 58: Empty Vec Element Type Inference

**Affected Categories:** lists-arrays, type-inference  
**Failure Count:** ~15+

**Problem:** Empty vectors in nested structures default element type to `serde_json::Value`.

**Examples:**
```rust
// Expected:
pub fn test_empty_nested() -> Vec<Vec<i32>> {
    let data = vec![vec![], vec![1, 2], vec![], vec![3]];
    return data;
}

// Actual:
pub fn test_empty_nested() -> Vec<Vec<serde_json::Value>> { ... }
```

**Fix:** Infer element type from non-empty siblings in the same collection.

---

### Issue 59: Constant String Extraction (`const STR__`)

**Affected Categories:** examples-stdlib  
**Failure Count:** ~30+

**Problem:** Repeated string literals are extracted into constants but with mangled names like `STR__`.

**Examples:**
```rust
// Expected:
log::info!("{}", "=".repeat(60));

// Actual (uses const that may not exist):
const STR__: &'static str = "=";
log::info!("{}", STR__.repeat(60 as usize));
```

**Fix:** Either inline repeated strings or generate proper constant names.

---

### Issue 60: `.copied()` vs `.cloned()` Inconsistency

**Affected Categories:** examples-stdlib, iterators  
**Failure Count:** ~20+

**Problem:** Iterator methods use `.copied()` and `.cloned()` inconsistently.

**Examples:**
```rust
// Expected (for Copy types):
.iter().copied().map(...)

// Actual (incorrect for Copy types):
.iter().cloned().map(...)
```

**Fix:** Use `.copied()` for `Copy` types, `.cloned()` for `Clone` types.

---

### Issue 61: Async Generator State Machine Formatting

**Affected Categories:** async-generators  
**Failure Count:** ~22

**Problem:** Async generator state machine code is generated without proper formatting/indentation.

**Examples:**
```rust
// Actual (poor formatting):
#[doc = " Generator state struct"] #[derive(Debug)] struct GenState {
    state: usize ,
}
#[doc = " Generator function - returns Iterator"] pub fn gen() -> impl Iterator<Item = i32>{
    GenState {
    state: 0 ,
}
}

// Expected (properly formatted):
#[derive(Debug)]
struct GenState {
    state: usize,
}

pub fn gen() -> impl Iterator<Item = i32> {
    GenState { state: 0 }
}
```

**Fix:** Run formatter on generated generator code.

---

### Issue 62: Floor Division Wraps in Unnecessary `Result`

**Affected Categories:** operators, constant-type-inference  
**Failure Count:** ~30+

**Problem:** Floor division (`//`) generates `Result<i32, ZeroDivisionError>` even when divisor is a non-zero constant.

**Examples:**
```rust
// Expected (divisor is constant 2):
pub fn floor_divide() -> i32 {
    return 5 / 2;
}

// Actual:
pub fn floor_divide() -> Result<i32, ZeroDivisionError> {
    return Ok({ ... complex floor div logic ... });
}
```

**Fix:** Only wrap in Result if divisor could be zero at runtime.

---

### Issue 63: Module-Level Code Generates Empty Output

**Affected Categories:** match-mapping-patterns, counter, augmented-assignment  
**Failure Count:** ~50+

**Problem:** Some module-level code patterns generate completely empty output instead of a test function or main.

**Examples:**
```python
# Python
x = 10
x //= 3
print(x)
```
```rust
// Actual: Empty output (nothing generated)
```

**Fix:** Wrap standalone module-level code in a test or main function.

---

### Issue 64: Bytes Literal Contains Check

**Affected Categories:** bytes  
**Failure Count:** ~25

**Problem:** Bytes `.contains()` check generates incorrect syntax.

**Examples:**
```rust
// Expected:
let result = b"hello".contains(&b'l');

// Actual:
pub const result: serde_json::Value = b"hello".contains(&b"l");  // Wrong: b"l" instead of b'l'
```

**Fix:** Use byte literal `b'x'` for single byte checks.

---

### Issue 65: Async Stream Libraries Not Used

**Affected Categories:** async-generators  
**Failure Count:** ~22

**Problem:** Expected async generator code uses `async_stream` crate but actual code doesn't.

**Examples:**
```rust
// Expected:
use async_stream::stream;
use tokio_stream::StreamExt;

async fn gen() -> impl Stream<Item = i32> {
    stream! { yield 42; }
}

// Actual: Uses sync Iterator pattern instead of async Stream
```

**Fix:** Use proper async stream patterns for async generators.

---

### Issue 66: ChainMap Uses Non-Existent Rust Type

**Affected Categories:** chainmap  
**Failure Count:** ~27

**Problem:** Python's `ChainMap` is transpiled using `std::collections::ChainMap` which doesn't exist in Rust.

**Examples:**
```rust
// Actual (invalid):
use std::collections::ChainMap;
let c = ChainMap::new(dict1, dict2);

// Expected (manual implementation):
let maps = vec![dict1, dict2];
for map in &maps {
    if let Some(val) = map.get(&key) { ... }
}
```

**Fix:** Implement ChainMap as a custom struct or use Vec<HashMap> pattern.

---

### Issue 67: NamedTuple Generates Wrong Import

**Affected Categories:** namedtuple  
**Failure Count:** ~25

**Problem:** Python's `namedtuple` generates invalid `use std::collections::namedtuple` instead of a struct.

**Examples:**
```python
# Python
from collections import namedtuple
Point = namedtuple('Point', ['x', 'y'])
```
```rust
// Expected:
struct Point {
    x: i32,
    y: i32,
}

// Actual:
use std::collections::namedtuple;  // Invalid!
```

**Fix:** Generate struct definition with fields, not an import statement.

---

### Issue 68: Collection Method Bodies Not Generated

**Affected Categories:** deque, chainmap, counter  
**Failure Count:** ~80+

**Problem:** Collection-related code generates only the import without the method body.

**Examples:**
```python
# Python
from collections import deque
d = deque([1, 2])
d.append(3)
```
```rust
// Expected:
use std::collections::VecDeque;
let mut d = VecDeque::from(vec![1, 2]);
d.push_back(3);

// Actual:
use std::collections::VecDeque;
// No code body generated!
```

**Fix:** Generate complete method body, not just imports.

---

### Issue 69: Missing Bracket in Derive Macro

**Affected Categories:** new-vs-init, dataclasses-frozen  
**Failure Count:** ~20+

**Problem:** `#[derive(Debug, Copy, Clone]` missing closing bracket.

**Examples:**
```rust
// Expected:
#[derive(Debug, Copy, Clone)]
pub struct Counter { ... }

// Actual:
#[derive(Debug, Copy, Clone]  // Missing ]
pub struct Counter { ... }
```

**Fix:** Ensure derive macro attributes have matching brackets.

---

### Issue 70: Constructor Ignores Default Parameter

**Affected Categories:** new-vs-init, dataclasses  
**Failure Count:** ~25+

**Problem:** Constructor ignores passed parameter and uses hardcoded default.

**Examples:**
```rust
// Expected:
pub fn new(start: i32) -> Self {
    Self { value: start }
}

// Actual:
pub fn new(start: i32) -> Self {
    Self { value: 0 }  // Ignores 'start' parameter!
}
```

**Fix:** Use parameter value in struct initialization.

---

### Issue 71: `bytes` Type Not Mapped to `Vec<u8>`

**Affected Categories:** bytes-strings, bytes  
**Failure Count:** ~51

**Problem:** Python `bytes` type generates invalid Rust type `bytes` instead of `Vec<u8>`.

**Examples:**
```rust
// Expected:
pub fn test_bytes_empty() -> (Vec<u8>, Vec<u8>) {
    return (b"".to_vec(), Vec::new());
}

// Actual:
pub fn test_bytes_empty() -> (bytes, bytes) {  // Invalid type 'bytes'
    return (b"", bytes());
}
```

**Fix:** Map Python `bytes` type to `Vec<u8>`.

---

### Issue 72: Context Manager `__enter__`/`__exit__` Methods Generated

**Affected Categories:** parenthesized-context, contextlib-decorator  
**Failure Count:** ~30+

**Problem:** Context managers generate Python-like `__enter__`/`__exit__` methods instead of RAII pattern.

**Examples:**
```rust
// Expected (RAII pattern):
{
    let a = 1;
    { let b = 2; ... }
}

// Actual (invalid Python-style):
impl CM {
    pub fn __enter__(&self) { ... }
    pub fn __exit__(&self) { ... }
}
```

**Fix:** Use RAII/Drop pattern instead of dunder methods.

---

### Issue 73: `@contextmanager` Generator Mishandled

**Affected Categories:** contextlib-decorator  
**Failure Count:** ~20+

**Problem:** `@contextmanager` decorators generate iterator state machines instead of closure-based context.

**Examples:**
```rust
// Expected:
fn suppress_errors<F, R>(f: F) -> Option<R> 
where F: FnOnce() -> Result<R, String> {
    match f() { Ok(val) => Some(val), Err(_) => None }
}

// Actual (iterator pattern):
struct SuppressErrorsState { state: usize }
pub fn suppress_errors() -> impl Iterator<Item = serde_json::Value> { ... }
```

**Fix:** Recognize `@contextmanager` and generate closure-based context manager.

---

### Issue 74: `sys.getsizeof` Not Mapped to `std::mem::size_of`

**Affected Categories:** dunder-format  
**Failure Count:** ~10+

**Problem:** `sys.getsizeof()` not mapped to Rust's `std::mem::size_of`.

**Examples:**
```rust
// Expected:
fn sizeof(&self) -> usize {
    std::mem::size_of::<Self>() + self.data.len()
}

// Actual: Method not generated
```

**Fix:** Map `sys.getsizeof()` to `std::mem::size_of_val()`.

---

### Issue 75: Async Iterator `Stream` Trait Formatting

**Affected Categories:** async-for  
**Failure Count:** ~15+

**Problem:** Async iterators generate poorly formatted code with missing newlines.

**Examples:**
```rust
// Actual (poor formatting):
#[derive(Debug, Clone)] pub struct AsyncRange {
    pub n: serde_json::Value, pub i: i32
}
impl AsyncRange {
    pub fn new(n: serde_json::Value) -> Self {
    Self {
    n, i: 0
}
}
```

**Fix:** Run formatter on async iterator generated code.

---

### Issue 76: Double `.clone()` Calls

**Affected Categories:** regressions  
**Failure Count:** ~15+

**Problem:** Unnecessary double clone operations: `(original.clone()).clone()`.

**Examples:**
```rust
// Expected:
let mut copied = original.clone();

// Actual:
let mut copied = (original.clone()).clone();
```

**Fix:** Remove redundant clone operations.

---

### Issue 77: Double Type Cast `as i32 as i32`

**Affected Categories:** regressions, assignment  
**Failure Count:** ~20+

**Problem:** Redundant double type casts.

**Examples:**
```rust
// Expected:
return original.len() as i32;

// Actual:
return original.len() as i32 as i32;
```

**Fix:** Remove redundant casts.

---

### Issue 78: Index Expression Operator Precedence

**Affected Categories:** assignment  
**Failure Count:** ~15+

**Problem:** Array indexing with expression has incorrect operator precedence.

**Examples:**
```rust
// Expected:
items[(i + 1) as usize] = 100;

// Actual:
items[i + 1 as usize] = 100;  // Wrong precedence - casts 1, not (i+1)
```

**Fix:** Parenthesize index expressions before cast.

---

### Issue 79: `lazy_static!` Not Used for Vec Constants

**Affected Categories:** basic-types  
**Failure Count:** ~10+

**Problem:** Vec constants need `lazy_static!` but are declared as `const`.

**Examples:**
```rust
// Expected:
lazy_static! {
    pub static ref VEC: Vec<i32> = vec![1, -2, 3, -4];
}

// Actual:
pub const VEC: serde_json::Value = vec![1, -2, 3, -4];
```

**Fix:** Use `lazy_static!` or `once_cell` for non-const initialized constants.

---

### Issue 80: ABC Generates Struct Instead of Trait

**Affected Categories:** abc  
**Failure Count:** ~20+

**Problem:** Python ABC generates a struct instead of a trait.

**Examples:**
```rust
// Expected:
trait MyABC {}

// Actual:
pub struct MyABC {}
impl MyABC { pub fn new() -> Self { Self {} } }
```

**Fix:** Generate trait for ABC-derived classes.

---

### Issue 81: String Methods Return Empty Output

**Affected Categories:** string-methods-all  
**Failure Count:** ~20+

**Problem:** Some string methods generate empty output.

**Examples:**
```rust
// Expected:
pub const result: Vec<String> = "a\nb\nc"
    .lines()
    .map(|s| s.to_string())
    .collect::<Vec<String>>();

// Actual: (empty)
```

**Fix:** Generate proper string method calls.

---

### Issue 82: Assert Statements Generate Empty Output

**Affected Categories:** assert-statement  
**Failure Count:** ~15+

**Problem:** Assert statements in module-level code generate empty output.

**Examples:**
```python
# Python
assert True
result = True
```
```rust
// Expected:
fn main() {
    assert!(true);
    let result = true;
}

// Actual: (empty)
```

**Fix:** Wrap module-level asserts in main/test function.

---

### Issue 83: `nonlocal` Uses Invalid Rust Syntax

**Affected Categories:** global-nonlocal  
**Failure Count:** ~19

**Problem:** `nonlocal` generates direct variable mutation that doesn't compile.

**Examples:**
```rust
// Expected:
use std::cell::Cell;
let count = Cell::new(0);
let increment = || { count.set(count.get() + 1); };

// Actual:
let count = 0;
fn increment() -> () {
    count += 1;  // Can't mutate captured variable!
}
```

**Fix:** Use `Cell`, `RefCell`, or `Rc<RefCell<>>` for nonlocal mutation.

---

### Issue 84: Functools Cache Removed

**Affected Categories:** functools-cache  
**Failure Count:** ~15+

**Problem:** `@functools.cache` decorator removed instead of implementing memoization.

**Examples:**
```rust
// Expected:
fn fib(n: i32, cache: &mut HashMap<i32, i32>) -> i32 {
    if let Some(&val) = cache.get(&n) { return val; }
    // ...
}

// Actual:
pub fn fib(n: i32) -> i32 {
    if n < 2 { return n; }
    return fib(n - 1) + fib(n - 2);  // No memoization!
}
```

**Fix:** Implement memoization with HashMap cache parameter or lazy_static.

---

### Issue 85: Exception Handlers Missing Control Flow

**Affected Categories:** error-handling, exceptions  
**Failure Count:** ~50+

**Problem:** Try/except blocks lose control flow - code after `return` is unreachable.

**Examples:**
```rust
// Actual (dead code after return):
{
    for _item in items.iter().cloned() {
        if item < 0 { panic!("negative"); }
        total += item;
    }
    return -1;
    log::info!("{}", "done processing");  // Unreachable!
}
```

**Fix:** Properly structure try/except/finally control flow.

---

### Issue 86: Match Guards Generate Empty Output

**Affected Categories:** match-guards  
**Failure Count:** ~10+

**Problem:** Pattern matching with guards generates empty output.

**Examples:**
```rust
// Expected:
let result = match value.as_slice() {
    [first, rest @ ..] if rest.len() > 0 => *first == 1 && rest.len() == 3,
    _ => false,
};

// Actual: (empty)
```

**Fix:** Support match guards in pattern matching.

---

### Issue 87: PathBuf Methods Not Fully Mapped

**Affected Categories:** examples-stdlib  
**Failure Count:** ~10+

**Problem:** PathBuf methods like `.name`, `.parent.name` not properly mapped.

**Examples:**
```rust
// Expected:
path.file_name().unwrap()
path.parent().unwrap().file_name()

// Actual:
path.name  // Invalid
path.parent.name  // Invalid
```

**Fix:** Map Python pathlib attributes to Rust PathBuf methods.

---

### Issue 88: HashMap Type Params Lose Specificity

**Affected Categories:** collections, exceptions  
**Failure Count:** ~25+

**Problem:** HashMap with specific key/value types gets `serde_json::Value` parameters.

**Examples:**
```rust
// Expected:
pub fn get_value(data: &HashMap<String, i32>, key: String) -> String

// Actual:
pub fn get_value(data: &HashMap<serde_json::Value, serde_json::Value>, key: String) -> String
```

**Fix:** Preserve HashMap type parameters from Python type hints.

---

### Issue 89: Missing `pub` on Constants

**Affected Categories:** lazy-static-detection  
**Failure Count:** ~10+

**Problem:** Constants are not marked `pub` when they should be.

**Examples:**
```rust
// Expected:
pub const ENABLED: bool = true;

// Actual:
const ENABLED: bool = true;
```

**Fix:** Add `pub` visibility modifier to exported constants.

---

### Issue 90: Field Alias Clone vs Reference

**Affected Categories:** field-alias-mutation  
**Failure Count:** ~10+

**Problem:** Mutable parameter references become immutable, and fields are moved instead of cloned.

**Examples:**
```rust
// Expected:
pub fn add_to_list(state: &mut State) {
    let mut items = state.items.clone();
    items.push(1);
}

// Actual:
pub fn add_to_list(state: &State) {  // Missing &mut
    let mut items = state.items;  // Missing .clone()
    items.push(1);
}
```

**Fix:** Preserve mutability and add `.clone()` for field access.

---

### Issue 91: Implicit Return Not Generated

**Affected Categories:** examples-stdlib, functions  
**Failure Count:** ~40+

**Problem:** Functions with implicit returns don't generate `return` statement.

**Examples:**
```rust
// Expected:
pub fn get_current_dir() -> String {
    return std::env::current_dir().unwrap().to_string_lossy().to_string();
}

// Actual:
pub fn get_current_dir() -> String {
    std::env::current_dir().unwrap().to_string_lossy().to_string()  // Missing return
}
```

**Fix:** Add explicit `return` for all function returns.

---

### Issue 92: String Lifetime Parameters in Functions

**Affected Categories:** examples-stdlib  
**Failure Count:** ~15+

**Problem:** Functions generate unnecessary lifetime parameters like `<'a, 'b>`.

**Examples:**
```rust
// Expected:
pub fn find_pattern(text: String, pattern: String) -> Vec<String>

// Actual:
pub fn find_pattern<'b, 'a>(text: &'a str, pattern: &'b str) -> Vec<String>
```

**Fix:** Use owned types unless lifetime parameters are necessary.

---

### Issue 93: `__match_args__` Generated for Structs

**Affected Categories:** match-class-patterns  
**Failure Count:** ~15+

**Problem:** Python's `__match_args__` attribute is generated on Rust structs.

**Examples:**
```rust
// Actual (invalid):
impl Point {
    pub const __match_args__: serde_json::Value = ("x".to_string(), "y".to_string());
}
```

**Fix:** Remove `__match_args__` - not needed in Rust pattern matching.

---

### Issue 94: `__call__` Not Mapped to `Fn` Traits

**Affected Categories:** dunder-callable  
**Failure Count:** ~20+

**Problem:** Python `__call__` dunder method generates incomplete struct without the call implementation.

**Examples:**
```rust
// Expected:
impl Adder {
    fn call(&self, x: i32, y: i32) -> i32 { x + y }
}

// Actual:
impl Adder {
    pub fn new() -> Self { Self {} }
}
// Missing call method!
```

**Fix:** Generate `call()` method or implement `Fn` trait.

---

### Issue 95: Bytes Slicing Returns Invalid `bytes` Type

**Affected Categories:** bytes-strings, bytes  
**Failure Count:** ~26

**Problem:** Byte slice operations return `bytes` type instead of `Vec<u8>`.

**Examples:**
```rust
// Expected:
pub fn test_bytes_slice() -> Vec<u8> {
    let bytes = b"hello";
    return bytes[1..4].to_vec();
}

// Actual:
pub fn test_bytes_slice() -> bytes { ... }  // Invalid 'bytes' type
```

**Fix:** Return `Vec<u8>` for byte slice operations.

---

### Issue 96: Dataclass `compare=False` Field Handling

**Affected Categories:** dataclasses-comparison  
**Failure Count:** ~10+

**Problem:** Dataclass fields with `compare=False` generate `field()` call instead of proper exclusion.

**Examples:**
```rust
// Expected:
impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key  // timestamp excluded
    }
}

// Actual:
impl Record {
    pub const timestamp: f64 = field();  // Invalid
}
```

**Fix:** Implement custom PartialEq/Ord to exclude marked fields.

---

### Issue 97: CLI Complex Patterns Generate Code

**Affected Categories:** cli-integration  
**Failure Count:** ~10+

**Problem:** Previously "Statement type not yet supported" errors now generate code, but with issues like missing `HashMap` import and invalid indexing.

**Examples:**
```rust
// Actual issues:
self.difficulty_ranges[self.difficulty as usize]  // String can't be cast to usize
```

**Fix:** Use `.get()` for HashMap access, not array indexing.

---

### Issue 98: Async Generator Methods Not Mapped

**Affected Categories:** async-generators  
**Failure Count:** ~22

**Problem:** Async generator methods like `.asend()`, `.aclose()` not mapped to Rust stream methods.

**Examples:**
```rust
// Actual (invalid):
g.asend(None).await;
g.aclose().await;

// Expected:
drop(g);  // or use proper stream termination
```

**Fix:** Map async generator protocol methods to Rust stream equivalents.

---

### Issue 99: Try/Finally Missing Block Structure

**Affected Categories:** error-handling  
**Failure Count:** ~20+

**Problem:** Try/finally blocks lose their block structure.

**Examples:**
```rust
// Expected:
let mut x = 0;
{ x = 42; log::info!("cleanup"); }
return x;

// Actual:
let x = 0;
x = 42;  // Missing mut, no block
log::info!("cleanup");
return x;
```

**Fix:** Preserve block structure and mutability.

---

### Issue 100: Numeric Underscore Separators Not Preserved

**Affected Categories:** binary-literals  
**Failure Count:** ~22

**Problem:** Python numeric underscores `1_000_000` converted to `1000000`.

**Examples:**
```rust
// Expected:
let x = 1_000_000;

// Actual:
pub const x: i32 = 1000000;
```

**Fix:** Preserve underscore separators in numeric literals.

---

### Issue 101: Context Manager Drop vs `__exit__`

**Affected Categories:** context-managers  
**Failure Count:** ~20+

**Problem:** Context managers generate `__exit__` with exception parameters instead of Drop trait.

**Examples:**
```rust
// Expected:
impl Drop for Observer {
    fn drop(&mut self) { self.success = true; }
}

// Actual:
pub fn __exit__(&mut self, exc_type: Value, exc_val: Value, exc_tb: Value) -> bool { ... }
```

**Fix:** Use Drop trait for simple cleanup, custom RAII for exception handling.

---

### Issue 102: `__init__` Default Parameters Not Generating Factory Methods

**Affected Categories:** dunder-lifecycle  
**Failure Count:** ~15+

**Problem:** Python `__init__` with default parameters doesn't generate Rust factory method variants.

**Examples:**
```rust
// Expected:
impl Config {
    fn new() -> Self { Config { timeout: 30, retries: 3 } }
    fn with_timeout(timeout: i32) -> Self { Config { timeout, retries: 3 } }
    fn with_both(timeout: i32, retries: i32) -> Self { Config { timeout, retries } }
}

// Actual:
impl Config {
    pub fn new(timeout: Value, retries: Value) -> Self { Self { timeout, retries } }
}
```

**Fix:** Generate factory methods for default parameter combinations.

---

### Issue 103: Internal Functions Marked `pub`

**Affected Categories:** multi-module  
**Failure Count:** ~10+

**Problem:** Functions that should be private are marked `pub`.

**Examples:**
```rust
// Expected:
fn internal_helper(x: i32) -> i32 { return x + 1; }

// Actual:
pub fn internal_helper(x: i32) -> i32 { return x + 1; }
```

**Fix:** Respect Python naming conventions (`_` prefix) for visibility.

---

### Issue 104: Missing Trailing Semicolon After Blocks

**Affected Categories:** modules-math, stdlib-misc  
**Failure Count:** ~15+

**Problem:** Functions end with `};` instead of `}`.

**Examples:**
```rust
// Expected:
pub fn absolute_float(x: f64) -> f64 {
    return (x as f64).abs();
}

// Actual:
pub fn absolute_float(x: f64) -> f64 {
    return (x as f64).abs();
};  // Extra semicolon
```

**Fix:** Remove trailing semicolons after function blocks.

---

### Issue 105: `is not` Identity Check Generates Equality

**Affected Categories:** chained-comparisons  
**Failure Count:** ~10+

**Problem:** Python `is not` generates `!=` instead of `!Rc::ptr_eq`.

**Examples:**
```rust
// Expected:
let result = !Rc::ptr_eq(&a, &b) && !Rc::ptr_eq(&b, &c);

// Actual:
pub const result: Value = (a != b) && (b != c);
```

**Fix:** Use `Rc::ptr_eq` for identity checks on reference types.

---

### Issue 106: Match Guards/Or Patterns Generate Empty

**Affected Categories:** match-guards, match-or-patterns  
**Failure Count:** ~15+

**Problem:** Complex match patterns with guards or `|` generate empty output.

**Examples:**
```rust
// Expected:
let result = match tup {
    () | (_,) | (_, _) => true,
    _ => false,
};

// Actual: (empty)
```

**Fix:** Support match or-patterns and guards.

---

### Issue 107: Descriptors Generate Invalid `__slots__`

**Affected Categories:** descriptors  
**Failure Count:** ~17

**Problem:** Descriptor pattern generates invalid `__slots__` const and no getter/setter.

**Examples:**
```rust
// Expected:
impl Descriptor {
    fn get(&self) -> Option<i32> { self.value }
    fn set(&mut self, val: i32) { self.value = Some(val); }
}

// Actual:
impl MyClass {
    pub const __slots__: Vec<String> = vec!["_x".to_string()];
    pub const x: Value = Descriptor::new();
}
```

**Fix:** Generate proper getter/setter methods for descriptors.

---

### Issue 108: Random Module Uses Thread-Local RNG Pattern

**Affected Categories:** examples-stdlib  
**Failure Count:** ~5+

**Problem:** Random module generates `DEPYLER_RNG.with(|rng| ...)` pattern that may not be defined.

**Examples:**
```rust
// Actual:
let rand_int: i32 = DEPYLER_RNG.with(|rng| rng.borrow_mut().gen_range(1..=10));

// Expected:
let rand_int: i32 = rand::thread_rng().gen_range(1..=10);
```

**Fix:** Define thread-local RNG or use `rand::thread_rng()` directly.

---

### Issue 109: CSE Optimization Creates Unused Variables

**Affected Categories:** examples-stdlib  
**Failure Count:** ~25+

**Problem:** CSE (Common Subexpression Elimination) creates `_cse_temp_N` variables that aren't always needed.

**Examples:**
```rust
// Actual (over-optimized):
let _cse_temp_0 = a == b;
let eq: bool = _cse_temp_0;

// Expected (simpler):
let eq: bool = a == b;
```

**Fix:** Only apply CSE when expression is actually reused.

---

### Issue 110: Boolean Operators Generate Wrong Code for Bools

**Affected Categories:** examples-stdlib  
**Failure Count:** ~10+

**Problem:** Boolean `and`/`or` generates `!= 0` checks for bool types.

**Examples:**
```rust
// Expected:
let and_result: bool = a && b;
let or_result: bool = a || b;

// Actual:
let and_result: bool = (a != 0) && (b != 0);  // a and b are already bool!
let or_result: bool = (a != 0) || (b != 0);
```

**Fix:** Don't add `!= 0` checks for boolean operands.

---

### Issue 111: Enum Auto-Value Not Mapped

**Affected Categories:** examples-stdlib, enum-basic  
**Failure Count:** ~20+

**Problem:** Enum `auto()` values generate `auto()` call instead of auto-incrementing integers.

**Examples:**
```rust
// Expected:
pub const PENDING: i32 = 1;
pub const APPROVED: i32 = 2;
pub const REJECTED: i32 = 3;

// Actual:
pub const PENDING: i32 = auto();  // Invalid
pub const APPROVED: i32 = auto();
```

**Fix:** Replace `auto()` with auto-incrementing integer values.

---

### Issue 112: String Format `.format()` Not Mapped

**Affected Categories:** functions  
**Failure Count:** ~15+

**Problem:** Python string `.format()` not mapped to Rust's `.replace()` chain.

**Examples:**
```rust
// Expected:
let template = "{name} is {age}";
return template.replace("{name}", "Alice").replace("{age}", "30");

// Actual:
let template = "{name} is {age}".to_string();
return format!("{}", template);  // Doesn't substitute!
```

**Fix:** Map `.format()` with named placeholders to `.replace()` calls.

---

### Issue 113: `dataclasses.asdict` Generates Invalid Call

**Affected Categories:** dataclasses-advanced  
**Failure Count:** ~10+

**Problem:** `dataclasses.asdict()` generates unresolved `asdict(p)` call.

**Examples:**
```rust
// Expected:
let d = p.as_dict();

// Actual:
let d = asdict(p);  // asdict not defined
```

**Fix:** Generate struct method `as_dict()` that returns HashMap.

---

### Issue 114: Division Generates Wrong Result Type

**Affected Categories:** constant-type-inference  
**Failure Count:** ~10+

**Problem:** Integer division `/` generates wrong result (should be float in Python).

**Examples:**
```rust
// Expected (Python semantics):
pub fn divide_ints() -> f64 {
    return 5 as f64 / 2 as f64;  // = 2.5
}

// Actual:
pub fn divide_ints() -> Result<f64, ZeroDivisionError> {
    return Ok(2);  // Wrong value and wrapped!
}
```

**Fix:** Use float division for `/` operator, integer division for `//`.

---

### Issue 115: Enum Type Hint Parameters Incorrect

**Affected Categories:** enum-advanced  
**Failure Count:** ~24

**Problem:** Functions taking enum parameters use `&Color` instead of `Color`, and `.name` is not a valid property.

**Examples:**
```rust
// Expected:
fn f(color: Color) -> String {
    format!("{:?}", color)
}

// Actual:
pub fn f(color: &Color) -> String {
    return color.name;  // .name doesn't exist!
}
```

**Fix:** Generate proper Rust enum with `format!("{:?}", ...)` for name.

---

### Issue 116: Dataclass Inheritance Missing Parent Fields

**Affected Categories:** dataclasses-frozen  
**Failure Count:** ~10+

**Problem:** Inherited dataclass structs don't include parent fields in constructor.

**Examples:**
```rust
// Expected (Point3D extends Point2D):
pub fn new(x: i32, y: i32, z: i32) -> Self { ... }

// Actual:
pub fn new(z: i32) -> Self { Self { z } }  // Missing x, y!
```

**Fix:** Include inherited fields in child struct and constructor.

---

### Issue 117: Dead Code Elimination Removes Used Variables

**Affected Categories:** optimizer-inlining  
**Failure Count:** ~5+

**Problem:** DCE incorrectly preserves unused variables instead of removing them.

**Examples:**
```rust
// Expected (no unused):
let temp = n * 2;
let result = temp + 5;

// Actual (unused preserved):
let temp = n * 2;
let unused = 100;  // Should be removed!
let result = temp + 5;
```

**Fix:** Properly identify and remove unused variables.

---

### Issue 118: Extra Reference Operator on Already-Reference Parameters

**Affected Categories:** interprocedural-mutation  
**Failure Count:** ~10+

**Problem:** Functions with reference parameters add extra `&` when calling.

**Examples:**
```rust
// Expected:
return if get_left(pair) > get_right(pair) { ... }

// Actual:
return if get_left(&pair) > get_right(&pair) { ... }  // Extra &
```

**Fix:** Don't add `&` if parameter is already a reference.

---

### Issue 119: Protocol/Trait Not Generated

**Affected Categories:** protocols  
**Failure Count:** ~11

**Problem:** Python Protocol classes don't generate Rust traits.

**Examples:**
```rust
// Expected:
pub trait Sized {
    fn size(&self) -> i32;
}

// Actual:
// No trait generated - just struct with method
pub struct Collection { ... }
impl Collection {
    pub fn size(&self) -> i32 { ... }
}
```

**Fix:** Generate trait definition for Protocol classes.

---

### Issue 120: `cached_property` Generates Invalid Code

**Affected Categories:** functools-cache  
**Failure Count:** ~10+

**Problem:** `@cached_property` generates invalid code with undefined variables.

**Examples:**
```rust
// Expected:
struct MyClass {
    prop: Lazy<i32>,
}

// Actual:
impl MyClass {
    pub fn prop(&self) -> i32 {
        {}
        call_count += 1;  // Undefined variable!
        return 42;
    }
}
```

**Fix:** Implement cached property with `once_cell::Lazy`.

---

### Issue 121: Complex Number Module Not Mapped

**Affected Categories:** complex-numbers  
**Failure Count:** ~24

**Problem:** Complex number operations generate empty output.

**Examples:**
```rust
// Expected:
use num::Complex;
let x = Complex::new(3.0, 4.0);

// Actual: (empty)
```

**Fix:** Map Python complex to `num::Complex` crate.

---

### Issue 122: Global Variable Write Without Atomic

**Affected Categories:** global-nonlocal  
**Failure Count:** ~19

**Problem:** Global variable writes generate invalid direct mutation.

**Examples:**
```rust
// Expected:
static COUNTER: AtomicI32 = AtomicI32::new(0);
fn increment() {
    COUNTER.fetch_add(1, Ordering::SeqCst);
}

// Actual:
pub fn increment() {
    counter += 1;  // Invalid - can't mutate global directly!
}
```

**Fix:** Use `AtomicI32` or `Mutex` for global mutable state.

---

### Issue 123: Asyncio Task API Not Mapped

**Affected Categories:** asyncio-tasks  
**Failure Count:** ~26

**Problem:** `asyncio.create_task` and `asyncio.wait` generate Python syntax.

**Examples:**
```rust
// Expected:
use tokio::task::JoinSet;
let mut set = JoinSet::new();
set.spawn(my_coro(1));

// Actual:
let task1 = asyncio.create_task(my_coro(1));  // Invalid!
let(done, pending) = asyncio.wait(...).await;  // Invalid!
```

**Fix:** Map asyncio task API to tokio JoinSet pattern.

---

### Issue 124: Class Decorator Generates Invalid Wrapper

**Affected Categories:** decorators-class  
**Failure Count:** ~15+

**Problem:** Class decorators generate Python-style wrapper functions.

**Examples:**
```rust
// Expected:
// #[decorator]
// fn f() -> i32 { 42 }

// Actual:
pub fn decorator(func: &serde_json::Value) {
    fn wrapper() -> () {
        return func(args);  // Invalid - args undefined!
    }
    return wrapper;  // Invalid - can't return fn
}
```

**Fix:** Document that decorators require proc_macro or remove wrapper.

---

### Issue 125: `__iter__`/`__next__` Dunder Methods Generated

**Affected Categories:** dunder-iteration  
**Failure Count:** ~24

**Problem:** Iterator classes generate Python dunder methods instead of implementing Iterator trait.

**Examples:**
```rust
// Expected:
impl Iterator for MyIterator {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> { ... }
}

// Actual:
impl MyIterator {
    pub fn __iter__(&self) -> &Self { return self; }
    pub fn __next__(&mut self) { ... panic!("StopIteration"); }
}
```

**Fix:** Implement Iterator trait, return `Option<T>` for end.

---

### Issue 126: Counter Operations Generate Empty Output

**Affected Categories:** counter  
**Failure Count:** ~28

**Problem:** Counter operations like `|` generate empty output.

**Examples:**
```rust
// Expected:
let mut result = c1.clone();
for (k, v2) in c2 {
    result.entry(k).and_modify(|v1| *v1 = (*v1).max(v2)).or_insert(v2);
}

// Actual: (only HashMap import, no code)
```

**Fix:** Generate Counter operation implementations.

---

### Issue 127: Slice Assignment Signature Wrong

**Affected Categories:** slicing  
**Failure Count:** ~15+

**Problem:** Slice assignment functions use `mut items` instead of `&mut items`.

**Examples:**
```rust
// Expected:
pub fn replace_middle(items: &mut Vec<i32>, ...) { ... }

// Actual:
pub fn replace_middle(mut items: Vec<i32>, ...) { ... }  // Takes ownership!
```

**Fix:** Use `&mut` for slice assignment functions.

---

### Issue 128: Async Comprehensions Generate Sync Iterator

**Affected Categories:** async-comprehensions  
**Failure Count:** ~10+

**Problem:** Async comprehensions generate synchronous iterator patterns.

**Examples:**
```rust
// Expected:
use futures::stream::StreamExt;
async let result: HashSet<i32> = stream::iter(0..5).collect().await;

// Actual (sync):
pub fn async_range(n: &Value) -> impl Iterator<Item = i32> { ... }
```

**Fix:** Use futures/tokio streams for async comprehensions.

---

### Issue 129: CSV Module Methods Not Fully Mapped

**Affected Categories:** examples-stdlib  
**Failure Count:** ~5+

**Problem:** CSV reader/writer methods like `.writerows()` not mapped to Rust csv crate.

**Examples:**
```rust
// Actual:
writer.writerows(data);  // Method doesn't exist!

// Expected:
for row in data {
    writer.write_record(row)?;
}
```

**Fix:** Map Python csv methods to rust csv crate equivalents.

---

### Issue 130: Enum Multi-Value Tuples Invalid

**Affected Categories:** enum-methods  
**Failure Count:** ~10+

**Problem:** Enum with tuple values generates invalid constant assignment.

**Examples:**
```rust
// Expected:
const JANUARY: Month = Month { number: 1, days: 31 };

// Actual:
pub const JANUARY: i32 = (1, 31);  // Can't assign tuple to i32!
```

**Fix:** Generate proper enum variants or associated constants with struct type.

---

### Issue 131: Error Previously Now Generates Code

**Affected Categories:** examples-algorithms  
**Failure Count:** ~5+

**Problem:** Previously "Error: Complex tuple unpacking not yet supported" now generates code, but with issues.

**Examples:**
```rust
// Actual (has issues):
return Ok(quicksort(left).iter()...+ quicksort(right));  // Can't add Vec with +
arr[i + 1 as usize] = ...  // Precedence wrong
```

**Fix:** Use `.extend()` or `.chain()` for Vec concatenation.

---

### Issue 132: Pattern Guards Generate If-Else Instead of Match

**Affected Categories:** parser-pattern-matching  
**Failure Count:** ~5+

**Problem:** Match expressions with guards (`if n < 0`) generate nested if-else instead of proper Rust match with guards.

**Examples:**
```rust
// Expected:
match x {
    n if n < 0 => "negative".to_string(),
    n if n > 0 => "positive".to_string(),
    _ => "zero".to_string(),
}

// Actual:
if x < 0 {
    return "negative".to_string();
} else {
    if x > 0 { ... }
}
```

**Fix:** Detect match with guard conditions and preserve match syntax with `if` guards.

---

### Issue 133: Match Or-Pattern with None Generates Empty

**Affected Categories:** match-or-patterns  
**Failure Count:** ~10+

**Problem:** Match expressions with `None | Some(0)` patterns generate empty output.

**Examples:**
```python
# Python:
match x:
    case None | Some(0): return True
    case _: return False
```
```rust
// Expected:
match x {
    None | Some(0) => true,
    _ => false,
}

// Actual:
<empty>
```

**Fix:** Handle or-patterns involving `None` in match expressions.

---

### Issue 134: Double `as i32` Cast Generated

**Affected Categories:** argparse, control-flow, result-types  
**Failure Count:** ~20+

**Problem:** Length conversions generate `as i32 as i32` double casts.

**Examples:**
```rust
// Actual:
return lines.len() as i32 as i32;
return text.matches(char).count() as i32 as i32;
return Ok(items.len() as i32 as i32);
```

**Fix:** Dedup consecutive identical casts in code generation.

---

### Issue 135: IntoIterator Trait Not Generated for `__iter__`

**Affected Categories:** dunder-iteration  
**Failure Count:** ~15+

**Problem:** Classes with `__iter__` method generate a method stub instead of implementing `IntoIterator` trait.

**Examples:**
```rust
// Expected:
impl IntoIterator for MyList {
    type Item = i32;
    type IntoIter = std::vec::IntoIter<i32>;
    fn into_iter(self) -> Self::IntoIter { ... }
}

// Actual:
pub fn __iter__(&self) {
    return iter(self.items);  // iter() doesn't exist
}
```

**Fix:** Detect `__iter__` dunder and generate `IntoIterator` implementation.

---

### Issue 136: OrderedDict Uses Non-Existent `std::collections::IndexMap`

**Affected Categories:** ordereddict  
**Failure Count:** ~10+

**Problem:** Generates `use std::collections::IndexMap` but IndexMap is from the `indexmap` crate, not stdlib.

**Examples:**
```rust
// Expected:
use indexmap::IndexMap;

// Actual:
use std::collections::IndexMap;  // Doesn't exist!
```

**Fix:** Use `indexmap::IndexMap` from external crate or add cargo dependency.

---

### Issue 137: Decorator Functions Return Inner Wrapper Incorrectly

**Affected Categories:** decorators-with-args  
**Failure Count:** ~15+

**Problem:** Decorator functions generate code that references `func` and `args` as undefined.

**Examples:**
```rust
// Actual:
pub fn decorator(func: &serde_json::Value) {
    fn wrapper() -> () {
        return func(args);  // func and args undefined in this scope
    }
    return wrapper;
}
```

**Fix:** Generate closures capturing func properly or convert to proc macro pattern.

---

### Issue 138: Floor Division Adds Spurious ZeroDivisionError

**Affected Categories:** modules-math  
**Failure Count:** ~10+

**Problem:** Simple floor division generates ZeroDivisionError type even when divisor is constant non-zero.

**Examples:**
```python
# Python:
VALUE = 23.34
def test():
    return VALUE // 2  # 2 is never zero
```
```rust
// Actual:
pub struct ZeroDivisionError { ... }  // Unnecessary
pub fn test() -> Result<(), ZeroDivisionError>  // Wrong return type
```

**Fix:** Don't add ZeroDivisionError when divisor is provably non-zero constant.

---

### Issue 139: Const Missing `pub` Visibility

**Affected Categories:** lazy-static-detection  
**Failure Count:** ~5+

**Problem:** Module-level constants sometimes missing `pub` visibility.

**Examples:**
```rust
// Expected:
pub const ENABLED: bool = true;

// Actual:
const ENABLED: bool = true;
```

**Fix:** Make module-level constants `pub` by default (configurable).

---

### Issue 140: Context Manager Drop Not Generated

**Affected Categories:** context-managers, dunder-context  
**Failure Count:** ~20+

**Problem:** Context managers with `__enter__`/`__exit__` generate method stubs instead of `Drop` trait.

**Examples:**
```rust
// Expected:
impl Drop for Tracker {
    fn drop(&mut self) { ... }
}

// Actual:
pub fn __enter__(&self) -> &Self { ... }
pub fn __exit__(&self, exc_type: serde_json::Value, ...) { ... }
```

**Fix:** Convert `__exit__` to `Drop::drop` implementation with RAII semantics.

---

### Issue 141: Descriptor Protocol Not Implemented

**Affected Categories:** descriptors  
**Failure Count:** ~15+

**Problem:** Python descriptors with `__get__`/`__set__` generate empty structs without the protocol methods.

**Examples:**
```rust
// Expected:
struct NamedDescriptor {
    name: &'static str,
    value: Cell<Option<i32>>,
}
impl NamedDescriptor {
    fn get(&self) -> Option<i32> { ... }
    fn set(&self, val: i32) { ... }
}

// Actual:
pub struct NamedDescriptor {}
```

**Fix:** Implement descriptor protocol mapping to getter/setter pattern.

---

### Issue 142: Nested Comprehension Loses Inner Variable

**Affected Categories:** comprehensions  
**Failure Count:** ~10+

**Problem:** Nested list comprehensions lose the inner loop variable.

**Examples:**
```python
# Python:
[[y for y in range(3)] for x in range(2)]
```
```rust
// Expected:
(0..2).map(|x| (0..3).map(|y| y).collect::<Vec<_>>())

// Actual:
(0..2).map(|x| (0..3).collect::<Vec<_>>())  // Missing |y| y
```

**Fix:** Preserve inner comprehension expression in nested comprehensions.

---

### Issue 143: Struct Missing Copy When Should Be Copy

**Affected Categories:** interprocedural-mutation  
**Failure Count:** ~10+

**Problem:** Simple struct with only i32 field gets `Copy` added inconsistently between nested structs.

**Examples:**
```rust
// Level3 gets Copy:
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Level3 { pub value: i32 }

// Level2 doesn't (but references Level3):
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Level2 { pub level3: Level3 }
```

**Fix:** Propagate Copy derivability analysis through struct references.

---

### Issue 144: File I/O Return Type Changed to Result

**Affected Categories:** file  
**Failure Count:** ~10+

**Problem:** Simple file read functions get their return type changed to Result even when expected output doesn't have Result.

**Examples:**
```rust
// Expected:
pub fn read_text_file(path: String) -> String { ... }

// Actual:
pub fn read_text_file(path: String) -> Result<String, std::io::Error> { ... }
```

**Fix:** Only add Result wrapper when function explicitly handles errors, or make consistent.

---

### Issue 145: Dict Comprehension Key Type Becomes serde_json::Value

**Affected Categories:** comprehensions  
**Failure Count:** ~5+

**Problem:** Dict comprehension with integer keys generates `HashMap<serde_json::Value, i32>`.

**Examples:**
```rust
// Expected:
pub fn make_dict_comp() -> HashMap<i32, i32>

// Actual:
pub fn make_dict_comp() -> HashMap<serde_json::Value, i32>
```

**Fix:** Infer proper key type from comprehension expression.

---

### Issue 146: Dict Literal in Const Generates Invalid Syntax

**Affected Categories:** dictionaries  
**Failure Count:** ~5+

**Problem:** Module-level dict constants generate code inside const which isn't valid.

**Examples:**
```rust
// Actual (invalid):
pub const config: serde_json::Value = {
    let mut map = HashMap::new();  // Can't do this in const
    map.insert(...);
    map
};
```

**Fix:** Use `lazy_static!` or `once_cell::Lazy` for complex const initialization.

---

### Issue 147: Multiple Inheritance with __slots__ Not Handled

**Affected Categories:** multiple-inheritance  
**Failure Count:** ~10+

**Problem:** Classes with multiple inheritance and `__slots__` generate simple struct without proper field handling.

**Examples:**
```rust
// Actual:
pub struct Example {
    pub value: serde_json::Value,  // Should be typed properly
}
```

**Fix:** Merge fields from parent classes when handling multiple inheritance.

---

### Issue 148: Enum with Custom Method Generates Struct

**Affected Categories:** enum-methods  
**Failure Count:** ~10+

**Problem:** Python enum with custom methods generates struct with constants instead of Rust enum.

**Examples:**
```rust
// Expected:
#[derive(Debug)]
enum Status { Pending = 1, Active = 2, Done = 3 }
impl Status { fn describe(&self) -> String { ... } }

// Actual:
pub struct Status {}
impl Status {
    pub const PENDING: i32 = 1;
    pub const ACTIVE: i32 = 2;
    pub fn describe(&self) { ... }  // Uses undefined self.name, self.value
}
```

**Fix:** Generate proper Rust enum with associated impl block.

---

### Issue 149: Raw String Regex Pattern Escaped Incorrectly

**Affected Categories:** raw-strings  
**Failure Count:** ~5+

**Problem:** Raw string regex patterns lose their raw-ness and get double-escaped.

**Examples:**
```rust
// Expected:
let pattern = r"\d+\.\d+";

// Actual:
pub const pattern: &str = "\\d+\\.\\d+";  // Double escaped
```

**Fix:** Preserve raw string prefix `r"..."` in generated code.

---

### Issue 150: Bytes Mutation Test Generates Invalid Code

**Affected Categories:** bytes-strings  
**Failure Count:** ~5+

**Problem:** Test for bytes immutability generates code that tries to mutate, which won't compile.

**Examples:**
```rust
// Actual:
let b = b"x";
b[0 as usize] = 1;  // Won't compile - bytes are immutable
return false;
```

**Fix:** Generate compile-time check or const assertion instead of runtime mutation attempt.

---

### Issue 151: Reflected Operators Not Implemented with Traits

**Affected Categories:** dunder-reflected  
**Failure Count:** ~15+

**Problem:** Reflected operators (`__radd__`, `__rmul__`, `__rtruediv__`, `__rand__`) don't generate trait impls.

**Examples:**
```rust
// Expected:
impl Div<Number> for i32 {
    type Output = Number;
    fn div(self, other: Number) -> Number { ... }
}

// Actual:
pub struct Number { pub value: serde_json::Value }
// No trait impl
```

**Fix:** Generate trait implementation for reversed operand types.

---

### Issue 152: Metaclass `__new__` Not Transpiled to Factory Pattern

**Affected Categories:** metaclasses  
**Failure Count:** ~15+

**Problem:** Metaclasses with `__new__` generate empty structs instead of factory patterns or traits.

**Examples:**
```rust
// Expected:
impl Default for MyClass { fn default() -> Self { MyClass { added_by_meta: true } } }

// Actual:
pub struct Meta {}
pub struct MyClass {}
```

**Fix:** Convert metaclass pattern to builder/factory or derive macro pattern.

---

### Issue 153: Comparison Dunders Not Generating PartialOrd

**Affected Categories:** dunder-comparison  
**Failure Count:** ~20+

**Problem:** Classes with `__le__`, `__lt__`, etc. don't generate `PartialOrd` trait.

**Examples:**
```rust
// Expected:
#[derive(PartialEq, PartialOrd)]
struct Number { x: i32 }

// Actual:
pub struct Number { pub x: serde_json::Value }
// No PartialOrd
```

**Fix:** Map comparison dunders to PartialOrd/Ord trait implementations.

---

### Issue 154: contextlib.contextmanager Generates Invalid Iterator

**Affected Categories:** contextlib-decorator  
**Failure Count:** ~10+

**Problem:** `@contextmanager` decorated generators produce iterator pattern instead of Drop pattern.

**Examples:**
```rust
// Actual:
struct TrackExceptionState { state: usize }
pub fn track_exception() -> impl Iterator<Item = serde_json::Value>  // Not useful
```

**Fix:** Convert `@contextmanager` to RAII guard pattern with Drop.

---

### Issue 155: Type Guard Unwrap Without Clone on Copy Type

**Affected Categories:** type-guards  
**Failure Count:** ~5+

**Problem:** Optional narrowing uses `unwrap()` on `&Option<i32>` which moves, not `.clone()` or `*` deref.

**Examples:**
```rust
// Expected:
return value.as_ref().unwrap().clone();

// Actual:
return value.unwrap();  // Error: cannot move out of `*value`
```

**Fix:** Use `.as_ref().unwrap().clone()` or `*value.as_ref().unwrap()` for Copy types.

---

### Issue 156: Leap Year Logic Wraps in Unnecessary Result

**Affected Categories:** datetime-edge-cases  
**Failure Count:** ~5+

**Problem:** Simple modulo-based leap year check generates Result<bool, ZeroDivisionError>.

**Examples:**
```rust
// Expected:
pub fn is_leap_year(year: i32) -> bool

// Actual:
pub fn is_leap_year(year: i32) -> Result<bool, ZeroDivisionError>  // % 400, % 100 can't divide by zero
```

**Fix:** Don't generate ZeroDivisionError for modulo with constant non-zero operands.

---

### Issue 157: bytes.decode() Not Transpiled

**Affected Categories:** bytes-strings  
**Failure Count:** ~5+

**Problem:** `b"hello".decode("utf-8")` generates literal Python syntax instead of Rust.

**Examples:**
```rust
// Expected:
String::from_utf8(b"hello".to_vec()).unwrap()

// Actual:
return b"hello".decode("utf-8");  // Python syntax in Rust
```

**Fix:** Map `.decode()` to `String::from_utf8()`.

---

### Issue 158: partialmethod Generates Invalid Import

**Affected Categories:** functools-partial  
**Failure Count:** ~5+

**Problem:** `functools.partialmethod` generates `use std::partialmethod` which doesn't exist.

**Examples:**
```rust
// Actual:
use std::partialmethod;  // Doesn't exist
pub const add_one: serde_json::Value = partialmethod(add, 1);
```

**Fix:** Implement partialmethod as a regular method delegating to the target.

---

### Issue 159: Template String Substitute Generates Empty

**Affected Categories:** template-strings  
**Failure Count:** ~5+

**Problem:** `string.Template` usage generates empty output.

**Examples:**
```python
# Python:
from string import Template
t = Template("Hello $name")
result = t.substitute(name="World")
```
```rust
// Actual:
<empty>
```

**Fix:** Map Template.substitute to format! with named arguments.

---

### Issue 160: str.split with Maxsplit Generates Empty

**Affected Categories:** string-methods-all  
**Failure Count:** ~5+

**Problem:** `str.split(sep, maxsplit)` doesn't generate `splitn()`.

**Examples:**
```python
# Python:
"a,b,c,d".split(",", 2)  # Returns ['a', 'b', 'c,d']
```
```rust
// Expected:
"a,b,c,d".splitn(3, ",").collect::<Vec<_>>()

// Actual:
<empty>
```

**Fix:** Map split with maxsplit to `splitn(maxsplit + 1, sep)`.

---

### Issue 161: Counter Empty Initialization Missing Variable

**Affected Categories:** counter  
**Failure Count:** ~5+

**Problem:** Empty Counter initialization loses the variable assignment.

**Examples:**
```rust
// Expected:
let c: HashMap<String, usize> = HashMap::new();

// Actual:
use std::collections::HashMap;
// Missing variable declaration
```

**Fix:** Preserve variable binding for empty Counter creation.

---

### Issue 162: Augmented Division Generates Empty

**Affected Categories:** augmented-assignment  
**Failure Count:** ~10+

**Problem:** `/=` augmented assignment generates empty output.

**Examples:**
```python
# Python:
x = 10
x /= 4
```
```rust
// Expected:
let mut x = 10;
x /= 4;

// Actual:
<empty>
```

**Fix:** Implement DivAssign for `/=` operator.

---

### Issue 163: Metaclass `__instancecheck__` Not Mapped to Trait Bound

**Affected Categories:** metaclasses  
**Failure Count:** ~5+

**Problem:** Metaclass with `__instancecheck__` generates empty structs instead of trait bounds.

**Examples:**
```rust
// Expected:
trait HasSpecialAttr { ... }
fn is_compatible<T: HasSpecialAttr>(_: &T) -> bool { true }

// Actual:
pub struct InstanceCheckMeta {}
pub struct MyClass {}
// No trait
```

**Fix:** Map `__instancecheck__` to trait-based type checking pattern.

---

### Issue 164: Bitwise Int + Custom Type Missing Trait Impl

**Affected Categories:** dunder-bitwise  
**Failure Count:** ~10+

**Problem:** `int & CustomType` pattern doesn't generate `BitAnd<CustomType> for i32`.

**Examples:**
```rust
// Expected:
impl BitAnd<CustomFlags> for i32 {
    type Output = CustomFlags;
    fn bitand(self, other: CustomFlags) -> CustomFlags { ... }
}

// Actual:
pub struct CustomFlags { pub value: serde_json::Value }
// No trait impl
```

**Fix:** Generate reversed trait impl for int + custom type operations.

---

### Issue 165: String Parameter Type &str vs String Inconsistent

**Affected Categories:** cse-optimization  
**Failure Count:** ~10+

**Problem:** String comparison functions sometimes expect `&str`, sometimes `String`.

**Examples:**
```rust
// Expected:
pub fn check_not_done(status: &str) -> bool

// Actual:
pub fn check_not_done(status: String) -> bool
```

**Fix:** Prefer `&str` for read-only string parameters.

---

### Issue 166: Class Method from_* Factory Not Returning Self

**Affected Categories:** class-methods  
**Failure Count:** ~5+

**Problem:** Factory classmethod like `from_birth_year` doesn't have return type annotation.

**Examples:**
```rust
// Expected:
pub fn from_birth_year(name: String, year: i32) -> Self { ... }

// Actual:
pub fn from_birth_year(name: serde_json::Value, year: i32) {  // No return type
    let mut age = 2023 - year;
    return Self::new(name);
}
```

**Fix:** Add `-> Self` return type to factory classmethods.

---

### Issue 167: `__init_subclass__` Chain Not Implemented

**Affected Categories:** init-subclass  
**Failure Count:** ~5+

**Problem:** `__init_subclass__` inheritance chain generates separate structs, not trait inheritance.

**Examples:**
```rust
// Expected:
trait A { fn init() -> Vec<&'static str> { vec!["A"] } }
trait B: A { fn init() -> Vec<&'static str> { ... } }

// Actual:
pub struct A { pub const chain: Vec<serde_json::Value> = vec![] }
pub struct B {}
```

**Fix:** Map `__init_subclass__` pattern to trait inheritance with super calls.

---

### Issue 168: Dataclass Field with Callable Factory Type

**Affected Categories:** dataclasses-field  
**Failure Count:** ~5+

**Problem:** Dataclass field with `default_factory=MyClass` doesn't use `Default::default()` properly.

**Examples:**
```rust
// Expected:
impl Default for Container {
    fn default() -> Self { Self { obj: MyClass::default() } }
}

// Actual:
pub fn new() -> Self { Self { obj: Default::default() } }  // Works but inconsistent
```

**Fix:** Generate consistent Default impl using field's factory.

---

### Issue 169: Ellipsis Truthiness Test Generates Empty

**Affected Categories:** ellipsis  
**Failure Count:** ~5+

**Problem:** Test for ellipsis truthiness generates empty output.

**Examples:**
```python
# Python:
if ...:
    print("truthy")
```
```rust
// Expected:
if true { log::info!("truthy"); }

// Actual:
<empty>
```

**Fix:** Treat `...` (ellipsis) as truthy constant `true`.

---

### Issue 170: Dynamic Import find_spec Generates Empty

**Affected Categories:** imports-dynamic  
**Failure Count:** ~5+

**Problem:** `importlib.util.find_spec()` generates empty output.

**Examples:**
```python
# Python:
import importlib.util
spec = importlib.util.find_spec('os')
```
```rust
// Actual:
<empty>
```

**Fix:** Generate compile-time check comment or cfg-based availability check.

---

### Issue 171: Try-Except Dead Code After Return

**Affected Categories:** exceptions  
**Failure Count:** ~15+

**Problem:** Try-except blocks have dead code after return statements.

**Examples:**
```rust
// Actual:
{
    let ratio = (a as f64) / (b as f64);
    return ratio;
    return 0.0;  // Dead code - unreachable
}
```

**Fix:** DCE should remove code after unconditional return.

---

### Issue 172: Multiple Except Clauses Generate Sequential Returns

**Affected Categories:** exceptions  
**Failure Count:** ~10+

**Problem:** Multiple except clause types generate sequential return statements instead of match arms.

**Examples:**
```rust
// Actual:
if let Ok(__parse_result) = data.parse::<i32>() {
    __parse_result
} else {
    return -1;
    return -2;  // Both handlers executed?
}
```

**Fix:** Generate proper match/if-else for distinct exception types.

---

### Issue 173: Walrus Operator (`:=`) Generates Empty Output

**Affected Categories:** walrus-operator  
**Failure Count:** ~3

**Problem:** Assignment expressions using the walrus operator generate empty output instead of extracting the assignment.

**Examples:**
```python
# Python
items = [1, 2, 3, 4, 5]
if (n := len(items)) > 3:
    print(f"List has {n} items")
```
```rust
// Expected:
let items = vec![1, 2, 3, 4, 5];
let n = items.len() as i32;
if n > 3 {
    log::info!("{}", format!("List has {} items, which is more than 3", n));
}

// Actual:
<empty>
```

**Fix:** Extract walrus operator to separate `let` binding before the condition.

---

### Issue 174: Deferred Initialization Incorrectly Marked `mut`

**Affected Categories:** mutable-variable-detection  
**Failure Count:** ~5+

**Problem:** Variables with deferred initialization (assigned in if/else branches) are incorrectly marked as `mut` when they're only assigned once.

**Examples:**
```rust
// Expected:
pub fn choose(cond: bool) -> i32 {
    let x;  // No mut needed
    if cond { x = 1; }
    else { x = 2; }
    return x;
}

// Actual:
pub fn choose(cond: bool) -> i32 {
    let mut x;  // Incorrectly marked mut
    ...
}
```

**Fix:** Distinguish deferred initialization from reassignment.

---

### Issue 175: For Loop Iterator Variable Naming (`player` vs `_player`)

**Affected Categories:** for-loop-iterator-mutation  
**Failure Count:** ~10+

**Problem:** Loop variables are prefixed with `_` when they're actually used, or `.iter()` vs `&collection` inconsistency.

**Examples:**
```rust
// Expected:
for player in state.players.iter() {
    count += 1;
}

// Actual:
for _player in &state.players {  // Wrong variable name
    count += 1;
}
```

**Fix:** Don't prefix with `_` unless variable is truly unused; be consistent with iterator style.

---

### Issue 176: Deeply Nested Attribute Access Changes Parameter Type

**Affected Categories:** param-attribute-access  
**Failure Count:** ~10+

**Problem:** Functions with deeply nested attribute access on parameters generate generic type parameters instead of concrete type references.

**Examples:**
```rust
// Expected:
pub fn get_deep(a: &A) -> i32 {
    return a.b.c.d.value;
}

// Actual:
pub fn get_deep<A: Clone>(a: A) -> i32 {  // Wrong type parameter
    return a.b.c.d.value;
}
```

**Fix:** Preserve concrete parameter types for attribute access.

---

### Issue 177: HashMap Literal Uses Block Syntax Instead of Collector

**Affected Categories:** conditional-imports, import-deduplication  
**Failure Count:** ~15+

**Problem:** HashMap initialization uses multi-line block syntax instead of more idiomatic `.into_iter().collect()`.

**Examples:**
```rust
// Expected:
let a: HashMap<String, i32> = [("x".to_string(), 1)].into_iter().collect();

// Actual:
let a = {
    let mut map = HashMap::new();
    map.insert("x".to_string(), 1);
    map
};
```

**Fix:** Use collector pattern for HashMap initialization from literals.

---

### Issue 178: `Vec::new()` vs `vec![]` Inconsistency

**Affected Categories:** trait-impls  
**Failure Count:** ~10+

**Problem:** Actual output uses `vec![]` while expected uses `Vec::new()` for empty vector initialization.

**Examples:**
```rust
// Expected:
Self { data: Vec::new() }

// Actual:
Self { data: vec![] }
```

**Fix:** Normalize to one style (both are valid but tests expect consistency).

---

### Issue 179: Copy Derive Missing for Simple Structs with Copy Fields

**Affected Categories:** param-attribute-access  
**Failure Count:** ~10+

**Problem:** Structs with only Copy fields (like `i32`) don't consistently get `#[derive(Copy)]`.

**Examples:**
```rust
// Expected:
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct D { pub value: i32 }

// Actual:
#[derive(Debug, Clone, PartialEq, Default)]
pub struct D { pub value: i32 }
```

**Fix:** Automatically derive Copy when all fields are Copy types.

---

### Issue 180: String Constant Uses `lazy_static!` Instead of `const &str`

**Affected Categories:** multi-module  
**Failure Count:** ~5+

**Problem:** Simple string constants are wrapped in `lazy_static!` when `const &str` would suffice.

**Examples:**
```rust
// Expected:
pub const VERSION: &str = "1.0.0";

// Actual:
lazy_static! {
    pub static ref VERSION: String = "1.0.0";
}
```

**Fix:** Use `const &str` for simple string literals.

---

### Issue 181: Extra `;` After Function Definition Braces

**Affected Categories:** custom-attributes  
**Failure Count:** ~10+

**Problem:** Functions generate trailing semicolon after closing brace and sometimes include extra code fragments.

**Examples:**
```rust
// Expected:
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Actual:
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
};
        let result1 = add(a.clone(), b.clone());
        ...
```

**Fix:** Remove trailing semicolons after function braces; ensure no test code is emitted with function.

---

### Issue 182: File I/O Return Type Inconsistency (`String` vs `Result<String, Error>`)

**Affected Categories:** io-streams  
**Failure Count:** ~10+

**Problem:** File reading functions have inconsistent return types between expected and actual.

**Examples:**
```rust
// Expected:
pub fn process_file(filepath: String) -> String

// Actual:
pub fn process_file(filepath: String) -> Result<String, std::io::Error>
```

**Fix:** Make consistent choice about whether to wrap I/O operations in Result.

---

### Issue 183: Borrow Conflict Resolution Uses Wrong Mutability

**Affected Categories:** root-test-files, ownership  
**Failure Count:** ~15+

**Problem:** Functions with borrow conflicts generate incorrect parameter mutability or add unnecessary reference operators.

**Examples:**
```rust
// Expected:
pub fn process_play(state: &mut State, player_index: i32) {
    assign_try(state, player_index, &state.team_in_possession.clone());
}

// Actual:
pub fn process_play(state: &mut State, player_index: &i32) {  // Wrong: &i32 instead of i32
    assign_try(&state, player_index, &state.team_in_possession);  // Missing .clone()
}
```

**Fix:** Use correct parameter mutability; clone fields when needed to avoid borrow conflicts.

---

### Issue 184: Augmented Assignment Not Applied (`+=` generates `= ... + ...`)

**Affected Categories:** root-test-files, regressions  
**Failure Count:** ~10+

**Problem:** Augmented assignment operators like `+=` are expanded to full assignment instead of using compound assignment.

**Examples:**
```rust
// Expected:
state.score += 5;

// Actual:
state.score = state.score + 5;
```

**Fix:** Use compound assignment operators where available.

---

### Issue 185: Generator State Variable Not Initialized Before Use

**Affected Categories:** generators, generator-liveness  
**Failure Count:** ~10+

**Problem:** Generator state variables are not initialized before being returned in the first yield.

**Examples:**
```rust
// Expected (state 0 initializes and returns):
0 => {
    self.x = 1;
    self.state = 1usize;
    return Some(self.x);
}

// Actual (missing initialization):
0 => {
    self.state = 1usize;
    return Some(self.x);  // self.x not initialized!
}
```

**Fix:** Ensure generator state variables are initialized before first use.

---

### Issue 186: argparse/clap Parser Generates Invalid Constant

**Affected Categories:** argparse  
**Failure Count:** ~25

**Problem:** ArgumentParser generates `clap::Parser()` as a const value instead of proper parser setup.

**Examples:**
```rust
// Expected:
use clap::Parser;
// (parser setup with Args struct)

// Actual:
use clap::Parser;
pub const parser: serde_json::Value = clap::Parser();  // Invalid
```

**Fix:** Generate proper clap Args struct with derive macros.

---

### Issue 187: `list()` Call Not Mapped to Vector Clone

**Affected Categories:** trait-impls  
**Failure Count:** ~5+

**Problem:** Python `list(x)` for copying a list generates `list(self.data)` in Rust instead of `.clone()` or `.to_vec()`.

**Examples:**
```rust
// Expected:
new.data = self.data.clone();

// Actual:
new.data = list(self.data);  // list() doesn't exist in Rust
```

**Fix:** Map `list()` copy to `.clone()` or `.to_vec()`.

---

### Issue 188: JSON Nested Access Missing Return Type

**Affected Categories:** modules-json  
**Failure Count:** ~10+

**Problem:** Functions accessing nested JSON values have missing or inconsistent return type annotations.

**Examples:**
```rust
// Expected:
pub fn get_nested_value(...) -> Result<i32, IndexError>

// Actual:
pub fn get_nested_value(...) {  // Missing return type
    ...
}
```

**Fix:** Infer and generate proper return types for JSON access functions.

---

### Issue 189: Unnecessary Error Type Generation for Simple Functions

**Affected Categories:** resource-management, copy-type-semantics  
**Failure Count:** ~20+

**Problem:** Simple functions that don't raise exceptions still generate error types like `ValueError` or `IndexError`.

**Examples:**
```rust
// Actual (unnecessary):
#[derive(Debug, Clone)]
pub struct ValueError { message: String }
impl std::fmt::Display for ValueError { ... }
impl std::error::Error for ValueError {}

pub fn cleanup_with_error(should_fail: bool) -> bool {
    if should_fail { return false; }
    return true;
}  // Function never uses ValueError!
```

**Fix:** Only generate error types when they're actually used.

---

## Action Plan

### Phase 1: Critical (Blocks Basic Usage)
1. Fix `serde_json::Value` fallback - add proper type inference
2. Fix module-level statement wrapping
3. Fix ownership/borrowing analysis
4. Fix return type inference (Issue 41)
5. Fix function callback parameters (Issue 45)

### Phase 2: High Priority (Common Features)
6. Implement proper ABC → trait transpilation
7. Fix enum generation
8. Complete async/await support
9. Fix context manager → Drop/RAII
10. Fix Protocol → trait with `impl Trait` syntax (Issue 33)
11. Fix string parameter ownership (`&str` vs `String`) (Issue 36)
12. Implement operator trait mappings (Issue 46)
13. Implement Display/Debug trait mappings (Issue 47)
14. Fix default parameter handling (Issue 44)

### Phase 3: Medium Priority (Extended Features)
15. Complete error handling → Result mapping
16. Fix dict type inference
17. Complete dunder method mappings
18. Implement match `as` pattern binding
19. Implement unpacking/destructuring
20. Implement global/nonlocal handling (Issue 31)
21. Fix `__slots__` handling (Issue 32)
22. Fix Result return type consistency (Issue 38)
23. Fix Option/Result double-wrapping (Issue 54)
24. Fix unnecessary error type generation (Issue 53)

### Phase 4: Polish
25. Standard library completeness
26. Formatting consistency
27. CSE optimization refinement
28. Remove redundant clones
29. Dead code elimination (Issue 39)
30. Remove double type casts (Issue 40)
31. Float precision handling (Issue 37)
32. Fix unwrap_or_default consistency (Issue 49)
33. Fix return keyword consistency (Issue 50)
34. Preserve numeric literal formats (Issue 51)
35. Fix semicolon/brace issues (Issue 52)
36. Template string support (Issue 48)

### Phase 5: Advanced/Unsupported Features (Document)
37. Descriptors (Issue 34)
38. `__init_subclass__` (Issue 35)
39. Metaclasses (Issue 13)
40. Ellipsis handling (Issue 42)
41. Python 3.11+ Self type (Issue 43)
42. Fix positional-only parameter parsing (Issue 55)
43. Implement varargs (`*args`) support (Issue 56)
44. Preserve type parameter order (Issue 57)
45. Fix empty Vec element type inference (Issue 58)
46. Fix constant string extraction naming (Issue 59)
47. Use `.copied()` vs `.cloned()` correctly (Issue 60)
48. Fix async generator formatting (Issue 61)
49. Optimize floor division for constant divisors (Issue 62)
50. Handle module-level code empty output (Issue 63)
51. Fix bytes literal contains check syntax (Issue 64)
52. Implement async stream libraries for async generators (Issue 65)
53. Fix ChainMap to use custom implementation (Issue 66)
54. Fix namedtuple to generate struct (Issue 67)
55. Ensure collection method bodies are generated (Issue 68)
56. Fix missing bracket in derive macro (Issue 69)
57. Use constructor parameters correctly (Issue 70)
58. Map `bytes` to `Vec<u8>` (Issue 71)
59. Fix context manager to use RAII pattern (Issue 72)
60. Handle `@contextmanager` decorator properly (Issue 73)
61. Map `sys.getsizeof` to `std::mem::size_of` (Issue 74)
62. Format async iterator code properly (Issue 75)
63. Remove double `.clone()` calls (Issue 76)
64. Remove double type casts (Issue 77)
65. Fix index expression operator precedence (Issue 78)
66. Use `lazy_static!` for Vec constants (Issue 79)
67. Generate trait for ABC classes (Issue 80)
68. Generate string method implementations (Issue 81)
69. Handle assert statements (Issue 82)
70. Fix `nonlocal` with Cell/RefCell (Issue 83)
71. Implement functools.cache memoization (Issue 84)
72. Fix exception handler control flow (Issue 85)
73. Support match guards (Issue 86)
74. Map PathBuf methods correctly (Issue 87)
75. Preserve HashMap type parameters (Issue 88)
76. Add `pub` to exported constants (Issue 89)
77. Fix field alias clone vs reference (Issue 90)
78. Add explicit return statements (Issue 91)
79. Remove unnecessary lifetime parameters (Issue 92)
80. Remove `__match_args__` from structs (Issue 93)
81. Map `__call__` to Fn traits or call method (Issue 94)
82. Fix bytes slicing return type (Issue 95)
83. Handle dataclass `compare=False` fields (Issue 96)
84. Fix CLI HashMap access patterns (Issue 97)
85. Map async generator protocol methods (Issue 98)
86. Preserve try/finally block structure (Issue 99)
87. Preserve numeric underscore separators (Issue 100)
88. Use Drop trait for context managers (Issue 101)
89. Generate factory methods for default params (Issue 102)
90. Respect visibility conventions (Issue 103)
91. Remove trailing semicolons after blocks (Issue 104)
92. Use identity checks for `is`/`is not` (Issue 105)
93. Support match or-patterns (Issue 106)
94. Fix descriptor getter/setter generation (Issue 107)
95. Define thread-local RNG or use thread_rng (Issue 108)
96. Optimize CSE to avoid unnecessary temps (Issue 109)
97. Don't add `!= 0` for bool operands (Issue 110)
98. Replace enum `auto()` with integers (Issue 111)
99. Map `.format()` to `.replace()` chain (Issue 112)
100. Generate `as_dict()` method for dataclasses (Issue 113)
101. Fix division operator semantics (Issue 114)
102. Fix enum type hint parameter handling (Issue 115)
103. Include parent fields in dataclass inheritance (Issue 116)
104. Fix DCE to actually remove unused variables (Issue 117)
105. Don't add extra `&` for reference params (Issue 118)
106. Generate trait for Protocol classes (Issue 119)
107. Implement cached_property with Lazy (Issue 120)
108. Map complex numbers to num::Complex (Issue 121)
109. Use AtomicI32 for global writes (Issue 122)
110. Map asyncio task API to tokio JoinSet (Issue 123)
111. Fix class decorator wrapper generation (Issue 124)
112. Implement Iterator trait for `__iter__` (Issue 125)
113. Generate Counter operation implementations (Issue 126)
114. Use `&mut` for slice assignment (Issue 127)
115. Use async streams for async comprehensions (Issue 128)
116. Map CSV module methods (Issue 129)
117. Fix enum multi-value tuple constants (Issue 130)
118. Fix Vec concatenation with chain/extend (Issue 131)
119. Preserve match with guard syntax (Issue 132)
120. Handle match or-patterns with None (Issue 133)
121. Dedup consecutive `as i32 as i32` casts (Issue 134)
122. Generate IntoIterator for `__iter__` (Issue 135)
123. Fix IndexMap import path (Issue 136)
124. Fix decorator closure capture (Issue 137)
125. Skip ZeroDivisionError for constant divisors (Issue 138)
126. Add pub to module-level constants (Issue 139)
127. Generate Drop for context managers (Issue 140)
128. Implement descriptor protocol (Issue 141)
129. Preserve inner variable in nested comprehensions (Issue 142)
130. Propagate Copy derivability (Issue 143)
131. Consistent Result wrapper for file I/O (Issue 144)
132. Fix dict comprehension key type inference (Issue 145)
133. Use lazy_static for dict constants (Issue 146)
134. Handle multiple inheritance slots (Issue 147)
135. Generate proper enum for enum with methods (Issue 148)
136. Preserve raw string prefix (Issue 149)
137. Fix bytes immutability test generation (Issue 150)
138. Implement reflected operator traits (Issue 151)
139. Map metaclass to factory pattern (Issue 152)
140. Generate PartialOrd for comparison dunders (Issue 153)
141. Convert @contextmanager to RAII guard (Issue 154)
142. Fix Optional unwrap for Copy types (Issue 155)
143. Skip ZeroDivisionError for modulo constants (Issue 156)
144. Map bytes.decode() to from_utf8 (Issue 157)
145. Implement partialmethod as delegation (Issue 158)
146. Map Template.substitute to format! (Issue 159)
147. Map split(maxsplit) to splitn (Issue 160)
148. Preserve Counter variable binding (Issue 161)
149. Implement /= augmented assignment (Issue 162)
150. Map __instancecheck__ to trait bounds (Issue 163)
151. Generate reversed BitAnd trait (Issue 164)
152. Prefer &str for string params (Issue 165)
153. Add return type to factory classmethods (Issue 166)
154. Map __init_subclass__ to trait inheritance (Issue 167)
155. Use factory for dataclass field defaults (Issue 168)
156. Treat ellipsis as truthy (Issue 169)
157. Handle importlib.util.find_spec (Issue 170)
158. Remove dead code after return in try-except (Issue 171)
159. Fix multiple except clause handling (Issue 172)
160. Extract walrus operator to let binding (Issue 173)
161. Fix deferred initialization mut detection (Issue 174)
162. Fix iterator variable naming convention (Issue 175)
163. Preserve concrete parameter types (Issue 176)
164. Use collector pattern for HashMap literals (Issue 177)
165. Normalize Vec::new() vs vec![] style (Issue 178)
166. Derive Copy for structs with Copy fields (Issue 179)
167. Use const &str for simple string constants (Issue 180)
168. Remove trailing semicolons after functions (Issue 181)
169. Consistent file I/O return type (Issue 182)
170. Fix borrow conflict resolution (Issue 183)
171. Use compound assignment operators (Issue 184)
172. Initialize generator state before use (Issue 185)
173. Generate proper clap Args struct (Issue 186)
174. Map list() copy to clone/to_vec (Issue 187)
175. Generate return types for JSON access (Issue 188)
176. Only generate error types when used (Issue 189)

---

## Files to Investigate

Key source files that likely need modification:

1. **Type Inference**: Look for type resolution/inference modules
2. **Code Generation**: Look for AST → Rust code generation
3. **Module Mapping**: Look for Python → Rust module mapping config
4. **Ownership Analysis**: Look for borrow checker/liveness analysis
5. **Decorator Handling**: Look for decorator expansion logic

---

## Top 50 Failure Categories (by count)

| # | Category | Count |
|---|----------|-------|
| 1 | examples-stdlib | 72 |
| 2 | control-flow | 55 |
| 3 | string-methods-all | 51 |
| 4 | error-handling | 50 |
| 5 | type-inference | 47 |
| 6 | optional-types | 46 |
| 7 | set-operations | 40 |
| 8 | dictionaries | 37 |
| 9 | functions | 32 |
| 10 | deque | 31 |
| 11 | classes | 31 |
| 12 | operators | 30 |
| 13 | fstrings | 28 |
| 14 | dataclasses-field | 28 |
| 15 | counter | 28 |
| 16 | chainmap | 27 |
| 17 | bytes-strings | 26 |
| 18 | asyncio-tasks | 26 |
| 19 | namedtuple | 25 |
| 20 | enum-basic | 25 |
| 21 | dunder-attribute | 25 |
| 22 | dataclasses-basic | 25 |
| 23 | dataclasses-advanced | 25 |
| 24 | cse-optimization | 25 |
| 25 | bytes | 25 |
| 26 | argparse | 25 |
| 27 | enum-advanced | 24 |
| 28 | dunder-iteration | 24 |
| 29 | dunder-context | 24 |
| 30 | complex-numbers | 24 |
| 31 | collections | 24 |
| 32 | modules-random | 23 |
| 33 | dunder-format | 23 |
| 34 | dataclasses-comparison | 23 |
| 35 | contextlib-utilities | 23 |
| 36 | template-strings | 22 |
| 37 | regressions | 22 |
| 38 | ordereddict | 22 |
| 39 | functools-wraps | 22 |
| 40 | functools-partial | 22 |
| 41 | functools-cache | 22 |
| 42 | exceptions | 22 |
| 43 | enum-methods | 22 |
| 44 | dunder-container | 22 |
| 45 | binary-literals | 22 |
| 46 | async-generators | 22 |
| 47 | raw-strings | 21 |
| 48 | packages-all | 21 |
| 49 | namespace-packages | 21 |
| 50 | memoryview | 21 |

---

## Complete Failure Category List (130 categories)

| Category | Count | | Category | Count |
|----------|-------|---|----------|-------|
| examples-stdlib | 72 | | dunder-reflected | 21 |
| control-flow | 55 | | decorators-basic | 21 |
| string-methods-all | 51 | | dataclasses-frozen | 21 |
| error-handling | 50 | | await-expressions | 21 |
| type-inference | 47 | | augmented-assignment | 21 |
| optional-types | 46 | | assignment | 21 |
| set-operations | 40 | | ternary-expressions | 20 |
| dictionaries | 37 | | multiple-inheritance | 20 |
| functions | 32 | | match-class-patterns | 20 |
| deque | 31 | | imports-from | 20 |
| classes | 31 | | dunder-lifecycle | 20 |
| operators | 30 | | dunder-inplace | 20 |
| fstrings | 28 | | dunder-callable | 20 |
| dataclasses-field | 28 | | dunder-arithmetic | 20 |
| counter | 28 | | decorators-class | 20 |
| chainmap | 27 | | contextlib-decorator | 20 |
| bytes-strings | 26 | | async-with | 20 |
| asyncio-tasks | 26 | | async-for | 20 |
| namedtuple | 25 | | string-operations | 19 |
| enum-basic | 25 | | parenthesized-context | 19 |
| dunder-attribute | 25 | | packages-init | 19 |
| dataclasses-basic | 25 | | match-guards | 19 |
| dataclasses-advanced | 25 | | match-as-patterns | 19 |
| cse-optimization | 25 | | imports-basic | 19 |
| bytes | 25 | | global-nonlocal | 19 |
| argparse | 25 | | exception-groups | 19 |
| enum-advanced | 24 | | dunder-comparison | 19 |
| dunder-iteration | 24 | | decorators-with-args | 19 |
| dunder-context | 24 | | context-managers | 19 |
| complex-numbers | 24 | | class-methods | 19 |
| collections | 24 | | assert-statement | 19 |
| modules-random | 23 | | abc | 19 |
| dunder-format | 23 | | lists-arrays | 18 |
| dataclasses-comparison | 23 | | enum-int | 18 |
| contextlib-utilities | 23 | | dunder-unary | 18 |
| template-strings | 22 | | dunder-bitwise | 18 |
| regressions | 22 | | boundary-values | 18 |
| ordereddict | 22 | | async-functions | 18 |
| functools-wraps | 22 | | slots | 17 |
| functools-partial | 22 | | properties | 17 |
| functools-cache | 22 | | parser-pattern-matching | 17 |
| exceptions | 22 | | ownership | 17 |
| enum-methods | 22 | | imports-relative | 17 |
| dunder-container | 22 | | imports-dynamic | 17 |
| binary-literals | 22 | | descriptors | 17 |
| async-generators | 22 | | chained-comparisons | 17 |
| raw-strings | 21 | | unpacking | 16 |
| packages-all | 21 | | type-alias-statement | 16 |
| namespace-packages | 21 | | string-encoding | 16 |
| memoryview | 21 | | performance-patterns | 16 |
| match-or-patterns | 21 | | new-vs-init | 16 |
| match-mapping-patterns | 21 | | keyword-only-args | 16 |
| examples-tooling | 21 | | format-spec | 16 |
| file | 16 | | type-guards | 12 |
| const-evaluation | 16 | | numeric-precision | 12 |
| async-comprehensions | 16 | | multi-module | 12 |
| stdlib-misc | 15 | | return-statements | 11 |
| metaclasses | 15 | | result-types | 11 |
| lambdas | 15 | | protocols | 11 |
| init-subclass | 15 | | modules-csv | 11 |
| fstrings-debug | 15 | | lifetimes | 11 |
| ellipsis | 15 | | iterators | 11 |
| concurrency | 15 | | ffi-ctypes | 11 |
| class-getitem | 15 | | decorator-expansion | 11 |
| verification-contracts | 14 | | cli-integration | 11 |
| type-parameter-lists | 14 | | cargo-integration | 11 |
| positional-only-args | 14 | | unicode-strings | 10 |
| fstrings-nested | 14 | | serialization | 10 |
| basic-types | 14 | | mutability | 10 |
| numeric-arrays | 13 | | interprocedural-mutation | 10 |
| modules-os-sys | 13 | | numeric-csv | 9 |
| modules-collections | 13 | | datetime-edge-cases | 9 |
| crypto | 13 | | del-statement | 8 |
| collection-edge-cases | 13 | | ... and 30+ more | <10 each |

---

## Notes

- Many failures share common root causes (especially `serde_json::Value` fallback)
- Fixing type inference would resolve ~30% of failures
- Some features (metaclasses, complex numbers) may need to be documented as unsupported
- Consider adding a "Python compatibility mode" that generates less idiomatic but working Rust
- **Key insight:** Issues 1, 2, and 36 alone account for ~900+ failures
- **Quick wins:** Issues 50, 51, 52 are formatting/syntax fixes that could be resolved quickly
- **High impact:** Issues 45, 46, 47 would significantly improve OOP transpilation quality
