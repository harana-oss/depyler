# Depyler Outstanding Issues

## Failing Tests

### regressions.toml
- `list_indexing_bounds_checking` - Missing `IndexError` struct definition
- `bounds_checking_array_indexing` - Missing `IndexError` struct definition
- `copy_copy_list` - Unwanted `#[doc = "// NOTE: Map Python module 'copy'()"]`
- `copy_copy_dict` - Unwanted `#[doc = "// NOTE: Map Python module 'copy'()"]`
- `copy_deepcopy_list` - Missing `IndexError`, unwanted module comment
- `untyped_list_parameter` - `total = total + num` instead of `total += num`
- `untyped_dict_parameter` - Wrong dict access pattern
- `rust_code_formatting_consistency` - `total = total + n` instead of `total += n`
- `large_conditional_function` - CSE temps not inlined, no augmented assignment
- `sports_simulation_dataclasses` - Missing `Copy` derive, unwanted module comment
- `sports_simulation_game_logic` - CSE temps, string comparison with `.to_string()`
- `sports_simulation_event_handler` - Missing `IndexError`, unwanted module comment

### resource-management.toml
- `file_handle_cleanup` - Wrong file I/O pattern (generates `?` without Result)
- `file_write_cleanup` - Wrong file I/O pattern
- `file_lines_cleanup` - Wrong file I/O pattern
- `socket_cleanup` - Extra `_get_field`/`_set_field` methods
- `socket_auto_close` - Missing `Drop` impl, extra methods
- `tempfile_cleanup` - Extra `_get_field`/`_set_field` methods
- `nested_resource_cleanup` - Wrong struct order, extra methods
- `resource_stack` - Uses `serde_json::Value` instead of `String`
- `exception_during_cleanup` - Extra `_get_field`/`_set_field` methods
- `cleanup_error_handling` - Unwanted `ValueError` struct, wrong control flow
- `finally_cleanup` - Extra braces around code

### result-error-propagation.toml
- `plain_caller_uses_result_function` - CSE temps, wrong try/except handling
- `function_returning_error_with_format_string` - Extra parens in `||` condition

### return-statements.toml
- `early_return` - CSE temps not inlined
- `final_vs_early_return` - CSE temps not inlined
- `optional_early_return_none` - Missing `IndexError`, CSE temps
- `result_return_ok` - Missing error types, wrong division operator
- `result_early_return` - Missing error types, wrong division operator
- `result_optional_return_some` - CSE temps not inlined
- `result_optional_return_none` - CSE temps not inlined
- `result_optional_early_return_some` - CSE temps not inlined
- `result_optional_early_return_none` - Missing `IndexError`, CSE temps
- `empty_return_can_fail` - CSE temps not inlined
- `empty_return_can_fail_optional` - CSE temps not inlined
- `multiple_early_returns` - CSE temps not inlined
- `complex_return_patterns` - CSE temps not inlined

### return-value-mutation.toml
- `return_value_mutated_at_call_site` - Missing `Copy`, unwanted module comment
- `return_value_not_mutated` - Missing `Copy`, unwanted module comment
- `return_value_method_mutation` - Unwanted module comment
- `nested_return_value_access` - Missing `Copy`, unwanted module comment

### string-operations.toml
- `string_slice_chars` - Missing `IndexError`
- ~~`string_multiply` - Extra semicolon after function~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- ~~`string_slice_last_n` - Extra semicolon~~ **FIXED** (Test expectation error - added missing semicolon and closing brace)
- ~~`string_slice_without_last_n` - Extra semicolon~~ **FIXED** (Test expectation error - added missing semicolon and closing brace)
- ~~`string_reverse` - Extra semicolon~~ **FIXED** (Test expectation error - added missing semicolon and closing brace)
- ~~`string_slice_start_stop` - Extra semicolon~~ **FIXED** (Test expectation error - added missing semicolon and closing brace)
- `fstring_concatenation` - CSE temp not inlined
- ~~`fstring_in_function_arg` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `string_center` - Formatting differs
- ~~`string_ljust` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- ~~`string_rjust` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- ~~`string_zfill` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- ~~`str_expandtabs` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `str_format` - Should fail but succeeds
- ~~`string_param_owned` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `string_clone_field_access` - Returns `&String` instead of `String`
- ~~`string_repeat` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `int_to_string_conversion` - Extra parens `(n).to_string()`
- ~~`vec_string_type` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `hashmap_string_key` - Missing `IndexError`
- ~~`string_chained_methods` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `string_ternary` - Formatting differs
- ~~`hashmap_string_value` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- ~~`hashmap_string_key_literal` - Extra semicolon~~ **FIXED** (Test expectation error - removed incorrect semicolon from expected output)
- `string_local_variable` - Extra parens in `.to_string()`
- `string_utils` - Many issues: error types, CSE temps, string handling
- `text_analyzer` - Many issues: error types, CSE temps, string handling
- `text_processing_combined` - Many issues: missing const, error types
- `string_concat_in_function_arg` - Uses nested `format!` instead of single
- `string_concat_str_param` - Extra `as i32`
- `string_concat_nested_expression` - Nested `format!` calls
- `string_literal_plus_variable` - Nested `format!` calls
- `string_concat_chained_calls` - Extra parens in `.to_string()`

