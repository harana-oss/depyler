# Depyler Transpilation Issues - Part 2

This document continues cataloging issues found in the `failures/` directory.

*Last updated: 2026-01-09*

---

## Issue 88: Positional-Only Parameter Syntax (`/`) Drops Parameters

**Affected Categories:** positional-only-args  
**Failure Count:** ~14

**Problem:** When Python functions use positional-only parameters (before `/`), the transpiler drops those parameters entirely from the function signature.

**Examples:**
```python
# Python
def greet(name, /):
    return f"Hello, {name}!"
```
```rust
// Expected:
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Actual:
pub fn greet() -> String {  // Parameter completely missing!
    return format!("Hello, {}!", name);
}
```

**Root Cause:** AST parser doesn't handle the `/` positional-only separator in function definitions.

**Fix:** Parse positional-only parameters and include them in the generated function signature.

---

## Issue 89: Raw String Literals Escape Backslashes Twice

**Affected Categories:** raw-strings  
**Failure Count:** ~21

**Problem:** Python raw strings (`r"..."`) are double-escaped in output, converting `\d` to `\\d`.

**Examples:**
```python
# Python
pattern = r"\d+\.\d+"
```
```rust
// Expected:
let pattern = r"\d+\.\d+";

// Actual:
pub const pattern: &str = "\\d+\\.\\d+";  // Double escaped, not raw
```

**Root Cause:** Raw string handling doesn't preserve the raw nature, instead escaping backslashes.

**Fix:** Use Rust raw strings (`r"..."` or `r#"..."#`) for Python raw strings.

---

## Issue 90: Type Alias Statement Generates Empty Output

**Affected Categories:** type-alias-statement  
**Failure Count:** ~16

**Problem:** Python 3.12+ type alias statements (`type X = Y`) generate empty output instead of Rust type aliases.

**Examples:**
```python
# Python
type Point = tuple[int, int]
result: Point = (1, 2)
```
```rust
// Expected:
fn main() {
    type Point = (i32, i32);
    let result: Point = (1, 2);
}

// Actual: (empty output)
```

**Root Cause:** New Python 3.12 `type` statement not recognized by the transpiler.

**Fix:** Handle `type X = Y` syntax and generate Rust type aliases.

---

## Issue 91: Exception Groups (`except*`) Not Supported

**Affected Categories:** exception-groups  
**Failure Count:** ~19

**Problem:** Python 3.11+ exception groups with `except*` syntax fail with "Statement type not yet supported: unknown".

**Examples:**
```python
# Python
try:
    raise ExceptionGroup("errors", [ValueError(), TypeError()])
except* ValueError:
    pass
```
```rust
// Expected: Some form of multiple error handling

// Actual:
// Transpilation failed: Statement type not yet supported: unknown
```

**Root Cause:** `except*` syntax not implemented in AST handler.

**Fix:** Either implement exception group handling or document as unsupported Python 3.11+ feature.

---

## Issue 92: Self Type (Python 3.11+) Fails on ClassDef

**Affected Categories:** version-features  
**Failure Count:** ~2

**Problem:** Classes using Python 3.11's `Self` type annotation fail with "Statement type not yet supported: ClassDef (classes)".

**Examples:**
```python
# Python
from typing import Self

class Builder:
    def add(self) -> Self:
        return self
```
```rust
// Expected:
impl Builder {
    pub fn add(self) -> Self {
        return self;
    }
}

// Actual:
// Transpilation failed: Statement type not yet supported: ClassDef (classes)
```

**Root Cause:** `Self` type from typing module not recognized, causing class parsing to fail.

**Fix:** Handle `typing.Self` annotation → map to Rust `Self` in impl blocks.

---

## Issue 93: Missing Clone on Borrowed Field Return

**Affected Categories:** ownership, lifetimes  
**Failure Count:** ~30+

**Problem:** Functions returning fields from borrowed parameters don't generate `.clone()`, causing borrow checker errors.

**Examples:**
```python
# Python
def get_data(state: State) -> dict[str, int]:
    return state.data
```
```rust
// Expected:
pub fn get_data(state: &State) -> HashMap<String, i32> {
    return state.data.clone();  // Clone needed to return owned data
}

// Actual:
pub fn get_data(state: &State) -> HashMap<String, i32> {
    return state.data;  // Won't compile - can't move out of borrow
}
```

**Root Cause:** Ownership analysis doesn't detect that returning a field from a borrow requires cloning.

**Fix:** Insert `.clone()` when returning non-Copy fields from borrowed parameters.

---

## Issue 94: Diamond Inheritance Generates Isolated Structs

**Affected Categories:** multiple-inheritance  
**Failure Count:** ~20+

**Problem:** Multiple inheritance patterns (diamond, MRO) generate independent structs without any trait relationships.

**Examples:**
```python
# Python
class A:
    def method(self): return "A"
class B(A): pass
class C(A): pass
class D(B, C): pass  # Diamond pattern
```
```rust
// Expected:
trait A {
    fn method(&self) -> &str { "A" }
}
struct B;
impl A for B {}
struct C;
impl A for C {}
struct D;
impl A for D {}

// Actual:
pub struct A {}  // No trait!
pub struct B {}  // No relationship to A
pub struct C {}  // No relationship to A
pub struct D {}  // No relationship to A
```

**Root Cause:** Inheritance patterns not detected; each class becomes an independent struct.

**Fix:** Detect inheritance and generate trait hierarchies or composition patterns.

---

## Issue 95: Functools.partial Generates Function Without Return Type

**Affected Categories:** functools-partial, functools-cache  
**Failure Count:** ~44+

**Problem:** Functions using `functools.partial` lose return type information and partial application logic.

**Examples:**
```python
# Python
from functools import partial
def add(a, b): return a + b
add5 = partial(add, 5)
```
```rust
// Expected:
fn add(a: i32, b: i32) -> i32 {
    a + b
}
let add5 = |b| add(5, b);

// Actual:
pub fn add(a: i32, b: i32) {  // Missing return type!
    return a + b;
}
// Partial application not generated!
```

**Root Cause:** Return type inference fails when functools decorators/functions are involved.

**Fix:** Preserve return types through functools usage; generate closures for partial application.

---

## Issue 96: Nested JSON/Dict Structure Has Redundant Error Types

**Affected Categories:** serialization, dictionaries  
**Failure Count:** ~20+

**Problem:** Functions working with nested dictionaries generate unnecessary error types and wrap results in `Result<T, E>` even when no errors are possible.

**Examples:**
```python
# Python
def create_nested() -> dict[str, dict[str, int]]:
    return {"level1": {"level2": 42}}
```
```rust
// Expected:
pub fn create_nested() -> HashMap<String, HashMap<String, i32>> {
    let mut inner = HashMap::new();
    inner.insert("level2".to_string(), 42);
    let mut outer = HashMap::new();
    outer.insert("level1".to_string(), inner);
    return outer;
}

// Actual (has unnecessary IndexError definition):
pub struct IndexError { message: String }
impl std::fmt::Display for IndexError { ... }
impl std::error::Error for IndexError {}
pub fn create_nested() -> HashMap<String, HashMap<String, i32>> { ... }
```

**Root Cause:** Error type generation is too eager, adding error types even when not used.

**Fix:** Only generate error types when they're actually referenced by the function.

---

## Issue 97: F-String Debug Syntax (`{x=}`) Not Expanded

**Affected Categories:** fstrings-debug  
**Failure Count:** ~15

**Problem:** Python f-string debug syntax `f"{x=}"` should expand to `"x=42"` but instead generates incorrect code.

**Examples:**
```python
# Python
x = 42
result = f"{x=}"  # Should print "x=42"
```
```rust
// Expected:
let x = 42;
let result = format!("x={}", x);

// Actual:
pub const x: i32 = 42;
pub const result: serde_json::Value = format!("Value: {}", x);  // Wrong expansion
```

**Root Cause:** Debug f-string syntax (`=` after variable name) not recognized and expanded.

**Fix:** Detect `{var=}` pattern and expand to `"var={var}"` format.

---

## Issue 98: Hashlib/Crypto Functions Missing Closing Brace

**Affected Categories:** crypto  
**Failure Count:** ~13

**Problem:** Cryptographic functions (hashlib) have inconsistent brace handling, sometimes missing closing braces.

**Examples:**
```rust
// Expected:
pub fn hash_sha256(data: &bytes) -> String {
    return {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    };
}

// Actual:
pub fn hash_sha256(data: &bytes) -> String {
    return {
        ...
    }  // Missing semicolon after closing brace
}    // Extra closing brace
```

**Root Cause:** Block expression syntax handling inconsistent.

**Fix:** Ensure block expressions have proper brace matching and semicolons.

---

## Issue 99: `object` Type Not Mapped to Any Type

**Affected Categories:** type-guards, type-inference  
**Failure Count:** ~12+

**Problem:** Python `object` type annotation is kept literally as `&object` which is invalid Rust.

**Examples:**
```python
# Python
def process(value: object) -> int:
    if isinstance(value, int):
        return value * 2
    return 0
```
```rust
// Expected:
pub fn process<T>(value: T) -> i32 { ... }
// Or:
pub fn process(value: &dyn Any) -> i32 { ... }

// Actual:
pub fn process(value: &object) -> i32 { ... }  // Invalid type 'object'
```

**Root Cause:** Python `object` type not mapped to any Rust equivalent.

**Fix:** Map `object` to generic `T` or `&dyn Any` depending on context.

---

## Issue 100: Implicit Type Narrowing Not Applied After `isinstance`

**Affected Categories:** type-guards  
**Failure Count:** ~12

**Problem:** After an `isinstance` check, type narrowing should allow methods specific to the narrowed type, but the original type is still used.

**Examples:**
```python
# Python
def process(value: object) -> int:
    if isinstance(value, int):
        return value * 2  # value is narrowed to int here
    return 0
```
```rust
// Expected:
if let Some(v) = value.downcast_ref::<i32>() {
    return v * 2;
}

// Actual:
if true {  // isinstance check becomes literal `true`
    return value * 2;  // Type not narrowed
}
```

