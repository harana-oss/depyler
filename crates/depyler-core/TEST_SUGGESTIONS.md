# Test Suggestions for depyler-core

This document contains test suggestions derived from comments, TODOs, FIXMEs, and implementation notes found in the crate.

## Items Moved to TOML Tests

The following items have been implemented as TOML transpilation tests in `/tests/toml/`:

- **#1 Regex Groups** → `modules-regex.toml` (re_match_groups, re_search_groups, re_match_vs_search_start)
- **#2 File Open Mode Tracking** → `file.toml` (file_read_text_mode, file_readlines_text_mode, file_readline_single, file_write_text_vs_binary, file_read_binary_mode_uses_text, file_iteration)
- **#3 Function Parameter Kwargs** → `functions.toml` (function_keyword_arguments, function_mixed_positional_kwargs, function_default_parameter_values, function_kwargs_method_call, function_multiple_defaults)
- **#4 UUID v1 Implementation** → `stdlib-misc.toml` (uuid1_uses_v4_placeholder, uuid4_random_generation, uuid_multiple_calls, uuid_in_data_structure)
- **#5 CSV DictWriter Fieldnames** → `modules-csv.toml` (csv_dictwriter_writerow_dict, csv_dictwriter_writeheader, csv_dictwriter_fieldnames_positional, csv_dictwriter_multiple_rows, csv_dictwriter_writerows_batch)
- **#5 HMAC** → `crypto.toml` (hmac_compare_digest, hmac_compare_digest_strings)
- **#6 re.match Start-of-String** → `modules-regex.toml` (re_match_vs_search_start)
- **#6 Generator Liveness Analysis** → `generator-liveness.toml` (generator_captures_used_variables, generator_variables_modified_between_yields, generator_nested_loops_with_yields, generator_with_local_scope_variables, generator_with_parameter_state)
- **#7 Optimizer Inlining Bug** → `optimizer-inlining.toml` (inlining_disabled_trivial_function_preserved, dead_code_elimination_preserves_used_assignments, inline_threshold_simple_function, function_chain_no_inline, dce_preserves_intermediate_variables)
- **#8 subprocess.run** → `modules-os-sys.toml` (subprocess_run_basic, subprocess_run_capture_output, subprocess_run_with_cwd, subprocess_run_check)
- **#8 Reference Return Type Bug** → `reference-return-types.toml` (constructor_return_no_reference, factory_method_returns_owned_value, tuple_return_with_constructed_values, struct_literal_return_direct_value, nested_constructor_in_return)
- **#9 String Method Return Types** → `string-methods-all.toml` (upper_returns_string, lower_returns_string, strip_returns_string, replace_returns_string, chained_string_methods_compile, string_method_on_literal)
- **#10 Bool to Int Casting** → `basic-types.toml` (int_bool_variable_cast, int_comparison_result_cast, int_bool_arithmetic, int_true_literal, int_false_literal)
- **#11 Float Literals** → `basic-types.toml` (float_literal_zero, float_literal_whole_number, float_literal_with_decimal, float_literal_negative_zero, float_literal_small)
- **#11 Error Propagation with ? Operator** → `result-error-propagation.toml` (result_caller_calls_plain_return, plain_caller_uses_result_function, nested_plain_function_calls, chained_function_calls_no_question_mark, function_returning_error_with_format_string)
- **#12 Integer Division** → `operators.toml` (int_div_int_returns_float, mixed_int_float_division, expression_division_returns_float)
- **#12 Generator State Struct Naming** → `generators.toml` (generator_state_snake_case_to_pascal, generator_state_multiple_underscores, generator_state_single_word)
- **#13 Generator Type Inference** → `generators.toml` (generator_state_type_from_assignment, generator_state_type_from_param)
- **#14 argparse Full Implementation** → `argparse.toml` (argparse_subcommand_basic, argparse_mutually_exclusive_basic, argparse_nargs_patterns, argparse_type_choices, argparse_nested_subcommands, argparse_argument_groups)
- **#15 Dead Code Elimination with Side Effects** → `dead-code-elimination.toml` (unused_assignment_eliminated, indexing_side_effect_preserved, method_call_side_effect_preserved, property_write_preserved, chained_assignments_only_used_preserved, multiple_unused_eliminated, loop_body_mutation_preserved, conditional_side_effect_preserved)
- **#16 String Concatenation in Function Arguments** → `string-operations.toml` (string_concat_in_function_arg, string_concat_str_param, string_concat_nested_expression, string_literal_plus_variable, string_concat_chained_calls, string_concat_multiple_args)
- **#17 Optional Type Unwrapping in Binary Operations** → `optional-types.toml` (optional_in_binary_addition, optional_field_method_call, optional_in_collection_lookup, optional_chain_access, optional_none_check_before_operation, optional_comparison_operation, optional_string_concatenation, optional_multiplication)
- **#18 Lifetime Analysis for Parameters** → `lifetime-parameters.toml` (function_parameters_no_static_lifetime, borrowed_list_parameter, multiple_parameters_lifetime_inference, string_parameter_owned, struct_field_access_borrowed)
- **#19 Timedelta** → `modules-datetime-time.toml` (timedelta_days, timedelta_hours_minutes, timedelta_seconds, timedelta_positional, timedelta_arithmetic)
- **#19 CSE Complexity** → `cse-optimization.toml` (method_call_list_index, method_call_str_find, len_function_cached, arithmetic_with_function_call, simple_addition_no_cse, simple_multiplication_no_cse)
- **#20 Base32** → `crypto.toml` (base64_b32encode, base64_b32decode - documented as requiring data-encoding crate)
- **#20 Iterator Type Handling** → `iterators.toml` (for_in_list_iteration, for_enumerate_with_value_access, hashmap_values_iteration, zip_into_iter_consumption, iter_vs_into_iter_choice)
- **#21 Error Type Wrapping** → `error-handling.toml` (single_error_type_return, raise_with_concrete_error, try_except_with_multiple_exceptions, error_from_format_string)
- **#22 Truthiness Conversion** → `basic-types.toml` (truthiness_string_if, truthiness_list_if, truthiness_dict_if, truthiness_integer_if)
- **#23 Power Operator Type Handling** → `operators.toml` (power_int_literal_exponent, power_float_exponent, power_variable_exponent, power_both_variables, pow_builtin_same_as_operator)
- **#24 Parameter Mutability Analysis** → `parameter-mutability.toml` (copy_type_reassignment_mut_not_ref_mut, non_copy_type_mutation_needs_ref_mut, attribute_mutation_needs_ref_mut, nested_field_mutation_needs_ref_mut, no_mutation_no_ref_mut, method_mutation_on_param_field)
- **#25 Field Alias Mutation Detection** → `field-alias-mutation.toml` (field_alias_attribute_mutation, field_alias_index_mutation, field_alias_method_mutation, value_copy_does_not_mark_param_mutated, direct_param_field_access_no_alias)
- **#26 is_copy_type() Classification** → `copy-type-semantics.toml` (primitive_int_is_copy, primitive_float_is_copy, primitive_bool_is_copy, string_is_not_copy, list_is_not_copy, dict_is_not_copy, tuple_of_copy_types)
- **#28 Return Value Mutation Analysis** → `return-value-mutation.toml` (return_value_mutated_at_call_site, return_value_not_mutated, return_value_method_mutation, nested_return_value_access)
- **#29 Parameter Borrowing Analysis** → `parameter-borrowing.toml` (string_parameter_owned, list_parameter_borrowed, struct_parameter_borrowed_read_only, function_returning_field_reference, multiple_string_params_owned)
- **#30 Mutable Variable Detection** → `mutable-variable-detection.toml` (reassignment_marks_variable_mutable, first_assignment_not_mutable, mutating_method_call_marks_mutable, augmented_assignment_marks_mutable, function_parameter_reassignment, deferred_initialization_not_mutable, loop_variable_mutable)
- **#31 is_mutating_method_name() Classification** → `mutating-method-names.toml` (list_append_is_mutating, list_extend_is_mutating, list_pop_is_mutating, list_clear_is_mutating, list_sort_is_mutating, list_reverse_is_mutating, dict_update_is_mutating, set_add_is_mutating, len_is_not_mutating, get_is_not_mutating, keys_is_not_mutating)
- **#32 Import Deduplication** → `import-deduplication.toml` (duplicate_hashmap_import_deduplicated, different_imports_both_kept, function_definitions_preserved)
- **#33 Conditional Import Generation** → `conditional-imports.toml` (hashmap_usage_generates_import, hashset_usage_generates_import, vecdeque_usage_generates_import, random_usage_generates_import, json_usage_generates_import, no_special_types_no_imports, multiple_types_multiple_imports)
- **#34 Constant Type Inference** → `constant-type-inference.toml` (literal_int_type, literal_float_type, literal_string_type, literal_bool_type, unary_minus_int_type, binary_add_int_type, binary_div_float_type, binary_floordiv_int_type, int_builtin_call_type, float_builtin_call_type, str_builtin_call_type)
- **#35 Lazy Static Requirement Detection** → `lazy-static-detection.toml` (vec_constant_requires_lazy_static, hashmap_constant_requires_lazy_static, string_constant_requires_lazy_static, int_constant_no_lazy_static, float_constant_no_lazy_static, bool_constant_no_lazy_static)
- **#36 Enum Copy Semantics** → `enum-copy-semantics.toml` (enum_type_no_clone, enum_multiple_uses_no_clone, struct_needs_clone, enum_in_match)
- **#37 Cast Expressions Without References** → `cast-expressions.toml` (float_cast_no_references, int_cast_no_references, str_cast_from_int, bool_cast_from_int, nested_cast_expression, cast_with_arithmetic)
- **#38 For Loop Iterator Mutation Detection** → `for-loop-iterator-mutation.toml` (for_loop_param_field_mutation, for_loop_param_field_no_mutation, for_loop_local_variable_iteration, for_loop_enumerate_mutation, for_loop_method_mutation_on_element, nested_for_loops_with_mutation)
- **#27 is_param_attribute_access() Detection** → `param-attribute-access.toml` (direct_attribute_access, nested_attribute_access, non_matching_param, variable_not_attribute, method_call_on_param, deeply_nested_attribute_access, multiple_params_attribute_access)

---

## Summary Statistics

- **Total remaining test suggestions:** 0 items (originally 45, all moved to TOML tests)

---

## Test Organization Recommendations

1. **Create test files by module**: e.g., `expr_gen_tests.rs`, `func_gen_tests.rs`
2. **Use `#[cfg(test)]` modules** within existing files where appropriate
3. **Create integration tests** in `tests/` directory for end-to-end transpilation
4. **Use property-based testing** for type conversion edge cases
5. **Document limitations** in test names: `test_uuid1_uses_v4_placeholder`
6. **For rust_gen.rs specifically**:
   - Add tests to the existing `#[cfg(test)]` module at the bottom of the file
   - Test helper functions like `is_copy_type`, `is_mutating_method_name` independently
   - Create HIR fixtures for complex mutation analysis scenarios
   - Use `create_test_context()` for consistent test setup
