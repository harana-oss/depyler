# Unnecessary Clone Elimination via Forward Analysis

## Problem Statement

The current transpiler generates unnecessary `.clone()` calls when assigning struct field values to local variables. For example:

**Python Input:**
```python
def calculate_player_of_match_distributions(state: State) -> list[float]:
    players = state.all_players
    home_score = state.home_match_score
    away_score = state.away_match_score
    margin = home_score - away_score
    total_points = home_score + away_score
    return [_calculate_percentage_chance(player, margin, total_points) for player in players]
```

**Current Rust Output (inefficient):**
```rust
pub fn calculate_player_of_match_distributions(state: &State) -> Vec<f64> {
    let players = state.all_players.clone();  // ← Unnecessary clone!
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    let margin = home_score - away_score;
    let total_points = home_score + away_score;
    players
        .iter()
        .cloned()
        .map(|player| _calculate_percentage_chance(&player, margin, total_points))
        .collect::<Vec<_>>()
}
```

**Optimal Rust Output:**
```rust
pub fn calculate_player_of_match_distributions(state: &State) -> Vec<f64> {
    let players = &state.all_players;  // ← Borrow instead of clone
    let home_score = state.home_match_score;
    let away_score = state.away_match_score;
    let margin = home_score - away_score;
    let total_points = home_score + away_score;
    players
        .iter()
        .cloned()
        .map(|player| _calculate_percentage_chance(&player, margin, total_points))
        .collect::<Vec<_>>()
}
```

## Root Cause Analysis

### Current Behavior

In `expr_gen.rs`, the `convert_attribute` function (around line 12048) determines whether to add `.clone()`:

```rust
let needs_clone =
    !self.ctx.is_assignment_target && !self.ctx.returns_reference && self.field_needs_clone(value, attr);

if needs_clone {
    Ok(parse_quote! { #value_expr.#attr_ident.clone() })
} else {
    Ok(parse_quote! { #value_expr.#attr_ident })
}
```

The `field_needs_clone` function returns `true` for non-Copy types (`String`, `Vec<T>`, `HashMap<K,V>`, etc.). This is conservative but overly aggressive.

### Why Clone is Currently Added

1. **Borrowed Parameters**: Functions receive structs as `&State` (borrowed reference)
2. **Field Access**: `state.all_players` returns `&Vec<Player>` (a reference to the field)
3. **Ownership Transfer**: To create a local `let players = ...` binding with an owned value, we need to clone
4. **No Forward Analysis**: The transpiler doesn't know how `players` will be used later

### When Clone is Actually Necessary

Clone is required when the variable is:
- Returned from the function
- Passed to a function taking ownership (`fn foo(v: Vec<T>)`)
- Stored in a data structure
- Moved in a closure that outlives the current scope
- Used after a potential mutation of the source

### When Clone is Unnecessary

Clone can be avoided when the variable is only:
- Iterated over (`.iter()`, `for x in &var`)
- Accessed for length (`.len()`)
- Passed by reference (`&var`)
- Used in read-only operations

## Proposed Solution: Forward Usage Analysis

### Phase 1: Variable Usage Collection

Add a pre-pass that analyzes how each variable is used after its definition:

```rust
#[derive(Debug, Clone, Default)]
pub struct VariableUsage {
    /// Variable is used in iteration context (for loop, .iter(), etc.)
    pub iter_uses: u32,
    /// Variable is passed by reference
    pub ref_uses: u32,
    /// Variable is moved/consumed (return, owned param, store)
    pub move_uses: u32,
    /// Variable is mutated
    pub mut_uses: u32,
    /// Variable is used in a closure that may escape
    pub closure_capture: bool,
}

impl VariableUsage {
    /// Returns true if the variable only needs to be borrowed, not owned
    pub fn can_borrow(&self) -> bool {
        self.move_uses == 0 && !self.closure_capture && self.mut_uses == 0
    }
}
```

### Phase 2: Analysis Implementation

Create a new analysis pass in `crates/depyler-analysis/src/usage_analysis.rs`:

```rust
pub struct UsageAnalyzer<'a> {
    /// Map from variable name to its usage patterns
    usages: HashMap<String, VariableUsage>,
    /// Current function being analyzed
    function_params: &'a [HirParam],
    /// Known function signatures for interprocedural analysis
    function_sigs: &'a HashMap<String, FunctionSignature>,
}

impl<'a> UsageAnalyzer<'a> {
    pub fn analyze_function(func: &HirFunction) -> HashMap<String, VariableUsage> {
        let mut analyzer = Self::new(func);
        analyzer.analyze_stmts(&func.body);
        analyzer.usages
    }

    fn analyze_expr(&mut self, expr: &HirExpr, context: UsageContext) {
        match expr {
            HirExpr::Var(name) => {
                self.record_use(name, context);
            }
            HirExpr::MethodCall { object, method, args, .. } => {
                // Detect iteration patterns
                if matches!(method.as_str(), "iter" | "into_iter" | "iter_mut") {
                    self.analyze_expr(object, UsageContext::Iteration);
                } else {
                    self.analyze_expr(object, UsageContext::MethodReceiver);
                }
                for arg in args {
                    self.analyze_expr(arg, self.infer_arg_context(method, arg));
                }
            }
            HirExpr::Call { func, args, .. } => {
                for (i, arg) in args.iter().enumerate() {
                    let ctx = self.get_param_context(func, i);
                    self.analyze_expr(arg, ctx);
                }
            }
            // ... handle other expression types
        }
    }

    fn record_use(&mut self, name: &str, context: UsageContext) {
        let usage = self.usages.entry(name.to_string()).or_default();
        match context {
            UsageContext::Iteration => usage.iter_uses += 1,
            UsageContext::Reference => usage.ref_uses += 1,
            UsageContext::Move => usage.move_uses += 1,
            UsageContext::Mutation => usage.mut_uses += 1,
            UsageContext::ClosureCapture => usage.closure_capture = true,
            UsageContext::MethodReceiver => usage.ref_uses += 1, // Most methods take &self
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UsageContext {
    Iteration,      // Used in .iter() or for loop
    Reference,      // Passed as &T
    Move,           // Passed as T (owned) or returned
    Mutation,       // Used as &mut T
    ClosureCapture, // Captured by a closure
    MethodReceiver, // Used as method receiver (usually &self)
}
```

