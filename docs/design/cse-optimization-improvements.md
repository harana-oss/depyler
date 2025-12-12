# Common Subexpression Elimination (CSE) Optimization Improvements

## Problem Statement

The current CSE optimization in Depyler creates unnecessary temporary variables for simple expressions that are only used once. This results in verbose, less readable Rust code.

### Current Behavior (Before Fix)

Given Python code like:
```python
if state.period.name != "ExtraTime":
    do_something()
```

The transpiler generates:
```rust
let _cse_temp_1 = state.period.name.clone() != "ExtraTime";
if _cse_temp_1 {
    do_something();
}
```

### Desired Behavior (After Fix)

The same Python code should generate cleaner Rust:
```rust
if state.period.name.clone() != "ExtraTime" {
    do_something();
}
```

## Root Cause Analysis

The issue is in `crates/depyler-core/src/optimizer.rs` in the `is_complex_expr` function. The original implementation considered ANY binary operation (except Add/Sub with simple operands) as "complex" enough to warrant CSE:

```rust
fn is_complex_expr(&self, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary { op, left, right } => {
            // Consider non-trivial operations or non-literal operands
            !matches!(op, BinOp::Add | BinOp::Sub)
                || !matches!(left.as_ref(), HirExpr::Var(_) | HirExpr::Literal(_))
                || !matches!(right.as_ref(), HirExpr::Var(_) | HirExpr::Literal(_))
        }
        HirExpr::Call { .. } => true,
        _ => false,
    }
}
```

This logic incorrectly treats simple comparisons (`==`, `!=`, `<`, `>`, etc.) as complex expressions, creating temp variables even for one-time uses.

## Recommended Solution

### Approach 1: Conservative Complexity Check (Implemented)

Only consider expressions "complex" when they:
1. Have **nested binary operations** (e.g., `(a + b) * c`)
2. Are arithmetic operations with **expensive operands** (function calls, method calls)
3. Are pure function calls (which may be reused)

```rust
fn is_complex_expr(&self, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary { op, left, right } => {
            // Only consider expressions with nested binary operations as complex enough for CSE.
            // Simple comparisons like `x != "string"` or `a > b` should not create temp variables.
            let has_nested_binary =
                matches!(left.as_ref(), HirExpr::Binary { .. }) 
                || matches!(right.as_ref(), HirExpr::Binary { .. });

            // Arithmetic operations with nested operands are worth CSE'ing
            let is_arithmetic = matches!(
                op, 
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            );

            has_nested_binary || (is_arithmetic && self.has_expensive_operand(left, right))
        }
        HirExpr::Call { .. } => true,
        _ => false,
    }
}

fn has_expensive_operand(&self, left: &HirExpr, right: &HirExpr) -> bool {
    let is_expensive = |e: &HirExpr| {
        matches!(e, HirExpr::Call { .. } | HirExpr::MethodCall { .. })
    };
    is_expensive(left) || is_expensive(right)
}
```

### Approach 2: Two-Pass CSE (Future Enhancement)

A more sophisticated approach would use two passes:

1. **First pass**: Count expression occurrences across the function
2. **Second pass**: Only create temp variables for expressions that occur **more than once**

This would eliminate single-use temps while still optimizing repeated expensive computations.

```rust
fn eliminate_common_subexpressions_program(&self, mut program: HirProgram) -> HirProgram {
    for func in &mut program.functions {
        // Pass 1: Count expression occurrences
        let mut expr_counts: HashMap<u64, usize> = HashMap::new();
        self.count_expr_occurrences(&func.body, &mut expr_counts);
        
        // Pass 2: Only CSE expressions that occur more than once
        let mut cse_map: HashMap<u64, (HirExpr, String)> = HashMap::new();
        let mut temp_counter = 0;
        func.body = self.eliminate_cse_in_body(
            &func.body, 
            &mut cse_map, 
            &mut temp_counter,
            &expr_counts  // New parameter
        );
    }
    program
}
```

### Approach 3: Use-Def Chain Analysis (Advanced)

For maximum optimization, implement proper use-def chain analysis to:
- Track where expressions are defined and used
- Identify truly redundant computations
- Consider control flow (expressions in loops are more valuable to CSE)

## Implementation Checklist

- [x] Update `is_complex_expr` to be more conservative
- [x] Add `has_expensive_operand` helper function
- [x] Add unit tests for CSE optimization
- [x] Add integration tests (TOML format)
- [ ] Consider implementing two-pass CSE for future optimization
- [ ] Benchmark impact on generated code size and performance

## Test Cases

See `tests/toml/cse-optimization.toml` for comprehensive test cases covering:
- Simple comparisons (should NOT create temps)
- Nested binary operations (should create temps)
- Repeated expressions (should create temps)
- Function calls in expressions
- Method calls in expressions

## Impact Assessment

### Positive Impacts
- Cleaner, more readable generated Rust code
- Fewer unnecessary variable allocations
- Reduced code size in output

### Potential Risks
- Some truly redundant computations may not be eliminated
- Need thorough testing to ensure no performance regressions

## Related Files

- `crates/depyler-core/src/optimizer.rs` - Main optimizer implementation
- `crates/depyler-core/src/hir.rs` - HIR expression definitions
- `tests/toml/cse-optimization.toml` - Test cases