**Root Cause:** `isinstance` checks replaced with `true` and no type narrowing logic applied.

**Fix:** Implement type narrowing for isinstance checks using match or if-let patterns.

---

## Issue 101: Vec Parameter Should Use Ownership, Not Borrow

**Affected Categories:** performance-patterns, functions  
**Failure Count:** ~20+

**Problem:** Function parameters of `Vec<T>` type sometimes use borrow (`&Vec<T>`) when ownership would be more efficient.

**Examples:**
```python
# Python
def sum_with_factor(items: list[int], factor: int) -> int:
    total = 0
    for item in items:
        total += item * factor
    return total
```
```rust
// Expected (ownership, more efficient):
pub fn sum_with_factor(items: Vec<i32>, factor: i32) -> i32 {
    let mut total = 0;
    for item in items {
        total += item * factor;
    }
    return total;
}

// Actual (borrow, less efficient):
pub fn sum_with_factor(items: &Vec<i32>, factor: i32) -> i32 {
    let mut total = 0;
    for item in items.iter().cloned() {  // Unnecessary clone
        total += item * factor;
    }
    return total;
}
```

**Root Cause:** Conservative borrowing strategy always uses references.

**Fix:** Analyze whether ownership or borrowing is more efficient; prefer ownership when collection is consumed.

---

## Issue 102: Contextmanager Decorator Creates Iterator Instead of Closure

**Affected Categories:** contextlib-decorator  
**Failure Count:** ~20+

**Problem:** `@contextmanager` decorated functions become iterator state machines instead of closure-based context managers.

**Examples:**
```python
# Python
from contextlib import contextmanager

@contextmanager
def simple():
    yield
```
```rust
// Expected:
fn simple<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}

// Actual:
struct SimpleState { state: usize }
pub fn simple() -> impl Iterator<Item = serde_json::Value> {
    SimpleState { state: 0 }
}
impl Iterator for SimpleState { ... }
```

**Root Cause:** `@contextmanager` not recognized; generator is used instead of closure pattern.

**Fix:** Detect `@contextmanager` decorator and generate closure-based RAII pattern.

---

## Issue 103: Formatting Extra Semicolon After Function Definition

**Affected Categories:** control-flow  
**Failure Count:** ~10+

**Problem:** Function definitions sometimes have an extra semicolon after the closing brace.

**Examples:**
```rust
// Expected:
pub fn abs_value(x: i32) -> i32 {
    if x < 0 {
        return (-x);
    } else {
        return x;
    }
}

// Actual:
pub fn abs_value(x: i32) -> i32 {
    if x < 0 {
        return (-x);
    } else {
        return x;
    }
};  // Extra semicolon!
```

**Root Cause:** Expression vs statement handling in code generation.

**Fix:** Don't add semicolon after function definition closing braces.

---

## Issue 104: Import Statement Generates Only Alias Without Usage

**Affected Categories:** imports-basic  
**Failure Count:** ~19+

**Problem:** Some import statements generate only the `use` statement without any accompanying code.

**Examples:**
```python
# Python
import json
data = {"key": "value"}
result = json.dumps(data)
```
```rust
// Expected:
use serde_json;
let mut data = HashMap::new();
data.insert("key", "value");
let result = serde_json::to_string(&data).unwrap();

// Actual:
use serde_json as json;
// All code after import is missing!
```

**Root Cause:** Module-level code after import not transpiled.

**Fix:** Include all module-level statements, not just imports.

---

## Issue 106: CSE Optimization Creates Unnecessary Mutable Variable

**Affected Categories:** cse-optimization, type-inference  
**Failure Count:** ~25+

**Problem:** Common subexpression elimination creates `mut` variables when immutable would suffice.

**Examples:**
```rust
// Expected:
let temp;
if true {
    temp = value * 2;
    result = temp + 1;
}

// Actual:
let mut temp;  // Unnecessary 'mut'
if true {
    let _cse_temp_0 = value.clone() * 2;  // Extra temp variable
    temp = _cse_temp_0;
    result = temp + 1;
}
```

**Root Cause:** CSE optimization doesn't analyze mutability requirements.

**Fix:** Only mark variables as `mut` when they're actually reassigned.

---

## Issue 107: Resource Management Functions Inconsistently Wrap in Result

**Affected Categories:** resource-management  
**Failure Count:** ~10

**Problem:** Functions using `unwrap_or_default()` shouldn't return `Result<T, E>` since they handle errors internally.

**Examples:**
```python
# Python
def read_file_safe(path: str) -> str:
    try:
        return open(path).read()
    except:
        return ""
```
```rust
// Expected:
pub fn read_file_safe(path: String) -> String {
    return std::fs::read_to_string(path).unwrap_or_default();
}

// Actual:
pub fn read_file_safe(path: String) -> Result<String, std::io::Error> {
    return std::fs::read_to_string(path).unwrap_or_default();  // Can't return String in Result<String, _>
}
```

**Root Cause:** Error handling analysis doesn't consider that try/except with default return handles errors internally.

**Fix:** Don't wrap in Result when function has internal error handling with defaults.

---

## Issue 108: Result Type Return But Function Uses `panic!`

**Affected Categories:** result-types, error-handling  
**Failure Count:** ~40+

**Problem:** Some functions that should use `Result<T, E>` instead use `panic!`, and vice versa—inconsistent error handling strategy.

**Examples:**
```python
# Python
def update_config(config: dict, key: str, value: int):
    if key not in config:
        raise KeyError("Key not found")
    config[key] = value
```
```rust
// Expected (consistent - use panic OR Result, not both):
pub fn update_config(config: &mut HashMap<String, i32>, key: String, value: i32) -> Result<(), KeyError> {
    if !config.contains_key(&key) {
        return Err(KeyError::new("Key not found"));
    }
    config.insert(key, value);
    Ok(())
}

// Actual (no Result type, uses panic):
pub fn update_config(config: &mut HashMap<String, i32>, key: String, value: i32) {
    if !config.get(&key).is_some() {
        panic!("{}", "Key not found");  // Should be Result::Err
    }
    config.insert(key, value);
}
```

**Root Cause:** Inconsistent decision between `panic!` and `Result<T, E>` for error handling.

**Fix:** Establish consistent error handling: use `Result` for recoverable errors, `panic!` for programmer errors.

---

## Issue 109: Classes Expected as Error but Actual Transpiles Successfully (Different Structure)

**Affected Categories:** classes  
**Failure Count:** ~31

**Problem:** Some class transpilation tests expect an error but actual code transpiles—just with different structure than expected.

**Examples:**
```rust
// Expected:
// Error: Statement type not yet supported

// Actual (transpiles but with different structure):
pub struct Point {
    pub x: i32,
    pub y: i32,
}
impl Point {
    pub fn new(x: i32, y: i32) -> Self { ... }
    pub fn distance_from_origin(&self) -> f64 { ... }
}
```

**Root Cause:** Test expectations may be outdated; transpiler now handles cases that previously failed.

**Fix:** Update test expectations to match new transpiler capabilities.

---

## Issue 110: `contains_key` vs `get().is_some()` Inconsistency

**Affected Categories:** dictionaries, serialization  
**Failure Count:** ~30+

**Problem:** Dictionary membership checks inconsistently use `contains_key()` vs `get().is_some()`.

**Examples:**
```rust
// Expected (idiomatic):
if !config.contains_key(&key) {
    return Err(...);
}

// Actual (verbose):
if !config.get(&key).is_some() {
    panic!("Key not found");
}
```

**Root Cause:** No standardization on dictionary membership check idiom.

**Fix:** Use `contains_key()` for membership checks, `get()` when value is needed.

---

## Issue 111: Keyword-Only Parameters (`*`) Drop All Parameters

**Affected Categories:** keyword-only-args  
**Failure Count:** ~16

**Problem:** When Python functions use keyword-only parameters (after `*`), the transpiler drops all parameters.

**Examples:**
```python
# Python
def greet(*, name: str) -> str:
    return f"Hello, {name}!"
```
```rust
// Expected:
struct GreetArgs { name: String }
fn greet(args: GreetArgs) -> String {
    format!("Hello, {}!", args.name)
}

// Actual:
pub fn greet() -> String {  // All parameters missing!
    return format!("Hello, {}!", name);
}
```

**Root Cause:** Keyword-only parameter syntax (`*,`) not handled.

**Fix:** Generate struct for keyword args or use named parameters pattern.

---

## Issue 112: Unpacking/Destructuring Generates Empty Output

**Affected Categories:** unpacking  
**Failure Count:** ~16

**Problem:** Tuple/list unpacking to multiple variables generates empty output.

**Examples:**
```python
# Python
a, b, c = [1, 2, 3]
print(f"a={a}, b={b}, c={c}")
```
```rust
// Expected:
let (a, b, c) = vec![1, 2, 3];
log::info!("{}", format!("a={}, b={}, c={}", a, b, c));

// Actual: (empty output)
```

**Root Cause:** Destructuring assignment statement not implemented.

**Fix:** Implement tuple unpacking via pattern matching.

---

## Issue 113: Del Statement Generates Empty Output

**Affected Categories:** del-statement  
**Failure Count:** ~8

**Problem:** Python `del` statement for removing dict keys generates empty output.

**Examples:**
```python
# Python
d = {"a": 1, "b": 2}
del d["b"]
```
```rust
// Expected:
let mut d = HashMap::new();
d.insert("a", 1);
d.insert("b", 2);
d.remove("b");

// Actual: (empty output)
```

**Root Cause:** `del` statement not implemented.

**Fix:** Map `del d[key]` to `d.remove(key)`, `del var` to drop/scope exit.

---

## Issue 114: Modulo Operator Generates ZeroDivisionError Unnecessarily  

**Affected Categories:** numeric-precision, datetime-edge-cases, mutability  
**Failure Count:** ~30+

**Problem:** Any use of modulo (`%`) operator generates ZeroDivisionError type and wraps return in Result, even when divisor is a constant.

