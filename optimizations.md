# Depyler: Suggested Optimization Passes

## Summary
- Biggest gaps: ownership/borrow inference, allocation reduction, and Python idiom lowering. These attack the dominant Py->Rust perf cliffs (forced clones, heap churn, intermediate `collect()`s).
- Several classic ILP/loop opts (LICM, unrolling, devirt) are likely partial wins; check overlap with `loop_optimization` and `iterator_fusion` before adding.
- Idiom-lowering passes should run early so existing perf passes see the right HIR shapes.

## Suggested passes

### Ownership and borrowing

| Pass | What it does | Notes |
|---|---|---|
| borrow_inference | Param `T` -> `&T` / `&mut T` when callee never owns or moves it | Highest single Py->Rust win; pairs with clone_elimination |
| last_use_move | Convert `.clone()` to move at the last use of a binding | Complements clone_elimination at use sites, not just defs |
| closure_capture_minimization | Capture only fields used in body, not whole self | Smaller closures, looser borrow constraints |

### Allocation reduction

| Pass | What it does | Notes |
|---|---|---|
| capacity_preallocation | `Vec::with_capacity(n)` / `String::with_capacity` / `HashMap::with_capacity` when n is bounded | Needs range info from constant_propagation |
| collect_elimination | Drop intermediate `.collect()` when next op is iterator-consuming | Run after iterator_fusion |
| string_concat_lowering | `+` chains and runtime f-string evaluation -> `format!` / `write!` | Single-allocation construction |

### Python idiom lowering (run early)

| Pass | What it does | Notes |
|---|---|---|
| comprehension_lowering | `[f(x) for x in xs if g(x)]` -> `xs.iter().filter(g).map(f).collect()` | Feeds iterator_fusion |
| generator_to_iterator | `yield`-functions -> hand-rolled `Iterator` impl with explicit state | No coroutine runtime needed |
| try_except_to_result | `try` / `except` -> `Result<T, E>` with `?` propagation | Removes hidden allocations and dynamic dispatch on exceptions |
| isinstance_to_match | `isinstance` chains -> exhaustive `match` on enum | Compiler verifies totality |
| dict_get_default_lowering | `d.get(k, default)` -> `.get(&k).copied().unwrap_or(default)` | Avoids boxed default allocations |
| question_mark_promotion | Explicit `match Err` patterns -> `?` | Smaller HIR for downstream passes |

### Type specialization

| Pass | What it does | Notes |
|---|---|---|
| integer_narrowing | `i64` -> `i32` / `u32` / `u16` / `u8` via interval analysis | Helps SIMD and struct packing |
| float_specialization | `f64` -> `f32` where precision analysis permits | Gate behind explicit flag |
| tuple_struct_promotion | `dataclass` / `namedtuple` -> plain `struct` with derived traits | Removes any per-field boxing |

### Control flow and dispatch

| Pass | What it does | Notes |
|---|---|---|
| licm | Hoist loop-invariant expressions | Confirm not already in `loop_optimization` |
| loop_unrolling | Unroll loops with small constant trip counts | Enables further folding and autovec |
| match_arm_reordering | Reorder arms by static frequency or guard cost | Branch predictor wins |
| or_pattern_consolidation | Merge arms with identical RHS into or-patterns | Smaller codegen |
| pattern_decision_tree | Optimize compiled match decision order | Matters for deeply nested matches |
| bounds_check_hint_insertion | `assert!` before tight loops to elide per-iteration checks | Common LLVM-friendly idiom |

### Purity and memoization

| Pass | What it does | Notes |
|---|---|---|
| purity_analysis | Mark side-effect-free fns; promote to `const fn` where eligible | Feeds const-eval and `#[must_use]` insertion |
| memoization_insertion | Cache results of pure fns called repeatedly with same args | Opt-in only; default risks unbounded growth |
| must_use_insertion | Annotate fns whose return is meaningful | Catches misuse, no perf impact |

### Async-specific

| Pass | What it does | Notes |
|---|---|---|
| async_elimination | Strip `async` from fns whose body contains no `.await` | Removes state-machine codegen overhead |
| join_combinator_lowering | Sequential `.await` over independent futures -> `join!` / `try_join!` | Concurrency without code change |

## Sequencing

1. Idiom lowering: comprehension_lowering, try_except_to_result, isinstance_to_match, generator_to_iterator, string_concat_lowering, dict_get_default_lowering.
2. Type / range: integer_narrowing, purity_analysis, constant_propagation (existing), constant_folding (existing).
3. Ownership: borrow_inference, last_use_move, closure_capture_minimization, then clone_elimination (existing).
4. Allocation shape: capacity_preallocation, collect_elimination.
5. Iterator: comprehension already lowered, then iterator_fusion (existing), then collect_elimination.
6. Loop and CFG: licm, loop_optimization (existing), loop_unrolling, branch_to_match (existing), match_arm_reordering, or_pattern_consolidation.
7. Late: must_use_insertion, async_elimination.

## Caveats

- Names above may overlap with what `loop_optimization`, `iterator_fusion`, and `string_optimization` already cover. Diff before implementing.
- `memoization_insertion`, `float_specialization` change semantics; require explicit opt-in.
- `borrow_inference` interacts with public API stability; safest scoped to crate-internal fns first.
- No quantitative speedup estimates given without sight of the benchmark suite or representative HIR. Confidence on the categorical ranking (ownership > allocation > idiom > classic) is high based on typical Py->Rust transpiler bottlenecks; per-pass magnitudes will vary by workload.