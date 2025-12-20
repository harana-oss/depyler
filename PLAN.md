# Test Failure Fix Plan

**Current Status:** ~120 TOML tests failing (down from 233, approximately 113 tests fixed)

## Priority 1: Critical Panics (33 tests) ✅ COMPLETED

These cause the transpiler to crash and should be fixed first.

### 1.1 Empty Ident Panic (21 tests) ✅
**Error:** `PANIC: Ident is not allowed to be empty; use Option<Ident>`

Affected files:
- `functools-partial.toml` (18 tests): `partial_basic`, `partial_call`, `partial_keyword`, `partial_multi_args`, `partial_mixed`, `partial_add_args`, `partial_func`, `partial_args`, `partial_keywords`, `partial_callback`, `partial_defaults`, `partial_adapter`, `partial_map`, `partial_vs_lambda`, `partial_advantages`, `partial_nested`, `partial_varargs`, `partial_kwargs_func`, `partial_override`
- `closures-late-binding.toml`: `late_binding_fix_partial`
- `closures.toml`: `closure_loop_fix_partial`
- `functools-wraps.toml`: `wraps_partial`
- `modules-itertools-functools.toml`: `functools_partial`

**Fix:** ✅ Implemented proper `functools.partial` → closure conversion in `expr_gen.rs`. Added `convert_partial_call()` and `infer_partial_remaining_params()` functions. Also skip empty `rust_name` mappings in `import_gen.rs`.

### 1.2 Invalid Ident Panic - os.path (6 tests) ✅
**Error:** `PANIC: "os.path" is not a valid Ident`

Affected files:
- `modules-os-sys.toml`: `os_path_exists`, `os_path_isfile`, `os_path_isdir`, `os_path_basename`, `os_path_dirname`, `os_path_relpath`
- `imports-basic.toml`: `import_dotted`

**Fix:** ✅ Fixed `module_mapper.rs` to handle dotted module paths by extracting the last component as the alias (e.g., "os.path" → "path").

### 1.3 Cast Method Call Panic (1 test) ✅
**Error:** `PANIC: casts cannot be followed by a method call`

Affected: `cast-expressions.toml` :: `nested_cast_expression`

**Fix:** ✅ Fixed `convert_str_conversion()` in `expr_gen.rs` to wrap cast expressions in parentheses: `(#arg).to_string()`.

### 1.4 Field KW-Only Sentinel Panic (1 test) ✅
**Error:** `PANIC: expected identifier or integer`

Affected: `dataclasses-field.toml` :: `field_kw_only_sentinel`

**Fix:** ✅ Fixed `direct_rules.rs` to filter out `KW_ONLY` sentinel fields (field name `_` with type `KW_ONLY`) in `convert_class_to_struct()`, `generate_dataclass_new()`, `generate_get_field_method()`, and `generate_set_field_method()`.

### 1.5 Multi-Dynamic Context Manager Panic (1 test) ✅
**Error:** `PANIC: expected one of: identifier, etc.`

Affected: `nested-context-managers.toml` :: `multi_dynamic`

**Fix:** ✅ Fixed comprehension functions in `expr_gen.rs` to handle Rust keywords (like `fn`) as loop variables using `syn::Ident::new_raw()`. Updated `convert_list_comp()`, `convert_set_comp()`, `convert_dict_comp()`, `convert_list_comp_first_element()`, and `parse_target_pattern()`.

---

## Priority 2: Statement Types Not Supported (~65 tests) ✅ MOSTLY COMPLETED

### 2.1 Global Statement (12 tests) ✅ COMPLETED
**Error:** `Statement type not yet supported: Global`

Affected files:
- `global-nonlocal.toml` (8 tests)
- `chained-comparisons.toml`, `contextlib-decorator.toml`, `functools-cache.toml` (4 tests)

**Fix:** ✅ Added `HirStmt::Global` variant to HIR, implemented AST conversion in `converters.rs`, and codegen in `stmt_gen.rs`. Emits as comment since Rust handles scoping differently.

### 2.2 Nonlocal Statement (15 tests) ✅ COMPLETED
**Error:** `Statement type not yet supported: Nonlocal`

Affected files:
- `closures-mutable.toml` (14 tests)
- `global-nonlocal.toml` (6 tests)

**Fix:** ✅ Added `HirStmt::Nonlocal` variant, implemented AST conversion and codegen. Emits as comment since Rust uses closure capture.

### 2.3 AsyncFor Statement (12 tests) ✅ COMPLETED
**Error:** `Statement type not yet supported: AsyncFor`