**Examples:**
```python
# Python
def is_leap_year(year: int) -> bool:
    return year % 4 == 0  # 4 is constant, never zero
```
```rust
// Expected:
pub fn is_leap_year(year: i32) -> bool {
    return year % 4 == 0;
}

// Actual:
pub struct ZeroDivisionError { ... }
pub fn is_leap_year(year: i32) -> Result<bool, ZeroDivisionError> {
    return Ok(year % 4 == 0);
}
```

**Root Cause:** Modulo operator treated as potentially failing regardless of operands.

**Fix:** Only generate ZeroDivisionError when divisor could actually be zero.

---

## Issue 115: Format Spec Uses Python Syntax Instead of Rust

**Affected Categories:** format-spec  
**Failure Count:** ~16

**Problem:** Format specifiers keep Python syntax (`.2f`) instead of Rust syntax (`.2`).

**Examples:**
```rust
// Expected:
return format!("{:.2}", 3.14159);

// Actual:
return format!("{:.2f}", 3.14159);  // 'f' is invalid in Rust
```

**Root Cause:** Format spec not translated from Python to Rust syntax.

**Fix:** Convert Python format specifiers to Rust equivalents.

---

## Issue 116: `__class_getitem__` Not Transpiled to Generics

**Affected Categories:** class-getitem  
**Failure Count:** ~15

**Problem:** Classes with `__class_getitem__` should become generic structs but generate plain structs.

**Examples:**
```python
# Python
class MyClass:
    def __class_getitem__(cls, item):
        return f"MyClass[{item}]"
```
```rust
// Expected:
struct MyClass<T> {
    _marker: std::marker::PhantomData<T>,
}
impl<T> MyClass<T> {
    fn type_name() -> String where T: 'static {
        format!("MyClass[{}]", std::any::type_name::<T>())
    }
}

// Actual:
pub struct MyClass {}
impl MyClass {
    pub fn new() -> Self { Self {} }
}
```

**Root Cause:** `__class_getitem__` dunder method not recognized.

**Fix:** Detect `__class_getitem__` and generate generic struct with PhantomData.

---

## Issue 117: Property Getter Missing Return Type

**Affected Categories:** properties  
**Failure Count:** ~17

**Problem:** Property getters generate methods without return types.

**Examples:**
```python
# Python
class Person:
    @property
    def name(self) -> str:
        return self._name
```
```rust
// Expected:
fn name(&self) -> &str {
    &self.name
}

// Actual:
pub fn name(&self) {  // Missing return type!
    return self._name.clone();
}
```

**Root Cause:** Property decorator doesn't preserve return type annotation.

**Fix:** Extract return type from property method signature.

---

## Issue 118: Double Type Cast `as i32 as i32`

**Affected Categories:** unicode-strings, string-encoding  
**Failure Count:** ~16+

**Problem:** Type casts are duplicated in the output.

**Examples:**
```rust
// Expected:
return s.len() as i32;

// Actual:
return s.len() as i32 as i32;  // Redundant cast
```

**Root Cause:** Cast insertion happens twice in code generation pipeline.

**Fix:** Deduplicate consecutive identical casts.

---

## Issue 119: Unicode Function Names Change Output Function Name

**Affected Categories:** boundary-values  
**Failure Count:** ~18

**Problem:** Tests expecting one function but getting a different function with unicode name.

**Examples:**
```rust
// Expected:
pub fn greet() -> String {
    return "Hello, 世界! 🌍".to_string();
}

// Actual:
pub fn функция(x: i32) -> i32 {  // Different function entirely
    return x * 2;
}
```

**Root Cause:** Test ordering or function selection issue in multi-function files.

**Fix:** Ensure correct function is selected from source file.

---

## Issue 120: Lambda Sort Key Leaves Code Outside Function

**Affected Categories:** lambdas  
**Failure Count:** ~15

**Problem:** Lambda tests leave test assertions outside the function body.

**Examples:**
```rust
// Expected:
pub fn sort_by_length(words: &Vec<String>) -> Vec<String> {
    return { let mut __sorted = words.clone(); __sorted.sort_by_key(|x| x.len()); __sorted };
}

// Actual:
pub fn sort_by_length(words: &Vec<String>) -> Vec<String> {
    return { ... };
    // Test code leaked outside function:
    let result = sort_by_length(&words);
    for i in 1..result.len() { ... }
}
```

**Root Cause:** Test harness code included in transpilation output.

**Fix:** Filter out test code from function body.

---

## Issue 121: Dead Code Elimination Not Applied

**Affected Categories:** dead-code-elimination  
**Failure Count:** ~9

**Problem:** Unused assignments should be eliminated but are preserved.

**Examples:**
```python
# Python
def use_only_y(x: int, y: int) -> int:
    unused = x * 2  # Never used
    return y
```
```rust
// Expected:
pub fn use_only_y(x: i32, y: i32) -> i32 {
    return y;  // unused eliminated
}

// Actual:
pub fn use_only_y(x: i32, y: i32) -> i32 {
    let unused = x * 2;  // Preserved, should be removed
    return y;
}
```

**Root Cause:** DCE pass not running or not effective.

**Fix:** Implement/enable dead code elimination pass.

---

## Issue 122: Field Mutation Should Use `&mut` Parameter

**Affected Categories:** field-alias-mutation, mutability  
**Failure Count:** ~10+

**Problem:** Functions that mutate struct fields should take `&mut` but take `&`.

**Examples:**
```python
# Python
def add_to_list(state: State):
    state.items.append(1)  # Mutates state
```
```rust
// Expected:
pub fn add_to_list(state: &mut State) {
    let mut items = state.items.clone();
    items.push(1);
}

// Actual:
pub fn add_to_list(state: &State) {  // Should be &mut!
    let mut items = state.items;  // Can't move out of borrow
    items.push(1);
}
```

**Root Cause:** Mutation analysis doesn't propagate through field access.

**Fix:** Detect field mutations and make parameter `&mut`.

---

## Issue 123: Function Missing Return Type When Has Return Statement

**Affected Categories:** borrow-checker-violations  
**Failure Count:** ~15+

**Problem:** Functions with return statements sometimes have no return type.

**Examples:**
```rust
// Expected:
pub fn caller() -> i32 {
    let data = vec![1, 2, 3];
    let result = consume(data);
    return result;
}

// Actual:
pub fn caller() {  // Missing return type!
    let data = vec![1, 2, 3];
    let result = consume(data);
    return result;
}
```

**Root Cause:** Return type inference fails.

**Fix:** Infer return type from return statements.

---

## Issue 124: Match Or-Patterns Generate Empty Output

**Affected Categories:** match-or-patterns  
**Failure Count:** ~21

**Problem:** Match statements with or-patterns (`|`) generate empty output.

**Examples:**
```python
# Python
match x:
    case 1 | 2 | 3:
        result = True
```
```rust
// Expected:
let result = match x {
    1 | 2 | 3 => true,
    _ => false,
};

// Actual: (empty output)
```

**Root Cause:** Or-patterns in match not implemented.

**Fix:** Support `|` pattern syntax in match arms.

---

## Issue 125: Chained Comparisons Use `pub const` with `serde_json::Value`

**Affected Categories:** chained-comparisons  
**Failure Count:** ~17

**Problem:** Chained comparisons as module-level code become invalid `pub const`.

**Examples:**
```python
# Python
a = 1
b = 2
c = 3
result = a < b < c
```
```rust
// Expected:
let a = 1;
let b = 2;
let c = 3;
let result = a < b && b < c;

// Actual:
pub const a: i32 = 1;
pub const b: i32 = 2;
pub const c: i32 = 3;
pub const result: serde_json::Value = (a < b) && (b < c);  // serde_json::Value for bool!
```

**Root Cause:** Module-level code incorrectly uses `const`; bool type falls back to serde_json::Value.

**Fix:** Use `let` for runtime values; properly infer `bool` type for comparisons.

---

## Issue 126: Augmented Assignment on Lists Generates Empty Output

**Affected Categories:** augmented-assignment  
**Failure Count:** ~21

**Problem:** `lst += [items]` generates empty output.

**Examples:**
```python
# Python
lst = [1, 2, 3]
lst += [4, 5]
```
```rust
// Expected:
let mut lst = vec![1, 2, 3];
lst.extend(vec![4, 5]);

// Actual: (empty output)
```

**Root Cause:** List augmented assignment not implemented.

**Fix:** Map `lst += items` to `lst.extend(items)`.

---

## Issue 127: DefaultDict Not Mapped to Entry API

**Affected Categories:** modules-collections  
**Failure Count:** ~13

**Problem:** Python `defaultdict` generates invalid `HashMap(list)` syntax.

**Examples:**
```python
# Python
from collections import defaultdict
groups = defaultdict(list)
```
```rust
// Expected:
let mut groups: HashMap<String, Vec<String>> = HashMap::new();
// Use entry API: groups.entry(key).or_insert_with(Vec::new).push(value);

// Actual:
let groups = std::collections::HashMap(list);  // Invalid syntax!
```

**Root Cause:** `defaultdict` not mapped to Rust's Entry API pattern.

**Fix:** Generate HashMap with entry().or_default() pattern.

---

## Issue 128: File Read Returns `Result` When Expected Plain Type

**Affected Categories:** file, io-streams  
**Failure Count:** ~16+

**Problem:** File operations that use `?` operator should return Result, but expected output wants plain type.

**Examples:**
```rust
// Expected:
pub fn read_file(path: String) -> String {
    let mut f = std::fs::File::open(path)?;
    // ...
}

// Actual:
pub fn read_file(path: String) -> Result<String, std::io::Error> {
    return std::fs::read_to_string(path).unwrap_or_default();
}
```

**Root Cause:** Inconsistent error handling strategy between expected and actual.

**Fix:** Standardize on either Result returns or unwrap_or_default pattern.

---

## Issue 129: Verification Assert Wrapped in Unnecessary Result

**Affected Categories:** verification-contracts  
**Failure Count:** ~14

**Problem:** Functions with `assert!` preconditions get wrapped in Result even when assert should panic.

