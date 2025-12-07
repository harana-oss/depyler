# Test Migration Plan: TOML-Based Test Definitions

## Overview

This document outlines the migration of depyler's test suite from inline Rust tests with pattern assertions to a declarative TOML-based format. This migration will improve:

- **Readability**: Test cases are easier to understand and review
- **Maintainability**: Adding new tests requires no Rust knowledge
- **Consistency**: Standardized test format across the entire suite
- **Coverage visibility**: Clear view of what Python constructs are tested

## Current State

- **Total test files**: 234
- **Files using pattern-based transpilation tests**: 191
- **Test pattern**: Python snippet → transpile → assert Rust output contains patterns

### Current Test Pattern

```rust
#[test]
fn test_string_lstrip() {
    let python_code = r#"
def strip_leading(s: str) -> str:
    return s.lstrip()
"#;

    let rust_code = transpile_and_check(python_code, &[]);
    assert!(rust_code.contains("trim_start()"), "Should contain trim_start()");
    assert!(!rust_code.contains("lstrip()"), "Should not contain lstrip()");
}
```

## Target State

### TOML Test Definition Format

```toml
[[test]]
name = "string_lstrip"
description = "Python lstrip() should transpile to Rust trim_start()"

[test.python]
code =  '''
def strip_leading(s: str) -> str:
    return s.lstrip()
'''

[test.assertions]
contains = ["trim_start()"]
not_contains = ["lstrip()"]
```

### Extended TOML Format (for complex tests)

```toml
[[test]]
name = "dict_nested_assignment"
description = "Nested dict assignment should use get_mut"

[test.python]
code =  '''
def test_nested():
    d = {}
    d["outer"] = {}
    d["outer"]["inner"] = "value"
    return d
'''

[test.assertions]
contains = ["get_mut", "unwrap()"]
count = { "get_mut" = { min = 2 } }  # At least 2 occurrences

[test.options]
compile_check = true  # Also verify Rust compiles
```

## Migration Strategy

### Phase 1: Infrastructure (Week 1)
- Create TOML test loader/runner
- Define schema for test definitions
- Create migration tooling

### Phase 2: Core Tests (Weeks 2-3)
- Migrate fundamental language constructs
- String operations, dictionaries, lists
- Control flow, functions, classes

### Phase 3: Standard Library (Weeks 3-4)
- math, json, os, sys modules
- collections, itertools, functools
- datetime, random, re

### Phase 4: Advanced Features (Week 5)
- Generators, lambdas, comprehensions
- Error handling, type inference
- Ownership and borrowing patterns

### Phase 5: Cleanup (Week 6)
- Remove old test files
- Update CI/CD
- Documentation

## File Consolidation Plan

Tests will be consolidated from 191+ files into ~30 focused TOML files:

| New File | Consolidates From | Test Count (est) |
|----------|-------------------|------------------|
| 01-basic-types.toml | test_basic.rs, test_constants.rs, test_dynamic_type.rs | ~15 |
| 02-functions.toml | test_func_gens.rs, test_nested_functions.rs, test_default_parameters.rs | ~40 |
| 03-classes.toml | test_classs.rs, test_property.rs | ~50 |
| 04-control-flow.toml | test_if_elif_*.rs, test_try_*.rs | ~30 |
| 05-string-operations.toml | test_strings.rs, test_string_*.rs | ~60 |
| 06-assignment.toml | test_assignment.rs, test_stmt_gen_assign*.rs | ~40 |
| 07-dictionaries.toml | test_dicts.rs, test_dict_*.rs | ~35 |
| 08-lists-arrays.toml | test_array_*.rs, test_list_*.rs | ~45 |
| 09-slicing.toml | test_slice_*.rs, test_indexings.rs | ~25 |
| 10-comprehensions.toml | test_list_comprehension.rs | ~20 |
| 11-generators.toml | test_generators.rs, test_generator_*.rs | ~35 |
| 12-lambdas.toml | test_lambdas.rs, test_lambda_*.rs | ~25 |
| 13-iterators.toml | test_iterator_*.rs | ~20 |
| 14-operators.toml | test_operators.rs, test_power_*.rs | ~40 |
| 15-error-handling.toml | test_error_*.rs, test_exception_*.rs | ~45 |
| 16-type-inference.toml | test_type_*.rs | ~50 |
| 17-ownership.toml | test_ownership_*.rs, test_borrowing_*.rs, test_lifetimes.rs | ~35 |
| 18-mutability.toml | test_mutability.rs, test_clone.rs | ~20 |
| 19-optional-types.toml | test_optional_*.rs | ~30 |
| 20-result-types.toml | test_result_*.rs | ~20 |
| 21-math-module.toml | test_math*.rs | ~50 |
| 22-json-module.toml | test_json*.rs | ~15 |
| 23-os-sys-modules.toml | test_os_*.rs, test_sys_*.rs | ~30 |
| 24-collections-module.toml | test_collections_*.rs, test_counter_*.rs | ~25 |
| 25-itertools-module.toml | test_itertools*.rs | ~20 |
| 26-datetime-module.toml | test_datetime*.rs, test_time*.rs | ~25 |
| 27-random-module.toml | test_random*.rs | ~20 |
| 28-regex-module.toml | test_re_*.rs | ~15 |
| 29-io-modules.toml | test_csv_*.rs, test_pathlib_*.rs | ~30 |
| 30-misc-modules.toml | test_*_unit.rs (remaining) | ~50 |

## Task Files

Each migration task is documented in a separate file:

- `01-basic-types.md` - Basic type tests migration
- `02-functions.md` - Function tests migration
- `03-classes.md` - Class tests migration
- ...and so on

## Success Criteria

1. All 191+ pattern-based tests migrated to TOML
2. Test coverage maintained (no regressions)
3. CI passes with new test runner
4. Old test files removed
5. Documentation updated

## Implementation Notes

### Test Runner Design

```rust
// Conceptual test runner structure
struct TomlTestRunner {
    test_dir: PathBuf,
}

impl TomlTestRunner {
    fn run_all_tests(&self) -> TestResults {
        let tests = self.load_all_toml_tests();
        tests.par_iter().map(|t| self.run_test(t)).collect()
    }
    
    fn run_test(&self, test: &TomlTest) -> TestResult {
        let rust_code = transpile(&test.python.snippet);
        for pattern in &test.assertions.contains {
            assert!(rust_code.contains(pattern));
        }
        for pattern in &test.assertions.not_contains {
            assert!(!rust_code.contains(pattern));
        }
        // ...
    }
}
```

### Parallel Execution

Tests can be run in parallel since they're independent. The TOML format enables easy sharding for CI.

### Backward Compatibility

During migration, both old and new tests will run. A feature flag can disable old tests once migration is complete.