Affected files:
- `async-generators.toml` (10 tests)
- `async-comprehensions.toml` (1 test)

**Fix:** ✅ Added `HirStmt::AsyncFor` variant, converts to `while let Some(item) = stream.next().await { ... }` pattern.

### 2.4 AsyncWith Statement (5 tests) ✅ COMPLETED
**Error:** `Statement type not yet supported: AsyncWith`

Affected files:
- `asyncio-tasks.toml` (4 tests)
- `contextlib-utilities.toml` (3 tests)

**Fix:** ✅ Added `HirStmt::AsyncWith` variant, wraps body in async block with context binding.

### 2.5 AsyncFunctionDef Statement (2 tests)
**Error:** `Statement type not yet supported: AsyncFunctionDef`

Affected: `async-functions.toml`, `await-expressions.toml`

**Fix:** Implement async function definition transpilation.

### 2.6 Delete Statement (4 tests)
**Error:** `Statement type not yet supported: Delete`

Affected: `chainmap.toml`, `contextlib-decorator.toml`, `properties.toml`

**Fix:** Implement delete statement - translate to `drop()` or removal from collections.

### 2.7 Import Statement (2 tests)
**Error:** `Statement type not yet supported: Import`

Affected: `cli-integration.toml`, `imports-basic.toml`

**Fix:** Implement import statement transpilation for in-function imports.

### 2.8 ClassDef Statement (2 tests)
**Error:** `Statement type not yet supported: ClassDef (classes)`

Affected: `enum-advanced.toml`

**Fix:** Handle nested class definitions.

---

## Priority 3: Expression Types Not Supported (~30 tests) ✅ PARTIALLY COMPLETED

### 3.1 FString with Variables (19 tests) ✅ COMPLETED
**Error:** `Expression type not yet supported: FString with ...`

Affected files:
- `dunder-context.toml` (11 tests)
- `classes.toml` (2 tests)
- `examples-networking.toml`, `examples-tooling.toml`, `functools-partial.toml`, `keyword-only-args.toml`, `multiple-inheritance.toml`

**Fix:** ✅ Implemented f-string interpolation in `expr_gen.rs` and `direct_rules.rs`. Added `convert_fstring()` method to `ExprConverter` in `direct_rules.rs`. Converts f-strings to Rust `format!()` macro.

### 3.2 Slice with Step/Variables (6 tests)
**Error:** `Expression type not yet supported: Slice with ...`

Affected files:
- `class-methods.toml`: step -1
- `examples-data-structures.toml`, `examples-file-processing.toml`, `examples-tooling.toml`, `examples-web-scraping.toml`

**Fix:** Implement slice with step parameter and variable bounds.

### 3.3 Yield Expression (2 tests)
**Error:** `Expression type not yet supported: Yield`

Affected: `dunder-iteration.toml`

**Fix:** Implement generator yield expressions.

### 3.4 IfExpr Expression (1 test)
**Error:** `Expression type not yet supported: IfExpr`

Affected: `examples-game-development.toml`

**Fix:** Implement ternary/conditional expression.

### 3.5 Walrus Operator (3 tests)
**Error:** `Expression type not yet supported` (walrus/named expression)

Affected: `walrus-operator.toml`, `closures-late-binding.toml`

**Fix:** Implement walrus operator (`:=`) support in comprehensions and generators.

---

## Priority 4: String Method: format (~28 tests) ✅ COMPLETED

**Error:** `Unknown string method: format`

Affected files:
- `format-spec.toml` (24 tests)
- `functions.toml`, `lifetimes.toml`, `string-methods-all.toml`, `string-operations.toml`

**Fix:** ✅ Implemented `str.format()` method in `expr_gen.rs`. Added `convert_python_format_to_rust()` helper function that parses Python format specifiers (`{0}`, `{name}`, `{:.2f}`) and converts to Rust `format!()` macro syntax.

---

## Priority 5: Unsupported Type Annotations (~26 tests)

### 5.1 String Literal Type Annotations (5 tests)
**Error:** `Unsupported type annotation: string literal`

Affected: `dataclasses-field.toml`, `defaultdict.toml`, `examples-mathematical.toml`, `new-vs-init.toml`

**Fix:** Handle forward reference type annotations (quoted class names).

### 5.2 Ellipsis in Type Annotations (5 tests)
**Error:** `Unsupported type annotation: Ellipsis`

Affected: `ellipsis.toml`, `examples-type-hints.toml`, `modules-itertools-functools.toml`

**Fix:** Handle `...` in `Callable[..., T]` and `Tuple[int, ...]` type hints.

