
# Depyler – Simplification, Optimisation & Readability Backlog

---

## 1  Wire up (or remove) the dataflow type inferencer  *(original)*

`DepylerPipeline::transpile` **does** now call `DataflowTypeInferencer` after
`ConstGenericInferencer`, so `parse_to_typed_hir` is no longer the only caller.
However, **two independent type-inference systems still coexist**:

| System | Where used | Mechanism |
|---|---|---|
| `types/type_hints.rs` – `TypeHintProvider` | `ast_bridge/converters.rs` (twice, lines ≈2335 & 2376) | Heuristic pattern-matching on HIR expressions |
| `dataflow/` – `DataflowTypeInferencer` | `transpile`, `transpile_with_dependencies`, `parse_to_typed_hir` | CFG + lattice + fixpoint solver |

**Actions:**
- The `TypeHintProvider` runs *during* AST→HIR conversion (inside `converters.rs`)
  to fill in missing parameter types.  The dataflow inferencer then runs *again*
  as a post-pass on the finished HIR.  The overlap should be audited: any
  information the heuristic adds that the dataflow pass would also add is wasted
  work.
- Consider moving parameter-type annotation into the dataflow pass so `TypeHintProvider`
  can be deleted.  It is 1 070 lines; removing it would shrink the codebase
  materially.
- At minimum, extract the two `TypeHintProvider` call-sites in `converters.rs`
  into a shared helper – they are near-identical copies (lines ≈2335–2410).



## 5  Dead / `#[allow(dead_code)]` fields that should be removed

Several struct fields are marked dead but kept:

| Location | Field | Note |
|---|---|---|
| `types/type_inference.rs` `TypeVarRegistry` | `active_bindings: HashMap<String, Type>` | "reserved for future use" – just remove it |
| `types/type_hints.rs` `TypeConstraint::ArgumentConstraint` | `_var`, `_func`, `_param_idx`, `_expected` | The whole variant appears unused; remove or implement |

Beyond those two headline items, `#[allow(dead_code)]` is pervasive:
`types/type_inference.rs` alone suppresses the warning at **14 separate sites** (lines 28, 341, 347, 349, 686, 701, 738, 747, 791, 845, 872, 885, 896, 917).
`types/type_hints.rs` has two more (lines 60, 88).
`analysis/borrowing_context.rs` has one at line 168.
See §17 for the clusters in `direct_rules.rs` and `lifetime_analysis.rs`.

The aggregate picture: at least **30 `#[allow(dead_code)]` attributes** exist across the codebase. A single `cargo check 2>&1 | grep "dead_code"` pass after removing the allow attributes would surface which items can simply be deleted.

These add noise and confuse future readers.

## 8  `offset_to_line_col` in `hir.rs` is O(n) and re-scans on every call

`Span::from_text_range` calls `offset_to_line_col` which iterates over the
entire source string char-by-char from the beginning for every span created.
During AST→HIR conversion this is called once per HIR node.

**Fix:** Pre-build a `Vec<u32>` of newline byte-offsets once (a "line table") in
`AstBridge::with_source` and pass it through to `Span::from_text_range` for O(log n)
binary-search lookups.

---

## 9  `Symbol = String` – consider an interned ID

`pub type Symbol = String` means every HIR node that stores a symbol name
(function name, variable name, parameter name) heap-allocates independently.
Because the same name can appear many times (every use of a variable), this is
both memory-wasteful and slow to clone.

**Fix:** Replace `Symbol` with an interned string type using a popular string interner library.

---

## 10  `Spanned<T>` wraps an `Option<Span>` – prefer `Option<Spanned<T>>`

`Spanned<T> { node: T, span: Option<Span> }` means every HIR node always
carries an optional span even when spans are disabled, and the common case is
`span: None`.  The ergonomic consequence is that callers must repeatedly
`.node`-unwrap.

**Fix:** Consider inverting to `type MaybeSpanned<T> = (T, Option<Span>)` or,
better, distinguish spanned and unspanned HIR at the type level so the `Option`
is hoisted to the module boundary, not every leaf node.


