# Test Helpers Migration Guide

## Task Description

Migrate all test files to use the centralized `test_helpers` module instead of directly using `DepylerPipeline::new()` and manual assertion patterns.

### Current State

- **8 files** already migrated
- **119 files** need migration

### Benefits of Migration

1. **Reduced Boilerplate**: Replace repeated `DepylerPipeline::new()` + manual assertions with single function calls
2. **Consistent Error Messages**: All tests get detailed, formatted error output automatically
3. **Automatic Compilation Validation**: `transpile_and_check()` and `transpile_and_compile()` verify the generated Rust code compiles
4. **Simpler Test Logic**: Tests focus on what patterns to expect, not how to run the pipeline

### Available Helper Functions

```rust
// Import the helpers
mod test_helpers;
use test_helpers::{transpile, transpile_and_check, transpile_check_absent, transpile_and_compile};

// Simple transpilation (no compilation check)
let rust_code = transpile(python_source);

// Transpile, check patterns exist, and verify compilation
let rust_code = transpile_and_check(python_source, &["pattern1", "pattern2"]);

// Transpile, check patterns are ABSENT, and verify compilation
let rust_code = transpile_check_absent(python_source, &["unwanted_pattern"]);

// Full result with compilation details
let result = transpile_and_compile(python_source, &["expected_pattern"]);
assert!(result.compilation_success);
```

### Migration Pattern

**Before:**
```rust
use depyler_core::DepylerPipeline;

#[test]
fn test_something() {
    let python = r#"
def foo(x: int) -> int:
    return x + 1
"#;
    let pipeline = DepylerPipeline::new();
    let result = pipeline.transpile(python);
    assert!(result.is_ok());
    let rust_code = result.unwrap();
    assert!(rust_code.contains("fn foo"));
}
```

**After:**
```rust
mod test_helpers;
use test_helpers::transpile_and_check;

#[test]
fn test_something() {
    let python = r#"
def foo(x: int) -> int:
    return x + 1
"#;
    transpile_and_check(python, &["fn foo"]);
}
```

---

## Files to Migrate

### A

- [ ] `tests/advanced_property_generators.rs`
- [ ] `tests/argparse_transpilation.rs`
- [ ] `tests/argparse_type_inference.rs`
- [ ] `tests/argument_type_error.rs`
- [ ] `tests/array_generation_test.rs`
- [ ] `tests/array_literal_regression_test.rs`
- [x] `tests/assignment_test.rs`

### B

- [ ] `tests/boolean_conversion_test.rs`
- [ ] `tests/boundary_value_tests.rs`
- [ ] `tests/bug_regression_tests.rs`

### C

- [ ] `tests/class_tests.rs`
- [ ] `tests/cli_tests.rs`
- [ ] `tests/codegen_coverage.rs`
- [ ] `tests/collections_showcase_test.rs`
- [x] `tests/constants_test.rs`
- [ ] `tests/context_coverage_test.rs`
- [ ] `tests/copy_bug_test.rs`
- [ ] `tests/coverage_analysis.rs`
- [ ] `tests/csv_api.rs`
- [ ] `tests/csv_kwargs.rs`
- [ ] `tests/custom_attributes_test.rs`

### D

- [ ] `tests/dataflow_type_casting_test.rs`
- [ ] `tests/debug_collections_test.rs`
- [x] `tests/default_arguments_test.rs`
- [ ] `tests/dict_tests.rs`
- [ ] `tests/dict_value_operations.rs`
- [ ] `tests/direct_rules_coverage.rs`
- [ ] `tests/direct_rules_coverage_test.rs`
- [ ] `tests/dynamic_type_test.rs`

### E

- [ ] `tests/edge_case_coverage.rs`
- [ ] `tests/error_gen_coverage.rs`
- [ ] `tests/error_handling_tests.rs`
- [ ] `tests/error_path_coverage.rs`
- [ ] `tests/example_validation.rs`
- [ ] `tests/exception_handling_tests.rs`
- [ ] `tests/exception_scope_test.rs`
- [ ] `tests/expr_gen_coverage_test.rs`
- [ ] `tests/expr_gen_extended_coverage_test.rs`
- [ ] `tests/expr_gen_methods_coverage.rs`
- [ ] `tests/expr_gen_untested_builtins.rs`

### F

- [x] `tests/final_constant_test.rs`
- [ ] `tests/floor_div_zero_handler.rs`
- [ ] `tests/formatting_test.rs`
- [ ] `tests/func_gen_tests.rs`
- [ ] `tests/function_borrowing_test.rs`
- [ ] `tests/functional_tests.rs`
- [ ] `tests/fuzzing_tests.rs`

### G

- [ ] `tests/generator_tests.rs`

### I

