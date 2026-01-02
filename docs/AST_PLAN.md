# Python AST Statement Support Plan

This document outlines the Python statement types from `rustpython_ast::Stmt` and their support status in Depyler's AST bridge (`ast_bridge.rs` and `converters.rs`).

## Summary

| Status | Count | Description |
|--------|-------|-------------|
| ✅ Supported | 25 | Fully converted to HIR |
| 🔶 Partial | 2 | Module-level only or limited |
| ❌ Not Supported | 4 | Not yet implemented |

---

## Fully Supported Statements (✅)

These statements are fully converted from Python AST to Depyler HIR:

| Statement | Python Example | Notes |
|-----------|----------------|-------|
| `FunctionDef` | `def foo(): ...` | Both top-level and nested functions |
| `AsyncFunctionDef` | `async def foo(): ...` | Full async support |
| `Return` | `return x` | With or without value |
| `Delete` | `del x` | Single and multiple targets |
| `Assign` | `x = 1` | Single target only (chained assignment unsupported) |
| `AugAssign` | `x += 1` | All augmented operators |
| `AnnAssign` | `x: int = 1` | With or without initialization |
| `For` | `for x in items:` | Standard iteration |
| `AsyncFor` | `async for x in items:` | Async iteration |
| `While` | `while cond:` | Including walrus operator patterns |
| `If` | `if cond:` | With elif/else chains |
| `With` | `with ctx:` | Multiple context managers supported |
| `AsyncWith` | `async with ctx:` | Single context manager only |
| `Raise` | `raise Error` | With optional cause |
| `Try` | `try: ... except:` | With handlers, else, finally |
| `Assert` | `assert cond` | With optional message |
| `Import` | `import os` | With alias support |
| `ImportFrom` | `from os import path` | With alias support |
| `Global` | `global x` | Variable declarations |
| `Nonlocal` | `nonlocal x` | Closure variable declarations |
| `Expr` | `foo()` | Expression statements |
| `Pass` | `pass` | No-op statement |
| `Break` | `break` | Loop control |
| `Continue` | `continue` | Loop control |

---

## Partially Supported Statements (🔶)

### `ClassDef`
**Python:** `class Foo: ...`

**Current Status:** Module-level classes are converted to HIR classes/protocols in `ast_bridge.rs`. Nested class definitions inside functions return an error.

**Recommendation:** **Should Support** - Nested classes are rare in Python but occasionally used for encapsulation patterns. Low priority as the module-level use case covers 95%+ of real-world code.

---

### `TypeAlias` (Python 3.12+)
**Python:** `type Point = tuple[int, int]`

**Current Status:** Handled at module level in `ast_bridge.rs` via `convert_type_alias_stmt`. Not handled in `StmtConverter` for nested contexts.

**Recommendation:** **Should Support** - The new `type` statement syntax is becoming standard. Module-level support is sufficient for now since type aliases are almost always defined at module scope.

---

## Not Supported Statements (❌)

### `Match` (Pattern Matching)
**Python:** 
```python
match value:
    case 1:
        ...
    case [x, y]:
        ...
```

**Current Status:** Returns error: "Statement type not yet supported: Match"

**Recommendation:** **Should Support (High Priority)** - Pattern matching (Python 3.10+) maps well to Rust's `match` expression. Benefits include:
- Direct semantic equivalent in Rust
- Enables idiomatic Python code transpilation  
- Growing adoption in Python ecosystem
- Supports exhaustiveness checking

**Implementation Complexity:** Medium - Requires handling match subjects, case patterns (literal, capture, wildcard, class, sequence, mapping, OR patterns), and guards.

---

### `TryStar` (Exception Groups)
**Python:**
```python
try:
    ...
except* ValueError:
    ...
```

**Current Status:** Falls through to generic error: "Statement type not yet supported: unknown"

**Recommendation:** **Should Not Support (Low Priority)** - Exception groups (Python 3.11+) are:
- Designed for concurrent exception handling (`asyncio.TaskGroup`)
- No direct Rust equivalent
- Rarely used outside async concurrent contexts
- Complex semantics that don't map cleanly to Rust's error handling

**Alternative:** Document that users should refactor exception group code to standard try/except before transpilation.

---

## Detailed Analysis of Unsupported Statements

### Why Support `Match`?

1. **Semantic Alignment:** Python's `match` and Rust's `match` share core concepts:
   - Pattern matching with variable binding
   - Guards (`if` conditions)
   - Destructuring sequences and mappings
   