## 14  `direct_rules.rs` is the largest single file (4 902 lines)

`rust_generator/direct_rules.rs` dwarfs every other file in the codebase.  It
contains at least five distinct concerns that already have natural homes in the
existing submodule hierarchy:

| Concern | Suggested destination |
|---|---|
| `convert_class_to_struct` + helpers | `rust_generator/class_gen.rs` |
| `convert_class_to_enum` / `convert_class_to_intflag` | `rust_generator/enum_gen.rs` (already `union_enum_gen.rs` exists) |
| `convert_abc_to_trait` | `rust_generator/trait_gen.rs` |
| `rust_type_to_syn_type` + family | `rust_generator/type_conv.rs` |
| Statement conversion helpers (`convert_*_assignment`, etc.) | `rust_generator/statements/` |

Splitting would bring each file under ~500 lines and make individual concerns
independently testable.

---

## 15  `Optimizer` and `OptimizerConfig` reconstructed on every `transpile` call

Both `transpile` and `transpile_with_dependencies` call
`optimizations::optimizer::Optimizer::new(OptimizerConfig::default())` inline,
constructing a fresh optimizer on every invocation.  The optimizer carries no
mutable state between calls, so this is wasteful (repeated heap allocation,
re-parsing of config) and prevents sharing a pre-configured optimizer.

**Fix:** Store a pre-built `Optimizer` (or at minimum an `OptimizerConfig`) in
`DepylerPipeline` so users can configure optimization behaviour, and the
construction cost is paid once at pipeline creation time.

---

## 16  `is_potentially_mutating_method` hard-codes a fixed list of method names

`direct_rules.rs::is_potentially_mutating_method` decides whether to emit
`&mut self` or `&self` by checking against a hard-coded list of ~20 method
names (`push`, `pop`, `append`, …).  Any method not in the list that actually
mutates state will silently generate `&self`, causing a Rust compile error in
the output.

**Fix:** Prefer using the HIR-level `method_mutates_self` body scan (which
already exists) as the sole mechanism for determining mutability, and delete the
name-based heuristic.  The body scan correctly detects `self.field = …`
assignments regardless of method name.

---

## 17  Multiple `#[allow(dead_code)]` clusters in `direct_rules.rs` and `lifetime_analysis.rs`

Six `#[allow(dead_code)]` attributes appear in `direct_rules.rs` (lines 1900,
2088, 3158, 3241, 3280, 3288) and five more in
`analysis/lifetime_analysis.rs` (lines 199, 323, 335, 770, 1024).  Suppressing
warnings rather than removing code makes it impossible to tell what is truly
in use.

**Fix:** For each suppressed item, either:
- Delete it if it is genuinely unused, or
- Move it behind a `#[cfg(test)]` attribute if it is test-only, or
- Integrate it into the live code path and remove the allow attribute.

---

## 18  `parse_to_typed_hir` and `parse_to_hir` have an avoidable double-parse path

`parse_to_typed_hir` calls `parse_to_hir` which calls `parse_python` internally,
then runs the const-inferencer and dataflow inferencer on the result.
`transpile` / `transpile_with_dependencies` replicate this exact sequence
inline rather than calling `parse_to_typed_hir`.  This creates three entry
points with overlapping multi-step bodies.

**Fix:** Once §2 (`build_rust_output`) is implemented, also have it call
`parse_to_typed_hir` internally so the inference steps are defined in exactly
one place:

```rust
fn build_rust_output(&self, src: &str)
    -> Result<(String, Vec<cargo_toml_gen::Dependency>)>
{
    let hir = self.parse_to_typed_hir(src)?;
    let optimized = /* optimizer */ .optimize_program(hir);
    rust_generator::generate_rust_file(&optimized, &self.transpiler.type_mapper)
}
```

---

## 19  `infer_expr_type` duplicated between `direct_rules.rs` and the dataflow layer