**Examples:**
```python
# Python
def divide(a: int, b: int) -> int:
    assert b != 0, "divisor cannot be zero"
    return a // b
```
```rust
// Expected (assert panics):
pub fn divide(a: i32, b: i32) -> i32 {
    assert!(b != 0, "divisor cannot be zero");
    return a / b;
}

// Actual (unnecessary Result):
pub fn divide(a: i32, b: i32) -> Result<i32, ZeroDivisionError> {
    assert!(b != 0, "{}", "divisor cannot be zero");
    return Ok(a / b);
}
```

**Root Cause:** Division triggers ZeroDivisionError even when assertion guards against it.

**Fix:** Recognize assertion guards and don't generate error types when guarded.

---

## Issue 130: `__init_subclass__` Generates Invalid Const Vec

**Affected Categories:** init-subclass  
**Failure Count:** ~15

**Problem:** `__init_subclass__` hook generates `pub const: Vec<...> = vec![]` which is invalid.

**Examples:**
```rust
// Expected (use trait + manual registration):
trait Base { fn name() -> &'static str; }

// Actual:
pub struct Base {}
impl Base {
    pub const subclass_created: Vec<serde_json::Value> = vec![];  // Invalid const!
}
```

**Root Cause:** Mutable class attributes become invalid const declarations.

**Fix:** Use lazy_static for mutable static data, or document as unsupported.

---

## Issue 131: Type Parameter Lists (Python 3.12+) Generate Placeholder

**Affected Categories:** type-parameter-lists  
**Failure Count:** ~14

**Problem:** Python 3.12 type parameter syntax generates placeholder instead of generic function.

**Examples:**
```python
# Python 3.12+
def f[T](x: T) -> T:
    return x
```
```rust
// Expected:
pub fn f<T>(x: T) -> T {
    return x;
}

// Actual:
pub const result: bool = true;  // Placeholder!
```

**Root Cause:** Python 3.12 type parameter syntax not implemented.

**Fix:** Parse `def f[T](...)` syntax and generate Rust generics.

---

## Issue 132: List Comprehension with Find Unnecessarily Wrapped in Result

**Affected Categories:** comprehensions  
**Failure Count:** ~10+

**Problem:** List comprehension optimizations like `next(x for x in ...)` generate IndexError type unnecessarily.

**Examples:**
```python
# Python
def first_even(numbers: list[int]) -> int:
    return next(x for x in numbers if x % 2 == 0)
```
```rust
// Expected:
pub fn first_even(numbers: &Vec<i32>) -> i32 {
    return numbers.iter().find(|x| **x % 2 == 0).cloned().unwrap();
}

// Actual:
pub struct IndexError { ... }
pub fn first_even(numbers: &Vec<i32>) -> Result<i32, IndexError> {
    return Ok(numbers.iter().find(|x| *x % 2 == 0).cloned().unwrap());
}
```

**Root Cause:** Iterator operations assumed to be fallible.

**Fix:** Don't generate error types for iterator find/filter operations.

---

## Issue 133: Integer Parameter Incorrectly Becomes Reference

**Affected Categories:** optimizer-inlining  
**Failure Count:** ~10+

**Problem:** Primitive integer parameters sometimes become references (`&i32`) instead of being passed by value.

**Examples:**
```rust
// Expected:
pub fn double_identity(x: i32) -> i32 {
    return identity(identity(x));
}

// Actual:
pub fn double_identity(x: &i32) -> i32 {  // Unnecessary reference!
    return identity(identity(x));
}
```

**Root Cause:** Reference insertion too aggressive for Copy types.

**Fix:** Don't use references for Copy types (i32, bool, char, etc.).

---

## Issue 134: Vec::new() vs vec![] Inconsistency

**Affected Categories:** trait-impls  
**Failure Count:** ~10+

**Problem:** Empty collections sometimes use `Vec::new()`, sometimes `vec![]`.

**Examples:**
```rust
// Expected:
Self {
    items: Vec::new(),
    name: String::new(),
}

// Actual:
Self {
    items: vec![],  // Inconsistent with String::new()
    name: "".to_string(),  // Inconsistent with Vec::new()
}
```

**Root Cause:** No standardization on empty collection initialization.

**Fix:** Use consistent patterns: `Vec::new()` or `vec![]`, `String::new()` or `"".to_string()`.

---

## Issue 135: String Concatenation Uses `+=` Instead of `format!`

**Affected Categories:** loop-for  
**Failure Count:** ~10+

**Problem:** String concatenation in loops uses `+=` which may have different semantics.

**Examples:**
```rust
// Expected:
result = format!("{}{}", result, val);

// Actual:
result += val;  // Different semantics, needs String + &str types to match
```

**Root Cause:** Inconsistent string concatenation strategy.

**Fix:** Use consistent pattern for string building in loops.

---

## Summary of New Issues (111-135)

| Issue | Category | Impact | Fix Complexity |
|-------|----------|--------|----------------|
| 111 | Keyword-Only Args | Medium | Medium |
| 112 | Unpacking | High | Medium |
| 113 | Del Statement | Medium | Easy |
| 114 | Modulo ZeroDivision | High | Medium |
| 115 | Format Spec | Medium | Easy |
| 116 | Class Getitem | Medium | Hard |
| 117 | Property Return Type | High | Medium |
| 118 | Double Cast | Low | Easy |
| 119 | Unicode Names | Low | Easy |
| 120 | Lambda Test Leak | Low | Easy |
| 121 | Dead Code Elim | Medium | Medium |
| 122 | Field Mutation | High | Medium |
| 123 | Missing Return Type | High | Medium |
| 124 | Match Or-Patterns | High | Medium |
| 125 | Chained Comparisons | Medium | Medium |
| 126 | Augmented Assign List | Medium | Medium |
| 127 | DefaultDict | High | Medium |
| 128 | File Result | Medium | Easy |
| 129 | Verification Assert | Medium | Medium |
| 130 | Init Subclass | Low | Medium |
| 131 | Type Params 3.12 | Medium | Hard |
| 132 | Comprehension Result | Medium | Easy |
| 133 | Int Reference | Medium | Easy |
| 134 | Vec Initialization | Low | Easy |
| 135 | String Concat | Low | Easy |

**Key Observations (Issues 111-135):**
- Issues 112, 114, 117, 122, 123, 124, 127 are high-impact
- Issues 113, 115, 118, 119, 120, 128, 132, 133, 134, 135 are easy fixes
- Python 3.12 features (131) need dedicated support
- Many issues relate to overly conservative error type generation (114, 129, 132)

---

## Issue 136: Implicit Return Inconsistency

**Affected Categories:** examples-algorithms, examples-data-structures  
**Failure Count:** ~30+

**Problem:** Some functions use implicit returns (expression without `return`) but transpiler adds explicit `return` statements inconsistently.

**Examples:**
```rust
// Expected (implicit return):
if n <= 1 {
    return n;
}
fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2)

// Actual (explicit return added):
if n <= 1 {
    return n;
}
return fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2);
```

**Root Cause:** Inconsistent handling of expression-body functions vs statement-body.

**Fix:** Standardize on one style (either always explicit or context-aware implicit).

---

## Issue 137: Mutable Reference vs Owned Value in Recursive Calls

**Affected Categories:** examples-algorithms  
**Failure Count:** ~20+

**Problem:** Functions that take `&mut HashMap` pass `&memo` instead of `&mut memo` to recursive calls.

**Examples:**
```rust
// Expected:
pub fn fibonacci_memoized(n: i32, memo: &mut HashMap<i32, i32>) -> Result<i32, IndexError> {
    result = fibonacci_memoized(n - 1, memo) + fibonacci_memoized(n - 2, memo);
}

// Actual:
pub fn fibonacci_memoized(n: i32, memo: &mut HashMap<i32, i32>) -> Result<i32, IndexError> {
    result = fibonacci_memoized(n - 1, &memo) + fibonacci_memoized(n - 2, &memo);  // Wrong: &memo instead of memo
}
```

**Root Cause:** Reference passing doesn't preserve mutability through recursive calls.

**Fix:** Track parameter mutability and pass `memo` (already mut ref) or `&mut memo` appropriately.

---

## Issue 138: Missing Derive `Copy` When Should Have, Extra When Shouldn't

**Affected Categories:** examples-data-structures, copy-type-semantics, reference-return-types  
**Failure Count:** ~25+

**Problem:** `Copy` trait derived on types that can't be Copy (contain Vec, String), or missing when it should be present.

**Examples:**
```rust
// Expected (Stack has Vec, can't be Copy):
#[derive(Debug, Clone)]
pub struct Stack {}

// Actual:
#[derive(Debug, Copy, Clone)]  // Invalid! Stack has _items: Vec
pub struct Stack {}
```

**Root Cause:** Copy derivation doesn't analyze struct field types.

**Fix:** Only derive Copy when all fields are Copy types.

---

## Issue 139: Mutating Methods Missing `&mut self`

**Affected Categories:** examples-data-structures  
**Failure Count:** ~15+

**Problem:** Methods that modify `self` (like `push`, `pop`) don't take `&mut self`.

**Examples:**
```rust
// Expected:
pub fn push(&mut self, item: i32) {
    self._items.push(item);
}

// Actual:
pub fn push(&self, item: i32) {  // Should be &mut self
    self._items.push(item);  // Won't compile
}
```

**Root Cause:** Self mutation analysis not applied to method signatures.

**Fix:** Detect self-mutating methods and use `&mut self`.

---

## Issue 140: `__eq__` Generates Struct Without `PartialEq` Trait

**Affected Categories:** dunder-comparison  
**Failure Count:** ~19+

**Problem:** Classes with `__eq__` don't generate `#[derive(PartialEq)]` or `impl PartialEq`.

**Examples:**
```rust
// Expected:
#[derive(PartialEq)]
struct Point { x: i32, y: i32 }

// Actual:
#[derive(Debug, Clone)]
pub struct Point { pub x: serde_json::Value, pub y: serde_json::Value }
// No PartialEq!
```