2. **Code Quality:** Pattern matching produces cleaner code than if/elif chains:
   ```python
   # Python
   match command:
       case ["quit"]:
           return False
       case ["load", filename]:
           load_file(filename)
   ```
   ```rust
   // Rust
   match command {
       ["quit"] => return false,
       ["load", filename] => load_file(filename),
   }
   ```

3. **Pattern Types to Support:**
   - `MatchValue` - Literal patterns
   - `MatchSingleton` - `True`, `False`, `None`
   - `MatchSequence` - List/tuple unpacking
   - `MatchMapping` - Dict patterns
   - `MatchClass` - Type patterns
   - `MatchStar` - Capture rest (`*rest`)
   - `MatchAs` - Capture/wildcard patterns
   - `MatchOr` - Alternative patterns

### Why Not Support `TryStar`?

1. **No Rust Equivalent:** Rust doesn't have exception groups. The closest patterns are:
   - `Result<T, Vec<E>>` - Awkward and uncommon
   - Custom error aggregation types
   
2. **Limited Use Case:** Exception groups are primarily for:
   - `asyncio.TaskGroup` concurrent task failures
   - `exceptiongroup` library for backporting
   
3. **Transformation Required:** Users would need to significantly restructure code:
   ```python
   # Python 3.11+
   try:
       async with asyncio.TaskGroup() as tg:
           tg.create_task(coro1())
           tg.create_task(coro2())
   except* ValueError as eg:
       for exc in eg.exceptions:
           handle(exc)
   ```
   
   No clean Rust equivalent exists without fundamentally changing the error handling strategy.

---

## Implementation Roadmap

### Phase 1: Match Statement (Priority: High)
1. Add `HirStmt::Match` variant to HIR
2. Implement `StmtConverter::convert_match`
3. Add pattern conversion for each pattern type
4. Implement Rust code generation for match expressions
5. Add comprehensive test coverage

### Phase 2: Nested Class Definitions (Priority: Low)
1. Allow `ClassDef` in `StmtConverter`
2. Generate inner struct definitions in Rust
3. Handle method visibility and access

### Phase 3: Documentation (Priority: Medium)
1. Document unsupported constructs
2. Provide migration guidance for exception groups
3. Add error messages suggesting alternatives

---

## Appendix: Full Statement Reference

```rust
pub enum Stmt<R = TextRange> {
    FunctionDef(StmtFunctionDef<R>),      // ✅ Supported
    AsyncFunctionDef(StmtAsyncFunctionDef<R>), // ✅ Supported
    ClassDef(StmtClassDef<R>),            // 🔶 Module-level only
    Return(StmtReturn<R>),                 // ✅ Supported
    Delete(StmtDelete<R>),                 // ✅ Supported
    Assign(StmtAssign<R>),                 // ✅ Supported
    TypeAlias(StmtTypeAlias<R>),           // 🔶 Module-level only
    AugAssign(StmtAugAssign<R>),           // ✅ Supported
    AnnAssign(StmtAnnAssign<R>),           // ✅ Supported
    For(StmtFor<R>),                       // ✅ Supported
    AsyncFor(StmtAsyncFor<R>),             // ✅ Supported
    While(StmtWhile<R>),                   // ✅ Supported
    If(StmtIf<R>),                         // ✅ Supported
    With(StmtWith<R>),                     // ✅ Supported
    AsyncWith(StmtAsyncWith<R>),           // ✅ Supported
    Match(StmtMatch<R>),                   // ❌ Not supported (should implement)
    Raise(StmtRaise<R>),                   // ✅ Supported
    Try(StmtTry<R>),                       // ✅ Supported
    TryStar(StmtTryStar<R>),               // ❌ Not supported (skip)
    Assert(StmtAssert<R>),                 // ✅ Supported
    Import(StmtImport<R>),                 // ✅ Supported
    ImportFrom(StmtImportFrom<R>),         // ✅ Supported
    Global(StmtGlobal<R>),                 // ✅ Supported
    Nonlocal(StmtNonlocal<R>),             // ✅ Supported
    Expr(StmtExpr<R>),                     // ✅ Supported
    Pass(StmtPass<R>),                     // ✅ Supported
    Break(StmtBreak<R>),                   // ✅ Supported
    Continue(StmtContinue<R>),             // ✅ Supported
}
```