`direct_rules.rs` contains a ~60-line `infer_expr_type` function that mirrors
basic type inference from literals and binary expressions.  The same logic
(and more) is already in `dataflow/solver.rs` and `types/type_inference.rs`.

**Fix:** Remove `infer_expr_type` (and its callers `infer_method_return_type` /
`collect_method_return_types`) from `direct_rules.rs` and instead call into the
existing dataflow or heuristic inferencer.

---

## 20  `AstBridge` built fresh on every `transpile` call

`ast_bridge::AstBridge::new().with_source(…).python_to_hir(ast)` is called
inside both `transpile` and `transpile_with_dependencies`.  `AstBridge` itself
holds no reusable state (no caches, no interner), so constructing it fresh is
harmless but noisy.

**Fix:** Either store a reusable `AstBridge` template in `DepylerPipeline`
(once `AstBridge` gains an interner – see §9), or at minimum extract
`AstBridge::new().with_source(src).python_to_hir(ast)` into a private helper
`fn parse_to_hir_inner` that is called from `parse_to_hir` (removing the
duplicate `with_source` call that currently lives in both `transpile` variants).

---

## 21  `expr_gen.rs` is 16 867 lines – the largest file in the codebase

`rust_generator/expr_gen.rs` is more than twice the size of the next-largest
file (`stmt_gen_old.rs`, which is dead code).  It contains a single private
`ExpressionConverter` struct and one enormous `impl ToRustExpr for HirExpr`
(starting at line 16 643).  There are essentially **no top-level public
functions** – all 16k+ lines are one giant match arm per expression kind,
mixed with dozens of helper methods on `ExpressionConverter`.

This makes the file:
- Impossible to navigate without IDE folding
- Impossible to review in code review
- A merge-conflict magnet
- Untestable at the unit level (helpers are buried inside a private impl)

**Fix:** Split by expression category.  Suggested submodule layout under
`rust_generator/expressions/`:

| New file | Contents |
|---|---|
| `mod.rs` | Re-exports + the `ToRustExpr` impl that dispatches to each sub-module |
| `literals.rs` | `Literal`, `FString`, byte-string handling |
| `calls.rs` | Function calls, method calls, `super()` |
| `collections.rs` | List/dict/set/tuple construction and comprehensions |
| `arithmetic.rs` | `BinOp`, `UnaryOp`, augmented arithmetic |
| `comparisons.rs` | `Compare`, `BoolOp`, chained comparisons |
| `closures.rs` | Lambda, generator expressions |
| `async_exprs.rs` | `Await`, async comprehensions |
| `subscripts.rs` | Subscript, slice, attribute access |

Each file should be ≤ 500 lines, making individual expression kinds independently
testable and reviewable.

---

## 22  `string_optimization.rs` is an isolated module with no test coverage path

`optimizations/string_optimization.rs` (358 lines) implements `StringOptimizer`
and `StringContext` for choosing between `&'static str`, `&str`, `String`, and
`Cow<str>`.  It is consumed only by `expr_gen.rs`.  The optimizations module
`mod.rs` does **not** re-export it, so it is only reachable via the internal
`crate::optimizations::string_optimization` path.

**Fix:**
- Expose `StringOptimizer` through `optimizations::mod.rs` so its behaviour can
  be tested independently of `expr_gen`.
- Add unit tests for each `StringContext` variant – they are pure functions and
  are trivially testable.
- Evaluate whether `StringOptimizer` belongs in `optimizations/` or alongside
  expression codegen in `rust_generator/expressions/literals.rs` (see §21).

---

## 23  `transpile_with_dependencies` doc-comment is malformed

The doc-comment for `transpile_with_dependencies` in `lib.rs` is missing its
summary sentence – it starts with "of Cargo dependencies needed to build it."
(line ≈ 212), which is a dangling continuation of a sentence that was never
started.  The `transpile` method's closing `///` block runs directly into the
`transpile_with_dependencies` doc block, so the rendered docs are misleading.