### Phase 3: Integration with Code Generation

Modify `CodeGenContext` to include usage information:

```rust
pub struct CodeGenContext<'a> {
    // ... existing fields ...
    
    /// Variable usage analysis results for the current function
    pub variable_usages: HashMap<String, VariableUsage>,
}
```

Modify `codegen_assign_stmt` in `stmt_gen.rs`:

```rust
pub(crate) fn codegen_assign_stmt(
    target: &AssignTarget,
    value: &HirExpr,
    type_annotation: &Option<Type>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // ... existing code ...

    // Check if this is a field access assignment
    if let (AssignTarget::Symbol(var_name), HirExpr::Attribute { .. }) = (target, value) {
        // Look up how this variable will be used
        if let Some(usage) = ctx.variable_usages.get(var_name) {
            if usage.can_borrow() {
                // Generate a borrow instead of a clone
                ctx.set_generate_borrow(true);
                let value_expr = value.to_rust_expr(ctx)?;
                ctx.set_generate_borrow(false);
                return codegen_assign_symbol(
                    var_name,
                    parse_quote! { &#value_expr },
                    type_annotation_tokens,
                    is_final,
                    ctx,
                );
            }
        }
    }

    // ... rest of existing code ...
}
```

### Phase 4: Handling Edge Cases

#### 4.1 Conditional Usage

```python
players = state.all_players
if condition:
    return players  # Move use
else:
    for p in players:  # Iter use
        print(p)
```

In this case, we have both move and iter uses. The conservative approach is to clone.

#### 4.2 Loop Re-assignment

```python
for _ in range(10):
    players = state.all_players
    process(players)  # If this takes ownership, need clone each iteration
```

#### 4.3 Closure Captures

```python
players = state.all_players
callback = lambda: players  # Captures players - may outlive scope
```

### Implementation Plan

#### Step 1: Add Usage Analysis Module (2-3 days)
- Create `crates/depyler-analysis/src/usage_analysis.rs`
- Implement `UsageAnalyzer` struct
- Add `VariableUsage` tracking
- Handle basic expression patterns

#### Step 2: Integrate with HIR Pipeline (1 day)
- Run usage analysis after HIR construction
- Store results in a function-level context
- Pass to code generation phase

#### Step 3: Modify Code Generation (2 days)
- Update `CodeGenContext` with usage info
- Modify `codegen_assign_stmt` to check usage
- Update `convert_attribute` to support borrow mode
- Add type tracking for borrowed variables

#### Step 4: Handle Complex Cases (2-3 days)
- Control flow analysis for conditional usage
- Closure capture detection
- Loop re-assignment handling
- Interprocedural analysis for function calls

#### Step 5: Testing & Validation (2 days)
- Add unit tests for usage analysis
- Add integration tests for generated code
- Benchmark memory/performance improvements
- Ensure no regressions in existing tests

### Alternative Approaches

#### Approach A: Lazy Clone (Simpler)

Instead of forward analysis, always generate borrows and let Rust's borrow checker fail. Then use error recovery to insert clones where needed.

**Pros**: Simpler implementation
**Cons**: Requires multiple compilation attempts, slower

#### Approach B: Conservative Heuristics (Simplest)

Add simple heuristics without full analysis:
- If variable name suggests iteration (`items`, `elements`, `list`, etc.)
- If variable is only used once
- If function is small (< 10 statements)

**Pros**: Very simple to implement
**Cons**: May miss optimization opportunities, fragile

#### Approach C: Annotation-Based (User Control)

Add Python annotations to control ownership:

```python
from depyler import borrow

@borrow
players = state.all_players  # Generates &state.all_players
```

**Pros**: User has explicit control
**Cons**: Requires code changes, not automatic

### Recommended Approach

Start with **Phase 1-3** of the forward analysis approach, handling the common case of simple iteration patterns. This covers the majority of real-world use cases while keeping complexity manageable.

Defer **Phase 4** (complex cases) until concrete examples arise that require it.

### Success Metrics

1. **Correctness**: All existing tests pass
2. **Performance**: 20-30% reduction in unnecessary clones for typical code
3. **Compilation**: Generated Rust code compiles without manual intervention
4. **Maintainability**: Analysis is isolated and testable

### Files to Modify

1. `crates/depyler-analysis/src/lib.rs` - Add usage analysis module
2. `crates/depyler-analysis/src/usage_analysis.rs` - New file
3. `crates/depyler-core/src/rust_gen/context.rs` - Add usage tracking
4. `crates/depyler-core/src/rust_gen/stmt_gen.rs` - Borrow generation
5. `crates/depyler-core/src/rust_gen/expr_gen.rs` - Conditional clone
6. `crates/depyler/src/lib.rs` - Wire up analysis phase

### References

- Rust RFC 2094 (NLL - Non-Lexical Lifetimes)
- "Ownership and Borrowing" in The Rust Programming Language
- MIR-based borrow checking in rustc