**Root Cause:** `__eq__` dunder method not mapped to PartialEq trait.

**Fix:** Detect `__eq__` → derive or implement `PartialEq`.

---

## Issue 141: Async For Uses Sync Iterator Pattern

**Affected Categories:** async-for, async-generators  
**Failure Count:** ~40+

**Problem:** Async for loops generate synchronous Iterator implementations instead of using async Stream.

**Examples:**
```rust
// Expected:
use futures::stream::{self, StreamExt};
async fn test() -> Vec<i32> {
    let mut result = Vec::new();
    let mut stream = stream::iter(0..5);
    while let Some(item) = stream.next().await {
        result.push(item);
    }
    result
}

// Actual (sync Iterator):
struct AsyncRangeState { state: usize, n: serde_json::Value }
impl Iterator for AsyncRangeState { ... }  // Sync, not async!
```

**Root Cause:** Async iteration not distinguished from sync iteration.

**Fix:** Use `futures::Stream` trait for async iteration, not `Iterator`.

---

## Issue 142: `contains_key` Used on String Instead of HashMap

**Affected Categories:** examples-data-structures  
**Failure Count:** ~15+

**Problem:** `contains_key` method called on strings where `contains` should be used.

**Examples:**
```rust
// Expected:
if opening.contains(&char) { ... }

// Actual:
if opening.contains_key(&char) { ... }  // contains_key is for HashMap, not String
```

**Root Cause:** Method selection doesn't check receiver type.

**Fix:** Use `contains` for strings/iterables, `contains_key` for HashMaps.

---

## Issue 143: Type Inference Falls Back to `serde_json::Value` for Primitives

**Affected Categories:** examples-type-hints  
**Failure Count:** ~47+

**Problem:** Even when type hints suggest primitives (int, str), transpiler uses `serde_json::Value`.

**Examples:**
```python
# Python
def process_numbers(a: int, b: int) -> int:
    return a + b
```
```rust
// Expected:
pub fn process_numbers(a: i32, b: i32) -> i32 { ... }

// Actual:
pub fn process_numbers<'b, 'a>(a: &'a serde_json::Value, b: &'b serde_json::Value) -> i32 { ... }
```

**Root Cause:** Type hints not properly translated even when explicit.

**Fix:** Respect Python type hints and map to appropriate Rust types.

---

## Issue 144: Nested Ternary/If-Else Generates Empty Output

**Affected Categories:** ternary-expressions  
**Failure Count:** ~20+

**Problem:** Nested ternary expressions at module level generate empty output.

**Examples:**
```python
# Python
x = False
y = True
result = "a" if x else ("b" if y else "c")
```
```rust
// Expected:
let x = false;
let y = true;
let result = if x { "a" } else if y { "b" } else { "c" };

// Actual: (empty output)
```

**Root Cause:** Nested conditionals at module level not handled.

**Fix:** Wrap module-level expressions in main/test function.

---

## Issue 145: In-Place Operators (`__iadd__`) Don't Generate `AddAssign` Trait

**Affected Categories:** dunder-inplace  
**Failure Count:** ~20+

**Problem:** Python `__iadd__` and similar dunder methods don't generate `std::ops::AddAssign` impl.

**Examples:**
```rust
// Expected:
impl AddAssign for Counter {
    fn add_assign(&mut self, other: Counter) {
        self.x += other.x;
    }
}

// Actual:
pub struct Counter { pub x: serde_json::Value }
// No AddAssign impl!
```

**Root Cause:** In-place operator dunders not mapped to traits.

**Fix:** Map `__iadd__` → `impl AddAssign`, `__isub__` → `impl SubAssign`, etc.

---

## Issue 146: Vec Ownership vs Reference Inconsistency

**Affected Categories:** collection-edge-cases  
**Failure Count:** ~13+

**Problem:** Functions that consume a Vec are transpiled to take `&Vec` and then clone unnecessarily.

**Examples:**
```rust
// Expected (ownership):
pub fn safe_filter(items: Vec<i32>) -> Vec<i32> {
    for item in items { ... }
}

// Actual (borrow + clone):
pub fn safe_filter(items: &Vec<i32>) -> Vec<i32> {
    for item in items.iter().cloned() { ... }  // Unnecessary clone
}
```

**Root Cause:** Conservative borrowing when ownership would be more efficient.

**Fix:** Analyze usage and prefer ownership when Vec is consumed.

---

## Issue 147: Decorator Pattern Generates Nested Function Mess

**Affected Categories:** decorators-basic  
**Failure Count:** ~21+

**Problem:** Complex decorators like `@retry` generate deeply nested, broken function structures.

**Examples:**
```rust
// Expected:
fn retry<F>(f: F, max_attempts: usize) -> impl Fn() -> Result<String, String>
where F: Fn() -> Result<String, String> { ... }

// Actual:
pub fn retry(max_attempts: &serde_json::Value) {
    fn decorator(f: ()) -> () {
        fn wrapper() -> () {
            // Broken nested structure
        }
    }
}
```

**Root Cause:** Decorator expansion doesn't preserve function types through nesting.

**Fix:** Implement proper closure-based decorator expansion with type preservation.

---

## Issue 148: Mutable Parameter Passed as Immutable Reference

**Affected Categories:** interprocedural-mutation  
**Failure Count:** ~10+

**Problem:** When calling helper functions with `&mut` params, `&settings` is passed instead of `settings`.

**Examples:**
```rust
// Expected:
pub fn increase_volume(settings: &mut Settings) {
    let current = get_volume(settings);
    set_volume(settings, current + 10);
}

// Actual:
pub fn increase_volume(settings: &mut Settings) {
    let current = get_volume(&settings);  // Wrong: &settings (immutable)
    set_volume(&settings, current + 10);  // Wrong: &settings (immutable)
}
```

**Root Cause:** Automatic reference insertion doesn't respect parameter mutability.

**Fix:** When passing `&mut T` to a function expecting `&T`, pass `settings` not `&settings`.

---

## Issue 149: Return Type Should Be Mutable Reference for Mutation

**Affected Categories:** return-value-mutation  
**Failure Count:** ~10+

**Problem:** Functions returning references for mutation should return `&mut T` not `T`.

**Examples:**
```rust
// Expected:
pub fn get_items<'a>(c: &'a mut Container) -> &'a mut Vec<i32> {
    return &mut c.items;
}

// Actual:
pub fn get_items(c: &Container) -> Vec<i32> {  // Returns owned, not mut ref
    return c.items;  // Won't compile - can't move out of borrow
}
```

**Root Cause:** Mutation analysis doesn't propagate to return type.

**Fix:** When return value is mutated by caller, return `&mut T`.

---

## Issue 150: Regex `re.compile` Uses Invalid Module-Level const

**Affected Categories:** modules-regex  
**Failure Count:** ~10+

**Problem:** `re.compile()` at module level generates invalid `pub const` with `serde_json::Value`.

**Examples:**
```python
# Python
import re
pattern = re.compile(r"\d+")
```
```rust
// Expected:
lazy_static! {
    static ref PATTERN: Regex = Regex::new(r"\d+").unwrap();
}

// Actual:
pub const pattern: serde_json::Value = regex::Regex::new("\\d+").unwrap();  // Invalid const!
```

**Root Cause:** Regex compilation result can't be const; needs lazy_static.

**Fix:** Use `lazy_static!` or `once_cell::Lazy` for compiled regex patterns.

---

## Issue 151: Missing Derive Bracket Closure

**Affected Categories:** ast-hir-codegen  
**Failure Count:** ~10+

**Problem:** `#[derive(...)]` attribute sometimes missing closing bracket.

**Examples:**
```rust
// Expected:
#[derive(Debug, Copy, Clone)]
pub struct Point { ... }

// Actual:
#[derive(Debug, Copy, Clone]  // Missing ]
pub struct Point { ... }
```

**Root Cause:** Code generation bug in derive attribute formatting.

**Fix:** Ensure derive macros have proper bracket closure.

---

## Issue 152: HashMap Initialization Style Inconsistency

**Affected Categories:** import-deduplication  
**Failure Count:** ~10+

**Problem:** HashMap initialization uses verbose multi-line pattern instead of `into_iter().collect()`.

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

**Root Cause:** No standardization on HashMap initialization idiom.

**Fix:** Use consistent pattern: either always verbose or always functional.

---

## Issue 153: Expression Type Not Supported Error

**Affected Categories:** examples-game-development  
**Failure Count:** ~10+

**Problem:** Complex expressions (like list comprehensions in class initialization) cause "Expression type not yet supported" errors.

**Examples:**
```
// Expected: Complex class with 2D board initialized

// Actual:
Error: Expression type not yet supported
```

**Root Cause:** List comprehension in class field default not implemented.

**Fix:** Support list comprehensions in struct field initialization.

---

## Issue 154: String `.encode()` Method Generates Redundant Code

**Affected Categories:** string-encoding  
**Failure Count:** ~16+

**Problem:** `s.encode("utf-8")` is unnecessary in Rust but still generated, and double cast added.

**Examples:**
```rust
// Expected:
pub fn byte_length(s: &str) -> i32 {
    return s.len() as i32;
}

// Actual:
pub fn byte_length(s: String) -> i32 {
    return s.encode("utf-8").len() as i32 as i32;  // .encode() doesn't exist, double cast
}
```

**Root Cause:** Python string encoding method not properly translated.

**Fix:** Rust strings are UTF-8 by default; `.encode("utf-8")` becomes no-op.

---

## Issue 155: Block Expression Missing Semicolon After Close Brace

**Affected Categories:** stdlib-misc  
**Failure Count:** ~15+

**Problem:** Block expressions used as return values missing semicolon after closing brace.

**Examples:**
```rust
// Expected:
pub fn generate_uuid4() -> String {
    return {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    };
}

// Actual:
pub fn generate_uuid4() -> String {
    return {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    }  // Missing semicolon
}
```

**Root Cause:** Block expression as return value not terminated properly.

**Fix:** Add semicolon after block expressions used as return values.