**Fix:** Give `transpile_with_dependencies` its own complete `///` doc-comment
block, separate from `transpile`'s examples.

---

## 25  `ExprConverter` in `converters.rs` uses `.as_ref().clone()` anti-pattern

Several sites in `converters.rs` use `e.as_ref().clone()` (e.g. lines 936,
942, 948) where `e` is already a `Box<T>` – dereferencing and re-boxing is
unnecessary.  The idiomatic equivalent is `(*e).clone()` or, better, taking
ownership directly when the box is not needed afterwards.

**Fix:** Replace `e.as_ref().clone()` with `*e.clone()` (or consume the box
directly) at each occurrence.  This is a pure cosmetic/performance cleanup that
has no semantic impact but makes the code clearer.

---

## 26  `interprocedural/` is not called from `transpile` or `parse_to_typed_hir`

`interprocedural/` provides a call-graph, mutation propagation analysis, and a
signature registry – but neither `DepylerPipeline::transpile` nor any public
pipeline method invokes it.  It is implemented but silently skipped.

**Fix:** Either:
- Wire interprocedural analysis into `parse_to_typed_hir` (after dataflow
  inference, before optimization) so it contributes to mutability decisions
  currently made by the heuristic in `direct_rules.rs` (see §16), or
- Add a clear `// Not yet wired into the pipeline` comment at the module root
  so it is not mistaken for active code, and add a tracking issue.

---


---

## 28  418 `unwrap()` calls in production code – replace with `?` or `ok_or`

`cargo grep -n "unwrap()" crates/depyler-core/src/` reports **418 `unwrap()`
calls** outside of test code.  Many of these appear in `expr_gen.rs` and
`interprocedural/call_graph.rs` and will panic at runtime if their assumption is
violated.

Key clusters:
- `interprocedural/call_graph.rs` lines 165–181: Tarjan SCC algorithm accesses
  HashMap entries with `.get(key).unwrap()` – these should use `.get(key)
  .expect("key inserted during DFS")` at minimum, or restructure with
  entry-API so the unwrap is impossible.
- `expr_gen.rs` (> 30 sites): `parse::<i32>().unwrap()` parsing compile-time
  integer constants extracted from Python AST literals – these should propagate
  a `TranspileError::InvalidLiteral` instead of panicking.
- `types/type_mapper.rs` line 200: `.unwrap()` on a map lookup that the
  surrounding comment implies "should always exist" – convert to `ok_or_else`
  and surface as a `TranspileError`.

**Fix:** Establish a policy: `unwrap()` is forbidden outside `#[cfg(test)]` and
`main()`.  Enforce it via a Clippy `#![deny(clippy::unwrap_used)]` attribute in
`lib.rs`.  Fix each violation either with `?` (for `Option`/`Result` in a
`Result`-returning function), `.expect("invariant: …")` (where the invariant is
provable and worth documenting), or proper `ok_or`/`ok_or_else` propagation.

---

## 29  `is_expr_float_type` / `is_expr_int_type` / `is_expr_string_type` / `is_expr_bool_type` re-implement `get_expr_type`

`CodeGenContext` has four parallel methods – `is_expr_float_type`,
`is_expr_int_type`, `is_expr_string_type`, `is_expr_bool_type` – each of which
independently pattern-matches over `HirExpr` variants to determine a scalar
type.  The same pattern matching already exists in `get_expr_type`.  The four
`is_expr_*` methods are called a combined **39 times** across the codebase.

**Fix:** Remove the four `is_expr_*` methods and replace their call sites with
`ctx.get_expr_type(expr) == Some(Type::Float)` (etc.) or, for readability, a
single helper:

```rust
fn expr_has_type(ctx: &CodeGenContext, expr: &HirExpr, ty: &Type) -> bool {
    ctx.get_expr_type(expr).as_ref() == Some(ty)
}
```

This halves the type-querying surface area on `CodeGenContext` and centralises
type resolution logic in one place.