- [ ] `tests/if_elif_variable_shadowing.rs`
- [ ] `tests/import_gen_coverage_test.rs`
- [ ] `tests/indexing_tests.rs`
- [ ] `tests/int_str_parsing_test.rs`
- [ ] `tests/integration_benchmarks.rs`
- [ ] `tests/interactive_doctests.rs`
- [ ] `tests/isinstance_test.rs`
- [ ] `tests/iterator_deref_test.rs`

### L

- [ ] `tests/lambda_tests.rs`
- [ ] `tests/lifetime_tests.rs`
- [ ] `tests/list_comprehension_test.rs`

### M

- [ ] `tests/main_return_type_test.rs`
- [x] `tests/math_tests.rs`
- [ ] `tests/method_ownership_test.rs`
- [ ] `tests/mutability_test.rs`
- [ ] `tests/mutation_testing.rs`

### N

- [ ] `tests/nested_functions.rs`

### O

- [ ] `tests/option_type_mismatch.rs`
- [ ] `tests/os_sys_platform.rs`

### P

- [ ] `tests/phase2_test.rs`
- [ ] `tests/phase3_test.rs`
- [ ] `tests/property_based_test_generation.rs`
- [ ] `tests/property_test.rs`
- [ ] `tests/property_test_benchmarks.rs`
- [ ] `tests/property_tests_ast_roundtrip.rs`
- [ ] `tests/property_tests_memory_safety.rs`
- [ ] `tests/property_tests_type_inference.rs`
- [ ] `tests/python_collections_test.rs`

### Q

- [ ] `tests/quality_assurance_automation.rs`

### R

- [ ] `tests/range_step_test.rs`
- [ ] `tests/range_v1_test.rs`
- [ ] `tests/result_return_wrapping.rs`
- [ ] `tests/result_unwrapping_test.rs`
- [ ] `tests/return_type_inference.rs`
- [ ] `tests/rust_gen_coverage_test.rs`
- [ ] `tests/rust_keywords_test.rs`

### S

- [x] `tests/setattr_test.rs`
- [ ] `tests/show_types_test.rs`
- [ ] `tests/slice_operations_test.rs`
- [ ] `tests/sorted_tests.rs`
- [ ] `tests/specialized_coverage_testing.rs`
- [ ] `tests/star_args_unpacking_test.rs`
- [ ] `tests/stmt_gen_assign_coverage_test.rs`
- [ ] `tests/stmt_gen_assign_index_coverage_test.rs`
- [ ] `tests/stmt_gen_assign_symbol_coverage_test.rs`
- [ ] `tests/stmt_gen_coverage_test.rs`
- [ ] `tests/stmt_gen_extended_coverage_test.rs`
- [ ] `tests/stmt_gen_for_coverage_test.rs`
- [ ] `tests/stmt_gen_if_coverage_test.rs`
- [ ] `tests/stmt_gen_raise_coverage_test.rs`
- [ ] `tests/stmt_gen_return_coverage_test.rs`
- [ ] `tests/stmt_gen_try_coverage_test.rs`
- [ ] `tests/stream_handling.rs`
- [ ] `tests/string_interoperability_test.rs`
- [ ] `tests/string_literals_os_module_test.rs`
- [ ] `tests/string_optimization_tests.rs`
- [ ] `tests/string_tests.rs`
- [ ] `tests/struct_field_borrow_test.rs`
- [ ] `tests/subcommand_field_access.rs`

### T

- [ ] `tests/ternary_expression_test.rs`
- [ ] `tests/test_argparse_handler_types.rs`
- [ ] `tests/test_collection_operations.rs`
- [ ] `tests/test_generic_inference.rs`
- [ ] `tests/try_block_analysis_test.rs`
- [ ] `tests/try_except_control_flow.rs`
- [x] `tests/tuple_test.rs`
- [ ] `tests/type_annotation_test.rs`
- [ ] `tests/type_gen_coverage_test.rs`
- [ ] `tests/type_inference.rs`
- [ ] `tests/type_mapper_coverage_test.rs`
- [ ] `tests/type_mapper_extended_coverage_test.rs`
- [ ] `tests/type_system.rs`

### U

- [ ] `tests/uninitialized_declarations_test.rs`
- [ ] `tests/unnecessary_returns_test.rs`
- [ ] `tests/unused_loop_vars_test.rs`

### V

- [ ] `tests/v3_17_coverage_tests.rs`
- [ ] `tests/validator_return_type.rs`
- [ ] `tests/valueerror_test.rs`

---

## Already Migrated

- [x] `tests/assignment_test.rs`
- [x] `tests/clone_test.rs`
- [x] `tests/constants_test.rs`
- [x] `tests/default_arguments_test.rs`
- [x] `tests/final_constant_test.rs`
- [x] `tests/math_tests.rs`
- [x] `tests/setattr_test.rs`
- [x] `tests/tuple_test.rs`

---

## Notes

- Some test files may require additional helper functions depending on their specific needs
- Property-based tests using `proptest` or `quickcheck` may need different migration strategies
- CLI tests that test the binary directly may not benefit from this migration