---

## Summary of New Issues (136-155)

| Issue | Category | Impact | Fix Complexity |
|-------|----------|--------|----------------|
| 136 | Implicit Return | Low | Easy |
| 137 | Mut Ref Recursive | High | Medium |
| 138 | Copy Derive | High | Medium |
| 139 | Mut Self Methods | High | Medium |
| 140 | PartialEq Dunder | High | Medium |
| 141 | Async Iterator | High | Hard |
| 142 | Contains Method | Medium | Easy |
| 143 | Type Hints Ignored | Critical | Medium |
| 144 | Nested Ternary | Medium | Medium |
| 145 | AddAssign Dunder | Medium | Medium |
| 146 | Vec Ownership | Medium | Medium |
| 147 | Decorator Pattern | High | Hard |
| 148 | Mut Param Passing | High | Medium |
| 149 | Mut Return Type | High | Medium |
| 150 | Regex Const | Medium | Easy |
| 151 | Derive Bracket | Low | Easy |
| 152 | HashMap Init | Low | Easy |
| 153 | Expression Error | Medium | Hard |
| 154 | String Encode | Medium | Easy |
| 155 | Block Semicolon | Low | Easy |

**Key Observations (Issues 136-155):**
- Issues 137, 138, 139, 140, 141, 147, 148, 149 relate to Rust's ownership/borrowing model
- Issue 143 (type hints ignored) is critical - explicit Python types should be respected
- Issues 140, 145 are dunder-to-trait mapping issues
- Issues 150, 151, 152, 155 are relatively easy syntax fixes

---

## Issue 156: Walrus Operator (`:=`) Generates Empty Output

**Affected Categories:** walrus-operator  
**Failure Count:** ~4+

**Problem:** Python walrus operator (assignment expression) generates empty output.

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
    log::info!("{}", format!("List has {} items", n));
}

// Actual: (empty output)
```

**Root Cause:** Walrus operator (`:=`) not implemented.

**Fix:** Extract assignment from expression, place before conditional.

---

## Issue 157: `__slots__` Generates Invalid Const Vec

**Affected Categories:** slots  
**Failure Count:** ~18+

**Problem:** `__slots__` class attribute generates invalid `const` with `Vec`.

**Examples:**
```python
# Python
class Point:
    __slots__ = ['x', 'y']
    def __init__(self, x, y):
        self.x = x
        self.y = y
```
```rust
// Expected:
struct Point {
    x: i32,
    y: i32,
}

// Actual:
pub struct Point { pub x: serde_json::Value, pub y: serde_json::Value }
impl Point {
    pub const __slots__: Vec<String> = vec![...];  // Invalid: Vec can't be const!
}
```

**Root Cause:** `__slots__` incorrectly translated to const field instead of being used for struct layout hints.

**Fix:** Use `__slots__` for memory layout hints, not generate as a field.

---

## Issue 158: Protocol (Typing) Not Translated to Trait

**Affected Categories:** protocols  
**Failure Count:** ~12+

**Problem:** `typing.Protocol` classes aren't translated to Rust traits.

**Examples:**
```python
# Python
from typing import Protocol

class Drawable(Protocol):
    def draw(self) -> str: ...

class Circle:
    def draw(self) -> str:
        return f"Circle with radius {self.radius}"

def render(shape: Drawable) -> str:
    return shape.draw()
```
```rust
// Expected:
pub trait Drawable {
    fn draw(&self) -> String;
}
impl Drawable for Circle { ... }
pub fn render(shape: &impl Drawable) -> String { ... }

// Actual:
pub struct Circle { ... }
impl Circle {
    pub fn draw(&self) -> String { ... }
}
pub fn render(shape: &Drawable) -> String {  // Invalid: Drawable not a trait
    return shape.draw();
}
```

**Root Cause:** `Protocol` classes not recognized as trait definitions.

**Fix:** Map `typing.Protocol` to `trait` definition.

---

## Issue 159: Metaclass Generates Empty Struct

**Affected Categories:** metaclasses  
**Failure Count:** ~15+

**Problem:** Classes using metaclasses lose the metaclass functionality.

**Examples:**
```python
# Python
class Meta(type):
    def __new__(mcs, name, bases, attrs):
        attrs['added_by_meta'] = True
        return super().__new__(mcs, name, bases, attrs)

class MyClass(metaclass=Meta):
    pass

obj = MyClass()
print(hasattr(obj, 'added_by_meta'))  # True
```
```rust
// Expected:
struct MyClass {
    added_by_meta: bool,
}
impl Default for MyClass {
    fn default() -> Self {
        MyClass { added_by_meta: true }
    }
}

// Actual:
pub struct Meta {}
pub struct MyClass {}  // Missing added_by_meta!
```

**Root Cause:** Metaclass functionality not transpiled.

**Fix:** Analyze metaclass `__new__` and inject attributes/methods into class.

---

## Issue 160: NamedTuple Generates Invalid Import

**Affected Categories:** namedtuple  
**Failure Count:** ~30+

**Problem:** `collections.namedtuple` generates `use std::collections::namedtuple` which doesn't exist.

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
use std::collections::namedtuple;  // Doesn't exist!
```

**Root Cause:** `namedtuple` not recognized as struct factory.

**Fix:** Map `namedtuple` to struct definition with fields.

---

## Issue 161: Global Variables Need `thread_local!` or `static`

**Affected Categories:** global-nonlocal  
**Failure Count:** ~20+

**Problem:** Python `global` keyword not translated to proper Rust global patterns.

**Examples:**
```python
# Python
x = 10
def modify():
    global x
    x = 20

modify()
print(x)  # 20
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
    let x = 20;  // Local variable, not global!
}
```

**Root Cause:** Global statement creates local shadowing instead of global mutation.

**Fix:** Use `thread_local!` or `static` with appropriate synchronization.

---

## Issue 162: Lambda in `sorted()` Key Function Works But Test Code Missing

**Affected Categories:** lambdas  
**Failure Count:** ~13+

**Problem:** Lambda functions for sorting work but test verification code is missing.

**Examples:**
```rust
// Expected to have test verification like:
for i in 1..result.len() {
    if result[i - 1] > result[i] {
        return TestResult::failed();
    }
}

// Actual: Test code truncated or missing
```

**Root Cause:** Test verification code not generated.

**Fix:** Generate complete test functions including assertions.

---

## Issue 163: Generator State Assignment Missing

**Affected Categories:** generators  
**Failure Count:** ~15+

**Problem:** Generator state machines don't include variable assignments.

**Examples:**
```rust
// Expected:
impl Iterator for CounterState {
    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            0 => {
                self.x = 1;  // Assignment present
                self.state = 1;
                return Some(self.x);
            }
            ...
        }
    }
}

// Actual: Missing `self.x = 1;`
```

**Root Cause:** Variable assignments within generators not captured.

**Fix:** Track assignments in generator body and emit them in state machine.

---

## Issue 164: Dict Comprehension Returns Wrong Key Type

**Affected Categories:** comprehensions  
**Failure Count:** ~15+

**Problem:** Dict comprehensions return `HashMap<serde_json::Value, _>` instead of proper key type.

**Examples:**
```python
# Python
def make_dict_comp() -> dict[str, int]:
    return {str(x): x * 2 for x in range(5)}
```
```rust
// Expected:
pub fn make_dict_comp() -> HashMap<String, i32> { ... }

// Actual:
pub fn make_dict_comp() -> HashMap<serde_json::Value, i32> { ... }
```

**Root Cause:** Dict comprehension key type not inferred from return type annotation.

**Fix:** Use return type annotation to determine key type.

---

## Issue 165: Positional-Only Parameters (`/`) Lose All Parameters

**Affected Categories:** positional-only-args  
**Failure Count:** ~15+

**Problem:** Functions with positional-only parameter syntax lose all parameters.

**Examples:**
```python
# Python
def greet(name, /):  # name is positional-only
    return f"Hello, {name}!"
```
```rust
// Expected:
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Actual:
pub fn greet() -> String {  // All parameters missing!
    return format!("Hello, {}!", name);
}
```

**Root Cause:** Positional-only parameter marker (`/`) causes parameter parsing failure.

**Fix:** Parse `/` as marker but don't affect parameter generation.

---

## Issue 166: Match Statement Guards Generate Empty Output

**Affected Categories:** match-guards  
**Failure Count:** ~20+

**Problem:** Python structural pattern matching with guards generates empty output.

**Examples:**
```python
# Python
value = 5
match value:
    case x if x > 0:
        result = True
    case _:
        result = False
```
```rust
// Expected:
let value = 5;
let result = match value {
    x if x > 0 => true,
    _ => false,
};

// Actual: (empty output)
```

**Root Cause:** Pattern matching with guards not implemented.

**Fix:** Implement Rust match with guard syntax.

---

## Issue 167: Starred Unpacking Generates Empty Output

**Affected Categories:** unpacking  
**Failure Count:** ~15+

**Problem:** Starred expressions in unpacking (`*rest`) generate empty output.

**Examples:**
```python
# Python
data = [1, 2, 3, 4]
a, *rest = data
```
```rust
// Expected:
let _data = vec![1, 2, 3, 4];
let a = _data[0];
let rest: Vec<_> = _data[1..].to_vec();

// Actual: (empty output)
```

**Root Cause:** Starred unpacking (`*variable`) not implemented.

**Fix:** Generate slice operations for starred elements.

---

## Issue 168: Python Enum Generates Struct With Consts

**Affected Categories:** enum-basic, enum-advanced, enum-methods  
**Failure Count:** ~50+

**Problem:** Python `Enum` classes generate structs with constants instead of Rust enums.

**Examples:**
```python
# Python
from enum import Enum
class Color(Enum):
    RED = 1
    GREEN = 2
```
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

**Root Cause:** Enum class not recognized as enum definition.

**Fix:** Detect `Enum` base class and generate Rust `enum`.

---

## Issue 169: `@lru_cache` Decorator Not Implemented

**Affected Categories:** functools-cache  
**Failure Count:** ~25+