### 5.3 Integer Constant in Type Annotations (2 tests)
**Error:** `Unsupported type annotation: integer constant`

Affected: `ellipsis.toml`, `imports-basic.toml`, `examples-tooling.toml`

**Fix:** Handle literal types.

### 5.4 Module.Type Annotations (4 tests)
**Error:** `Unsupported type annotation: argparse.Namespace`, `uuid.UUID`

Affected: `cli-integration.toml`, `examples-marco-polo-cli.toml`, `stdlib-misc.toml`

**Fix:** Handle qualified type names from external modules.

### 5.5 Complex Type Annotations (3 tests)
**Error:** `Complex type annotations not yet supported`

Affected: `bytes.toml`, `class-methods.toml`, `dataclasses-field.toml`

**Fix:** Handle more complex generic type expressions.

---

## Priority 6: Unsupported Features (~31 tests)

### 6.1 Multiple Context Managers (13 tests)
**Error:** `Multiple context managers not yet supported`

Affected: `nested-context-managers.toml`

**Fix:** Implement `with a, b:` syntax transpilation.

### 6.2 Complex Comprehension Targets (6 tests)
**Error:** `Complex comprehension targets not yet supported`

Affected: `async-comprehensions.toml`, `comprehensions.toml`, `iterators.toml`, `lambdas.toml`, `modules-itertools-functools.toml`

**Fix:** Handle tuple unpacking in comprehensions.

### 6.3 Nested List Comprehensions (2 tests)
**Error:** `Nested list comprehensions not yet supported`

Affected: `async-comprehensions.toml`, `comprehensions.toml`

**Fix:** Implement nested comprehension flattening.

### 6.4 Unsupported Constant Type - Ellipsis (7 tests)
**Error:** `Unsupported constant type`

Affected: `ellipsis.toml`

**Fix:** Handle `...` (Ellipsis) as a constant value.

### 6.5 Multiple Assignment Targets (1 test)
**Error:** `Multiple assignment targets not supported`

Affected: `assignment.toml` :: `multi_assign_same_value`

**Fix:** Handle `a = b = c = value` syntax.

### 6.6 Invalid Operator Is (2 tests)
**Error:** `Invalid operator Is for comparison conversion`

Affected: `dunder-context.toml`, `nested-context-managers.toml`

**Fix:** Handle `is` operator for identity comparison.

---

## Priority 7: Module Function Implementations (~7 tests)

### 7.1 Regex Functions
- `re.fullmatch` not implemented
- `split()` with maxsplit not supported

### 7.2 Crypto Functions
- `base64.b32encode/b32decode` needs data-encoding crate
- `hashlib.sha3_256` not implemented
- `hmac.new()` argument handling

### 7.3 OS/System Functions
- `os.path.join()` argument handling
- `sys.modules` attribute

### 7.4 Itertools Functions
- `itertools.combinations` not implemented

---

## Priority 8: Other Issues (~8 tests)

### 8.1 Self Keyword Conflict (3 tests)
**Error:** `Python variable 'self' conflicts with Rust keyword`

Affected: `decorators-basic.toml`, `metaclasses.toml`

**Fix:** Auto-rename `self` parameter in certain decorator contexts.

### 8.2 Nested Function Calls (3 tests)
**Error:** `Unsupported function call type: nested function call`

Affected: `closures.toml`

**Fix:** Handle closures that return callable functions.

### 8.3 Object Call (1 test)
**Error:** `Unsupported function call type: object call`

Affected: `dunder-callable.toml`

**Fix:** Handle `__call__` dunder method invocations.

---

## Priority 9: Parse Errors (4 files)

Files that fail to parse:
- `async-for.toml`
- `complex-numbers.toml`
- `examples-stdlib.toml`
- `exception-groups.toml`

**Fix:** Review TOML file syntax and Python code validity.

---

## Recommended Fix Order

1. **Week 1:** Fix all panics (Priority 1) - Stability
2. **Week 2:** String `format()` method (Priority 4) - High impact
3. **Week 3:** FString expressions (Priority 3.1) - High impact
4. **Week 4:** Statement types (Priority 2) - Core features
5. **Week 5:** Type annotations (Priority 5) - Type safety
6. **Week 6:** Remaining features (Priority 6-8) - Completeness

---

## Test Commands

```bash
# Run all TOML tests
cargo run -- test --path tests/toml

# Run specific test file
cargo run -- test --path tests/toml/functools-partial.toml

# Run with verbose output
cargo run -- test --path tests/toml -v

# Filter by test name
cargo run -- test --path tests/toml -f partial_basic
```
