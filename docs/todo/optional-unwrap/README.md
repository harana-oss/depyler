# Optional Type Auto-Unwrap Work Items

**Branch**: `math-fixes`  
**Created**: December 5, 2025  
**Last Updated**: December 6, 2025

This directory contains individual work items for implementing Python-like behavior for `Optional` types in the Depyler transpiler.

## Overview

In Python, Optional fields can be accessed directly without explicit unwrapping - operations on `None` values fail at runtime. The Rust transpilation should mimic this by adding `.unwrap()` automatically in appropriate contexts.

## Implementation Status Summary

| Category | Completed | Total | Status |
|----------|-----------|-------|--------|
| High Priority | 2 | 2 | ✅ 100% |
| Medium Priority | 4 | 5 | ⚠️ 80% |
| Low Priority | 4 | 5 | ⚠️ 80% |
| **Total** | **10** | **12** | **83%** |

## Completed Work

- ✅ **Binary Operations**: Optional fields are now auto-unwrapped in binary operations (`p.age * 2`)
- ✅ **Return Statements**: Correctly handles unwrapping when returning Optional from non-Optional functions
- ✅ **Function Arguments**: Unwraps Optional when passed to non-Optional parameters
- ✅ **Index Operations**: Unwraps Optional collections before indexing
- ✅ **Method Calls**: Unwraps Optional receivers for string methods
- ✅ **Chained Access**: Unwraps intermediate Optional fields in chains
- ✅ **Unary Operations**: Unwraps Optional operands for `-`, `not`, `~`
- ✅ **Augmented Assignment**: Handles `+=`, `-=` etc on Optional fields/variables
- ✅ **Assignment Type Check**: Auto-unwraps Optional when assigning to non-Optional variable
- ✅ **Conditional Context**: Full Python truthiness semantics for Optional types
- ✅ **Default Values (or pattern)**: `unwrap_or_else` for `optional or default`
- ✅ **String Formatting**: F-strings display Optional values correctly

## Work Items

### High Priority

| # | Title | Complexity | Status |
|---|-------|------------|--------|
| [001](./001-function-arguments.md) | Function Arguments | Medium-High | ✅ Completed |
| [002](./002-index-operations.md) | Index Operations | Medium | ✅ Completed |

### Medium Priority

| # | Title | Complexity | Status |
|---|-------|------------|--------|
| [003](./003-method-calls.md) | Method Calls | Medium | ✅ Completed |
| [004](./004-chained-access.md) | Chained Optional Access | High | ✅ Completed |
| [009](./009-default-values.md) | Default Values (or pattern) | Medium | ✅ Completed |
| [010](./010-string-formatting.md) | String Formatting (F-Strings) | Medium | ✅ Completed |
| [011](./011-method-chaining.md) | Method Chaining on Optional Results | High | ⚠️ Partial |

### Low Priority

| # | Title | Complexity | Status |
|---|-------|------------|--------|
| [005](./005-unary-operations.md) | Unary Operations | Low | ✅ Completed |
| [006](./006-augmented-assignment.md) | Augmented Assignment | Medium | ✅ Completed |
| [007](./007-assignment-type-check.md) | Assignment Type Check | Medium | ✅ Completed |
| [008](./008-conditional-context.md) | Conditional Context | Medium | ✅ Completed |
| [012](./012-collection-operations.md) | Collection Operations | Medium-High | Not Started |

## Recommended Implementation Order

```
Phase 1: Foundation
├── 001: Function Arguments (enables most common use case)
├── 002: Index Operations (Optional collections)
└── 003: Method Calls (complete partial implementation)

Phase 2: Complex Patterns
├── 004: Chained Optional Access
├── 011: Method Chaining
└── 009: Default Values

Phase 3: Edge Cases
├── 005: Unary Operations
├── 006: Augmented Assignment
├── 007: Assignment Type Check
├── 008: Conditional Context
├── 010: String Formatting
└── 012: Collection Operations
```

## Key Files to Modify

| File | Purpose |
|------|---------|
| `crates/depyler-core/src/rust_gen/expr_gen.rs` | Expression generation |
| `crates/depyler-core/src/rust_gen/stmt_gen.rs` | Statement generation |
| `crates/depyler-core/src/rust_gen/context.rs` | Code generation context |
| `crates/depyler-core/src/rust_gen/func_gen.rs` | Function generation |
| `crates/depyler-core/src/hir.rs` | HIR types including `Type::Optional` |

## Existing Helper Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `field_is_optional()` | `expr_gen.rs:11469` | Check if attribute access is Optional |
| `expr_is_optional()` | `expr_gen.rs:11807` | Check if any expression is Optional |
| `expr_is_optional()` | `stmt_gen.rs:222` | Duplicate for statement context |
| `convert_with_optional_unwrap()` | `expr_gen.rs:11504` | Auto-unwrap for binary ops |

## Testing Strategy

Each work item should include:
1. Unit tests for the specific transformation
2. Integration tests verifying Rust compilation
3. Runtime tests confirming correct behavior

Test file: `tests/test_optional_unwrap.rs` (to be created)

## Original Document

The original comprehensive document is preserved at:
`/Users/naden/Developer/depyler/docs/todo/optional-unwrap-remaining-work.md`