**Problem:** `@lru_cache` decorator doesn't generate caching code.

**Examples:**
```python
# Python
from functools import lru_cache

@lru_cache
def func(x):
    return x * x
```
```rust
// Expected:
use lru::LruCache;
lazy_static! {
    static ref CACHE: Mutex<LruCache<i32, i32>> = ...;
}
fn func(x: i32) -> i32 {
    let mut cache = CACHE.lock().unwrap();
    if let Some(&val) = cache.get(&x) { return val; }
    let val = x * x;
    cache.put(x, val);
    val
}

// Actual:
pub fn func(x: i32) {  // No caching!
    return x * x;
}
```

**Root Cause:** `@lru_cache` decorator not expanded to caching logic.

**Fix:** Generate LRU cache wrapper using `lazy_static` or `once_cell`.

---

## Issue 170: Dataclass Test Code Gets Extra Const Declarations

**Affected Categories:** dataclasses-basic, dataclasses-advanced  
**Failure Count:** ~60+

**Problem:** Dataclass test code generates unnecessary top-level const declarations.

**Examples:**
```rust
// Expected:
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Point { pub x: i32, pub y: i32 }

// Actual (extra lines):
pub const p: Point = Point::new(1, 2);  // Invalid: not const-constructible
pub const result: (i32, i32) = (p.x, p.y);
// ... then the struct definition
```

**Root Cause:** Test code generation creates invalid const declarations.

**Fix:** Generate test code in separate test function, not as top-level consts.

---

## Issue 171: Async Function Call Site Missing Await

**Affected Categories:** async-functions  
**Failure Count:** ~30+

**Problem:** Async function definitions work but call sites missing proper handling.

**Examples:**
```rust
// Expected:
pub const result: i32 = { 
    tokio::runtime::Runtime::new().unwrap().block_on(f())
};

// Actual:
pub const result: i32 = { f() };  // Missing await/runtime
```

**Root Cause:** Async function call doesn't include async runtime invocation.

**Fix:** Wrap async calls in `block_on` or use `await` inside async context.

---

## Issue 172: Context Manager `with` Generates `__enter__`/`__exit__` Methods

**Affected Categories:** context-managers  
**Failure Count:** ~25+

**Problem:** Context managers generate Python-style dunder methods instead of Rust `Drop`.

**Examples:**
```rust
// Expected:
impl Drop for File {
    fn drop(&mut self) {
        self.closed = true;
    }
}

// Actual:
pub fn __enter__(&self) -> &Self { return self; }
pub fn __exit__(...) -> bool { self.closed = true; return false; }
```

**Root Cause:** Context managers not mapped to Rust RAII patterns.

**Fix:** Use `Drop` trait for cleanup, generate scope-based resource handling.

---

## Issue 173: `@property` Generates serde_json::Value Fields

**Affected Categories:** properties  
**Failure Count:** ~18+

**Problem:** Properties that access private fields generate `serde_json::Value` types.

**Examples:**
```python
# Python
class Circle:
    def __init__(self, radius: float):
        self._radius = radius
    
    @property
    def area(self) -> float:
        return 3.14159 * self._radius ** 2
```
```rust
// Expected:
pub struct Circle { radius: f64 }
impl Circle {
    fn area(&self) -> f64 { 3.14159 * self.radius * self.radius }
}

// Actual:
pub struct Circle { pub _radius: serde_json::Value }  // Wrong type!
```

**Root Cause:** Property backing field type not inferred from constructor type hint.

**Fix:** Infer field types from `__init__` parameter type hints.

---

## Issue 174: Descriptors Not Mapped to Rust Patterns

**Affected Categories:** descriptors  
**Failure Count:** ~18+

**Problem:** Python descriptor protocol (`__get__`, `__set__`) not translated.

**Examples:**
```rust
// Expected:
impl fmt::Display for MyClass { ... }

// Actual:
pub struct Descriptor {}
pub struct MyClass {}
impl MyClass {
    pub const desc: serde_json::Value = Descriptor::new();  // Invalid
}
```

**Root Cause:** Descriptor protocol not recognized.

**Fix:** Map descriptors to appropriate Rust patterns (methods, traits, etc.).

---

## Issue 175: Multiple Inheritance Generates Separate Structs

**Affected Categories:** multiple-inheritance  
**Failure Count:** ~20+

**Problem:** Mixins and multiple inheritance generate separate unrelated structs.

**Examples:**
```python
# Python
class Mixin1:
    def feature1(self): return "feature1"

class MyClass(Mixin1, Base):
    pass
```
```rust
// Expected (using traits):
trait Mixin1 {
    fn feature1(&self) -> &str { "feature1" }
}
impl Mixin1 for MyClass {}

// Actual:
pub struct Mixin1 {}  // Separate struct, not trait!
pub struct MyClass {}  // No inheritance relationship
```

**Root Cause:** Multiple inheritance not mapped to Rust trait system.

**Fix:** Map mixin classes to traits, compose with `impl Trait for Struct`.

---

## Issue 176: ABC `@abstractmethod` Generates Empty Function Body

**Affected Categories:** abc  
**Failure Count:** ~19+

**Problem:** Abstract methods generate struct with empty function body instead of trait.

**Examples:**
```python
# Python
from abc import ABC, abstractmethod

class Base(ABC):
    @abstractmethod
    def method(self) -> int: ...
```
```rust
// Expected:
trait Base {
    fn method(&self) -> i32;
}

// Actual:
pub struct Base {}
impl Base {
    pub fn method(&self) -> i32 {
        {}  // Empty body!
    }
}
```

**Root Cause:** ABC pattern not recognized as trait definition.

**Fix:** Map `ABC` with `@abstractmethod` to trait definition.

---

## Issue 177: Exception Groups (`except*`) Not Supported

**Affected Categories:** exception-groups  
**Failure Count:** ~20+

**Problem:** Python 3.11 exception groups with `except*` fail to transpile.

**Examples:**
```python
# Python
try:
    raise ExceptionGroup("errors", [ValueError("a"), TypeError("b")])
except* ValueError:
    pass
```
```
// Error: Statement type not yet supported: unknown
```

**Root Cause:** `except*` syntax (Python 3.11+) not implemented.

**Fix:** Add support for exception group syntax, map to Rust error handling.

---

## Issue 178: VecDeque Operations Truncated

**Affected Categories:** deque  
**Failure Count:** ~30+

**Problem:** `collections.deque` operations only generate import, missing body.

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
// Body missing!
```

**Root Cause:** Module-level code after imports not generated.

**Fix:** Generate complete deque operations including initialization.

---

## Issue 179: ChainMap Generates Invalid Constructor Call

**Affected Categories:** chainmap  
**Failure Count:** ~30+

**Problem:** `ChainMap` generates call to non-existent stdlib type.

**Examples:**
```python
# Python
from collections import ChainMap
cm = ChainMap({"a": 1}, {"b": 2})
```
```rust
// Expected (custom implementation):
pub struct ChainMap { maps: Vec<HashMap<String, i32>> }
impl ChainMap {
    pub fn new(maps: Vec<HashMap<_, _>>) -> Self { ... }
}

// Actual:
use std::collections::ChainMap;  // Doesn't exist!
return ChainMap::new(dict1, dict2);
```

**Root Cause:** `ChainMap` mapped to non-existent Rust type.

**Fix:** Generate custom ChainMap implementation or use similar pattern.

---

## Issue 180: OrderedDict Uses Non-Existent IndexMap

**Affected Categories:** ordereddict  
**Failure Count:** ~25+

**Problem:** `OrderedDict` generates import of `std::collections::IndexMap` which doesn't exist.

**Examples:**
```python
# Python
from collections import OrderedDict
od = OrderedDict()
od["a"] = 1
```
```rust
// Expected:
use indexmap::IndexMap;  // External crate
let mut od = IndexMap::new();

// Actual:
use std::collections::IndexMap;  // Doesn't exist in std!
```

**Root Cause:** `IndexMap` not in std, needs `indexmap` crate.

**Fix:** Use `indexmap` crate or note HashMap preserves insertion order in Rust 1.36+.

---

## Issue 181: Complex Numbers Generate Empty Output

**Affected Categories:** complex-numbers  
**Failure Count:** ~25+

**Problem:** Complex number literals/operations generate empty output.

**Examples:**
```python
# Python
x = 3 + 4j
print(x.real, x.imag)
```
```rust
// Expected:
use num::Complex;
let x = Complex::new(3.0, 4.0);
log::info!("{}", x.re);
log::info!("{}", x.im);

// Actual: (empty output)
```

**Root Cause:** Complex number support not implemented.

**Fix:** Use `num` crate's `Complex<f64>` for complex numbers.

---

## Issue 182: Binary/Hex/Octal Literals Lose Original Base

**Affected Categories:** binary-literals  
**Failure Count:** ~25+

**Problem:** Binary/hex/octal literals converted to decimal, losing readability.

**Examples:**
```python
# Python
x = 0b1010
result = x == 10
```
```rust
// Expected:
let x = 0b1010;  // Keep binary representation
let result = x == 10;

// Actual:
pub const x: i32 = 10;  // Lost binary format
```

**Root Cause:** Numeric literals normalized to decimal.

**Fix:** Preserve original numeric base in generated code.

---

## Issue 183: Bytes Literal Returns Invalid `bytes` Type

**Affected Categories:** bytes-strings, bytes  
**Failure Count:** ~40+

**Problem:** Bytes literals return invalid `bytes` type instead of `Vec<u8>`.

**Examples:**
```python
# Python
def test_bytes_literal() -> bytes:
    return b"hello"
```
```rust
// Expected:
pub fn test_bytes_literal() -> Vec<u8> {
    return b"hello".to_vec();
}