### template-strings.toml (all 20 tests fail)
- `template_simple` through `template_no_placeholders` - `string.Template` not supported

### ternary-expressions.toml
- `ternary_strings` - `.to_string()` on literals, formatting
- `ternary_nested` - Not flattened to `else if`
- `ternary_deeply_nested` - Not flattened to `else if`
- `ternary_function_calls` - Extra `pub`, missing `&'static str`
- `ternary_method_calls` - Extra derives and methods
- `ternary_with_comprehension` - `.iter().cloned()` instead of `.into_iter()`
- `ternary_augmented_assignment` - Missing `mut`
- `ternary_as_argument` - Extra `pub`, wrong return style
- `ternary_multiple_args` - Wrong return type (missing `-> i32`)
- `ternary_with_none` - Wrong Option handling
- `ternary_same_type` - Uses `type_name_of_val` incorrectly
- `ternary_numeric_promotion` - Should be module-level const
- `ternary_short_circuit` - Extra `pub`, wrong return style
- `ternary_float_cast_no_reference` - Extra derives and methods
- `ternary_int_cast_no_reference` - Extra derives and methods
- `ternary_float_cast_with_expression` - Extra derives and methods

### trait-impls.toml
- `default_impl_complex` - `Vec::new()` vs `vec![]`, `String::new()` vs `"".to_string()`
- `clone_explicit` - `Vec::new()` vs `vec![]`

### type-alias-statement.toml (all 16 tests fail)
- `type_alias_basic` through `type_alias_runtime` - Wrong order (const before type), uses `pub const` instead of `let`

### type-inference.toml
- `infer_float_from_division` - Missing `ZeroDivisionError`
- `infer_string_from_str_call` - Extra parens
- `annotated_parameter` - Extra quickcheck boilerplate
- `infer_list_element_type` - Missing `IndexError`
- `infer_dict_types` - Missing `IndexError`
- `optional_type_annotation` - CSE temp
- `infer_filepath_from_open` - Extra semicolon
- `infer_list_from_iteration` - No augmented assignment
- `infer_csv_path` - Missing `IndexError`
- `int_str_multiple_calls` - CSE temps
- `untyped_list_parameter_no_dynamic` - No augmented assignment
- `untyped_dict_parameter_no_dynamic` - Missing `IndexError`, CSE temp
- `type_annotation_usize_to_i32` - Uses `saturating_sub` differently
- `simple_generic_function` - Extra semicolon
- `generic_list_function` - Missing `IndexError`
- `generic_dict` - Missing `IndexError`, wrong type param order
- `type_var_in_optional` - Extra semicolon
- `generic_method_with_type_vars` - Generates `TypeVar::new` const
- `generic_call_with_keyword_args_and_array` - RNG handling differs
- `type_mapper_union_without_none_enum` - Extra semicolon
- `type_mapper_generic_dict_type` - Wrong `unwrap_or` pattern
- `type_mapper_custom_type_parameter` - Generates `TypeVar::new` const
- `type_mapper_custom_type_name` - Missing `Copy`
- `type_mapper_reference_with_lifetime` - Extra semicolon
- `type_mapper_result_type` - CSE temp
- `type_mapper_typevar_mapping` - Generates `TypeVar::new` consts
- `type_mapper_all_type_features` - CSE temps
- `int_type_default_i32` - Extra quickcheck boilerplate
- `float_type_mapping` - Extra quickcheck boilerplate
- `list_with_inner_type` - No augmented assignment
- `dict_with_key_value_types` - Wrong `unwrap_or` pattern
- `custom_type_single_letter_type_param` - Generates `TypeVar::new` const
- `custom_type_dict_no_params` - Extra semicolon
- `typevar_mapping` - Generates `TypeVar::new` const
- `union_multiple_types` - Extra parens in `.to_string()`
- `complex_type_combinations` - CSE temps
- `union_type_processing` - Extra semicolon
- `union_enum_reuse` - Extra quickcheck boilerplate
- `complex_context_union` - CSE temp
- `named_import_item_any` - Extra semicolon
- `typevar_import` - Generates `TypeVar::new` const
- `unmapped_import` - Unwanted module comment

### type-parameter-lists.toml (all 14 tests fail)
- `typeparam_function` through `typeparam_variance` - Generates `pub const result: bool = true;` instead of proper generic code

### unicode-strings.toml
- `unicode_nfc` - Unwanted module comment
- `unicode_nfd` - Unwanted module comment
- `unicode_nfkc` - Unwanted module comment
- `unicode_nfkd` - Unwanted module comment
- `unicode_category` - **Not implemented**: `unicodedata.category`
- `unicode_charname` - **Not implemented**: `unicodedata.name`
- `unicode_lookup` - **Not implemented**: `unicodedata.lookup`
- `unicode_eq_normal` - Unwanted module comment
- `unicode_locale` - Unwanted module comment

### version-features.toml
- `exception_groups_311` - **Not implemented**: `except*` syntax
- `self_type_311` - **Not implemented**: `Self` type in classes

---

## Test Commands

```bash
# Run all TOML tests
cargo run -- test -j

# Run specific test file
cargo run -- test -p tests/toml/exceptions.toml

# Filter by name
cargo run -- test -f "abc"

# With compilation verification
cargo run -- test -c
```