// Actual:
pub fn test_bytes_literal() -> bytes {  // `bytes` is not a Rust type!
    return b"hello";
}
```

**Root Cause:** Python `bytes` type not mapped to `Vec<u8>`.

**Fix:** Map `bytes` to `Vec<u8>` or `&[u8]`.

---

## Issue 184: F-String at Module Level Generates Invalid Const

**Affected Categories:** fstrings, fstrings-debug  
**Failure Count:** ~35+

**Problem:** F-strings at module level generate invalid const with `format!`.

**Examples:**
```python
# Python
x = 42
result = f"Value: {x}"
```
```rust
// Expected:
let x = 42;
let result = format!("Value: {}", x);

// Actual:
pub const x: i32 = 42;
pub const result: serde_json::Value = format!("Value: {}", x);  // Invalid: format! not const
```

**Root Cause:** F-strings can't be const-evaluated.

**Fix:** Generate module-level f-strings in lazy_static or main function.

---

## Issue 185: Raw Strings Generate Valid Code But Wrong Context

**Affected Categories:** raw-strings  
**Failure Count:** ~20+

**Problem:** Raw strings work but module-level comparison generates invalid const.

**Examples:**
```python
# Python
s = r"\n"
result = s == "\\n"
```
```rust
// Expected:
let s = r"\n";
let result = s == "\\n";

// Actual:
pub const s: &str = "\\n";  // Raw string properly escaped
pub const result: serde_json::Value = s == "\\n";  // Invalid const comparison
```

**Root Cause:** Module-level expressions generate invalid const declarations.

**Fix:** Wrap module-level computations in main/test function.

---

## Issue 186: isinstance Type Narrowing Doesn't Clone

**Affected Categories:** type-guards  
**Failure Count:** ~12+

**Problem:** After isinstance check, value not cloned for multiplication.

**Examples:**
```python
# Python
def process(value) -> int:
    if isinstance(value, int):
        return value * 2
    return 0
```
```rust
// Expected:
if true {
    return value.clone() * 2;
}

// Actual:
if true {
    return value * 2;  // May need clone for move semantics
}
```

**Root Cause:** Type narrowing doesn't account for move semantics.

**Fix:** Add `.clone()` when needed for moved values.

---

## Issue 187: Threading Function Takes Owned Instead of Borrowed

**Affected Categories:** concurrency  
**Failure Count:** ~15+

**Problem:** Thread worker functions take `String` instead of `&str`.

**Examples:**
```rust
// Expected:
pub fn worker(name: &str) -> String {
    return format!("Hello from {}", name);
}
pub fn run_thread() -> String {
    return worker("main");
}

// Actual:
pub fn worker(name: String) -> String {
    return format!("Hello from {}", name);
}
pub fn run_thread() -> String {
    return worker("main".to_string());  // Unnecessary allocation
}
```

**Root Cause:** String parameters default to owned instead of borrowed.

**Fix:** Prefer `&str` for read-only string parameters.

---

## Issue 188: ctypes `c_float` Maps to `f64` Instead of `f32`

**Affected Categories:** ffi-ctypes  
**Failure Count:** ~15+

**Problem:** C types not correctly mapped to Rust equivalents.

**Examples:**
```rust
// Expected:
pub fn c_float_to_float(value: f32) -> f32 { ... }

// Actual:
pub fn c_float_to_float(value: f64) -> f64 { ... }  // Wrong size!
```

**Root Cause:** C type mapping incorrect (c_float is f32, not f64).

**Fix:** Map ctypes correctly: c_float→f32, c_double→f64.

---

## Issue 189: Simple Division Generates Unnecessary Error Type

**Affected Categories:** numeric-precision, serialization  
**Failure Count:** ~20+

**Problem:** Even safe division patterns generate ZeroDivisionError.

**Examples:**
```python
# Python
def safe_div(a: float, b: float) -> float:
    if b == 0.0:
        return 0.0
    return a / b
```
```rust
// Expected:
pub fn safe_div(a: f64, b: f64) -> f64 {
    if b == 0.0 { return 0.0; }
    return a / b;
}

// Actual:
pub struct ZeroDivisionError { ... }  // Unnecessary!
pub fn safe_div(a: f64, b: f64) -> Result<f64, ZeroDivisionError> { ... }
```

**Root Cause:** Division always triggers error type generation even when guarded.

**Fix:** Detect guard conditions that prevent division by zero.

---

## Issue 190: `#[inline]` Attribute Not Generated for Simple Accessors

**Affected Categories:** performance-patterns  
**Failure Count:** ~15+

**Problem:** Simple getter methods don't get `#[inline]` attribute.

**Examples:**
```rust
// Expected:
#[inline]
pub fn get_x(&self) -> i32 { return self.x; }

// Actual:
pub fn get_x(&self) -> i32 { return self.x; }  // Missing #[inline]
```

**Root Cause:** Optimization hints not generated.

**Fix:** Add `#[inline]` for simple accessor methods.

---

## Issue 191: Verification Assert Wraps in Unnecessary Result

**Affected Categories:** verification-contracts  
**Failure Count:** ~15+

**Problem:** Functions with assert still generate Result types.

**Examples:**
```python
# Python
def divide(a: int, b: int) -> int:
    assert b != 0, "divisor cannot be zero"
    return a // b
```
```rust
// Expected:
pub fn divide(a: i32, b: i32) -> i32 {
    assert!(b != 0, "divisor cannot be zero");
    return a / b;
}

// Actual:
pub fn divide(a: i32, b: i32) -> Result<i32, ZeroDivisionError> {  // Unnecessary Result
    assert!(b != 0, ...);
    return Ok(...);
}
```

**Root Cause:** Assert doesn't prevent Result type generation.

**Fix:** Detect assertions that guard against error conditions.

---

## Issue 192: String Concat Uses Format Instead of Plus

**Affected Categories:** lifetime-parameters  
**Failure Count:** ~5+

**Problem:** String concatenation uses `format!` instead of `+` operator.

**Examples:**
```rust
// Expected:
pub fn concat_strings(a: String, b: String) -> String {
    return a + &b;
}

// Actual:
pub fn concat_strings(a: String, b: String) -> String {
    return format!("{}{}", a, b);  // Less efficient
}
```

**Root Cause:** String concat not optimized to use `+` operator.

**Fix:** Use `a + &b` for simple concatenation.

---

## Issue 193: copy.copy Generates Double Clone

**Affected Categories:** regressions  
**Failure Count:** ~5+

**Problem:** `copy.copy()` generates redundant `.clone().clone()`.

**Examples:**
```rust
// Expected:
let mut copied = original.clone();

// Actual:
let mut copied = (original.clone()).clone();  // Double clone!
```

**Root Cause:** Copy operation applied twice.

**Fix:** Generate single `.clone()` for `copy.copy()`.

---

## Issue 194: `len()` Cast Generates Double Cast

**Affected Categories:** regressions  
**Failure Count:** ~10+

**Problem:** `len()` result cast twice to i32.

**Examples:**
```rust
// Expected:
return original.len() as i32;

// Actual:
return original.len() as i32 as i32;  // Double cast!
```

**Root Cause:** Cast applied at multiple points in code generation.

**Fix:** Track casts and avoid duplication.

---

## Issue 195: FrozenSet Generates Empty Output

**Affected Categories:** set-operations  
**Failure Count:** ~5+

**Problem:** `frozenset` operations generate empty output.

**Examples:**
```python
# Python
result = frozenset([1, 2, 3])
```
```rust
// Expected:
pub const result: std::sync::Arc<HashSet<i32>> =
    std::sync::Arc::new(vec![1, 2, 3].into_iter().collect::<HashSet<_>>());

// Actual: (empty output)
```

**Root Cause:** `frozenset` not implemented.

**Fix:** Map to `Arc<HashSet<T>>` or immutable set pattern.

---

## Summary of New Issues (156-195)

| Issue | Category | Impact | Fix Complexity |
|-------|----------|--------|----------------|
| 156 | Walrus Operator | Medium | Medium |
| 157 | __slots__ | Medium | Easy |
| 158 | Protocol/Trait | High | Hard |
| 159 | Metaclass | High | Hard |
| 160 | NamedTuple | High | Medium |
| 161 | Global Variables | High | Hard |
| 162 | Lambda Tests | Low | Easy |
| 163 | Generator State | Medium | Medium |
| 164 | Dict Comprehension | Medium | Easy |
| 165 | Positional-Only | Medium | Easy |
| 166 | Match Guards | High | Hard |
| 167 | Starred Unpacking | Medium | Medium |
| 168 | Python Enum | High | Medium |
| 169 | lru_cache | High | Hard |
| 170 | Dataclass Tests | Low | Easy |
| 171 | Async Call Site | High | Medium |
| 172 | Context Manager | High | Hard |
| 173 | Property Types | Medium | Medium |
| 174 | Descriptors | Medium | Hard |
| 175 | Multiple Inheritance | High | Hard |
| 176 | ABC Abstract | High | Medium |
| 177 | Exception Groups | Medium | Hard |
| 178 | VecDeque | Medium | Easy |
| 179 | ChainMap | Medium | Medium |
| 180 | OrderedDict | Medium | Easy |
| 181 | Complex Numbers | Medium | Medium |
| 182 | Binary Literals | Low | Easy |
| 183 | Bytes Type | High | Easy |
| 184 | F-String Const | Medium | Medium |
| 185 | Raw String Const | Low | Easy |
| 186 | isinstance Clone | Medium | Easy |
| 187 | Thread Owned | Low | Easy |
| 188 | ctypes Mapping | Medium | Easy |
| 189 | Division Error | Medium | Medium |
| 190 | Inline Attribute | Low | Easy |
| 191 | Assert Result | Medium | Medium |
| 192 | String Concat | Low | Easy |
| 193 | Double Clone | Low | Easy |
| 194 | Double Cast | Low | Easy |
| 195 | FrozenSet | Medium | Medium |

**Key Observations (Issues 156-195):**
- Issues 158, 159, 175, 176 relate to Python's OOP features → Rust traits
- Issues 168, 169, 177 need Python-specific feature support (Enum, lru_cache, except*)
- Issues 160, 179, 180, 181 need stdlib type mappings
- Issues 182-185, 192-194 are relatively easy code generation fixes
- Issues 161, 166, 167 relate to Python syntax that needs careful translation