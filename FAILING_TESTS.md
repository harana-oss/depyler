# Failing TOML Tests

**Test Summary:** 3915 total, 3682 passed, 233 failed, 0 skipped

## Parse Errors (4 files)

| File | Error |
|------|-------|
| `async-for.toml` | Failed to parse |
| `complex-numbers.toml` | Failed to parse |
| `examples-stdlib.toml` | Failed to parse |
| `exception-groups.toml` | Failed to parse |

## Failed Tests by Category

### Statement Type Not Yet Supported

| File | Test | Reason |
|------|------|--------|
| `augmented-assignment.toml` | `augmented_add` | Python parse error: unexpected indent at byte offset 0 |
| `async-comprehensions.toml` | `async_gen_expr` | Statement type not yet supported: AsyncFor |
| `async-functions.toml` | `async_closure` | Statement type not yet supported: AsyncFunctionDef |
| `async-generators.toml` | `async_generator_iterate` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_multi_yield` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_loop` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_try_finally` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_early_return` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_streaming` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_cursor` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_events` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_break_cleanup` | Statement type not yet supported: AsyncFor |
| `async-generators.toml` | `async_gen_nested` | Statement type not yet supported: AsyncFor |
| `asyncio-tasks.toml` | `task_group` | Statement type not yet supported: AsyncWith |
| `asyncio-tasks.toml` | `task_group_exception` | Statement type not yet supported: AsyncWith |
| `asyncio-tasks.toml` | `asyncio_lock` | Statement type not yet supported: AsyncWith |
| `asyncio-tasks.toml` | `asyncio_semaphore` | Statement type not yet supported: AsyncWith |
| `await-expressions.toml` | `await_nested` | Statement type not yet supported: AsyncFunctionDef |
| `chainmap.toml` | `chainmap_del` | Statement type not yet supported: Delete |
| `chained-comparisons.toml` | `chained_short_circuit` | Statement type not yet supported: Global |
| `cli-integration.toml` | `test_wordcount` | Statement type not yet supported: Import |
| `closures-mutable.toml` | `nonlocal_basic` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_modify` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_increment` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_counter` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_state` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_getter_setter` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_shared` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_inc_dec` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `mutable_factory_shared` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_chain` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_skip` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_method` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_del` | Statement type not yet supported: Nonlocal |
| `closures-mutable.toml` | `nonlocal_multiple` | Statement type not yet supported: Nonlocal |
| `contextlib-decorator.toml` | `contextmanager_temp_value` | Statement type not yet supported: Global |
| `contextlib-decorator.toml` | `contextmanager_environ` | Statement type not yet supported: Delete |
| `contextlib-utilities.toml` | `async_exitstack` | Statement type not yet supported: AsyncWith |
| `contextlib-utilities.toml` | `async_exitstack_enter` | Statement type not yet supported: AsyncWith |
| `contextlib-utilities.toml` | `aclosing` | Statement type not yet supported: AsyncWith |
| `enum-advanced.toml` | `enum_unique_error` | Statement type not yet supported: ClassDef (classes) |
| `enum-advanced.toml` | `enum_no_extend` | Statement type not yet supported: ClassDef (classes) |
| `functools-cache.toml` | `cache_hits` | Statement type not yet supported: Global |
| `functools-cache.toml` | `cache_different_args` | Statement type not yet supported: Global |
| `functools-cache.toml` | `lru_cache_typed` | Statement type not yet supported: Global |
| `functools-cache.toml` | `cached_property_once` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `global_basic` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `global_write` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `global_multiple` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `global_nested` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `nonlocal_basic` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `nonlocal_write` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `nonlocal_multiple` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `nonlocal_chain` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `closure_counter` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `closures_shared_state` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `global_no_assignment` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `global_in_method` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `nonlocal_error` | Statement type not yet supported: Nonlocal |
| `global-nonlocal.toml` | `global_and_nonlocal` | Statement type not yet supported: Global |
| `global-nonlocal.toml` | `mixed_scopes` | Statement type not yet supported: Nonlocal |
| `imports-basic.toml` | `import_in_function` | Statement type not yet supported: Import |
| `properties.toml` | `property_deleter` | Statement type not yet supported: Delete |
| `properties.toml` | `property_function` | Statement type not yet supported: Delete |

### Expression Type Not Yet Supported

| File | Test | Reason |
|------|------|--------|
| `class-methods.toml` | `staticmethod_utility` | Expression type not yet supported: Slice with step -1 |
| `classes.toml` | `basic_class_test` | Expression type not yet supported: FString with self.name |
| `classes.toml` | `test_class_methods` | Expression type not yet supported: FString with arguments |
| `dunder-context.toml` | `dunder_exit` | Expression type not yet supported: FString with exc_type |
| `dunder-context.toml` | `exit_no_exception` | Expression type not yet supported: FString with exc_type |
| `dunder-context.toml` | `exit_with_exception` | Expression type not yet supported: FString with __name__ |
| `dunder-context.toml` | `exit_exception_info` | Expression type not yet supported: FString with exc_val |
| `dunder-context.toml` | `cm_file_like` | Expression type not yet supported: FString with self.name |
| `dunder-context.toml` | `cm_timing` | Expression type not yet supported: FString with elapsed |
| `dunder-context.toml` | `cm_temp_state` | Expression type not yet supported: FString with self.new_val |
| `dunder-context.toml` | `nested_with` | Expression type not yet supported: FString with self.name |
| `dunder-context.toml` | `multiple_cm` | Expression type not yet supported: FString with self.name |
| `dunder-context.toml` | `cm_order` | Expression type not yet supported: FString with self.n |
| `dunder-context.toml` | `cm_reusable` | Expression type not yet supported: FString with self.name |
| `dunder-context.toml` | `cm_reentrant` | Expression type not yet supported: FString with self.level |
| `dunder-iteration.toml` | `dunder_iter_generator` | Expression type not yet supported: Yield |
| `dunder-iteration.toml` | `range_iterator` | Expression type not yet supported: Yield |
| `examples-data-structures.toml` | `queue` | Expression type not yet supported: Slice with variable start |
| `examples-file-processing.toml` | `csv_parser` | Expression type not yet supported: Slice with start 1 |
| `examples-file-processing.toml` | `log_analyzer` | Expression type not yet supported: Slice with start 3 |
| `examples-game-development.toml` | `tic_tac_toe` | Expression type not yet supported: IfExpr |
| `examples-networking.toml` | `http_client` | Expression type not yet supported: FString in request |
| `examples-tooling.toml` | `interactive_annotation` | Expression type not yet supported: Slice with variable bounds |
| `examples-tooling.toml` | `mcp_usage` | Expression type not yet supported: FString with emoji |
| `examples-web-scraping.toml` | `url_parser` | Expression type not yet supported: Slice with variable end |
| `functools-partial.toml` | `partialmethod_class` | Expression type not yet supported: FString with greeting |
| `keyword-only-args.toml` | `kwonly_classmethod` | Expression type not yet supported: FString with name/value |
| `multiple-inheritance.toml` | `mixin_methods` | Expression type not yet supported: FString with message |
| `walrus-operator.toml` | `walrus_in_list_comprehension` | Expression type not yet supported |
| `walrus-operator.toml` | `walrus_in_generator` | Expression type not yet supported |

### PANIC Errors

| File | Test | Reason |
|------|------|--------|
| `cast-expressions.toml` | `nested_cast_expression` | PANIC: casts cannot be followed by a method call |
| `closures-late-binding.toml` | `late_binding_fix_partial` | PANIC: Ident is not allowed to be empty |
| `closures.toml` | `closure_loop_fix_partial` | PANIC: Ident is not allowed to be empty |
| `dataclasses-field.toml` | `field_kw_only_sentinel` | PANIC: expected identifier or integer |
| `functools-partial.toml` | `partial_basic` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_call` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_keyword` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_multi_args` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_mixed` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_add_args` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_func` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_args` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_keywords` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_callback` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_defaults` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_adapter` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_map` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_vs_lambda` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_advantages` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_nested` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_varargs` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_kwargs_func` | PANIC: Ident is not allowed to be empty |
| `functools-partial.toml` | `partial_override` | PANIC: Ident is not allowed to be empty |
| `functools-wraps.toml` | `wraps_partial` | PANIC: Ident is not allowed to be empty |
| `imports-basic.toml` | `import_dotted` | PANIC: "os.path" is not a valid Ident |
| `modules-itertools-functools.toml` | `functools_partial` | PANIC: Ident is not allowed to be empty |
| `modules-os-sys.toml` | `os_path_exists` | PANIC: "os.path" is not a valid Ident |
| `modules-os-sys.toml` | `os_path_isfile` | PANIC: "os.path" is not a valid Ident |
| `modules-os-sys.toml` | `os_path_isdir` | PANIC: "os.path" is not a valid Ident |
| `modules-os-sys.toml` | `os_path_basename` | PANIC: "os.path" is not a valid Ident |
| `modules-os-sys.toml` | `os_path_dirname` | PANIC: "os.path" is not a valid Ident |
| `modules-os-sys.toml` | `os_path_relpath` | PANIC: "os.path" is not a valid Ident |
| `nested-context-managers.toml` | `multi_dynamic` | PANIC: expected one of: identifier, etc. |

### Unsupported Type Annotations

| File | Test | Reason |
|------|------|--------|
| `bytes.toml` | `bytes_index` | Complex type annotations not yet supported |
| `bytes.toml` | `bytes_slice` | Complex type annotations not yet supported |
| `class-methods.toml` | `classmethod_descriptor` | Complex type annotations not yet supported |
| `cli-integration.toml` | `marco_polo` | Unsupported type annotation: argparse.Namespace |
| `cli-integration.toml` | `marco_polo_annotated` | Unsupported type annotation: argparse.Namespace |
| `dataclasses-field.toml` | `field_metadata` | Complex type annotations not yet supported |
| `dataclasses-field.toml` | `field_string_type` | Unsupported type annotation: string literal "Node" |
| `defaultdict.toml` | `defaultdict_missing` | Unsupported type annotation: string literal |
| `defaultdict.toml` | `defaultdict_missing_method` | Unsupported type annotation: string literal |
| `defaultdict.toml` | `defaultdict_in` | Unsupported type annotation: string literal |
| `ellipsis.toml` | `ellipsis_callable` | Unsupported type annotation: Ellipsis |
| `ellipsis.toml` | `ellipsis_tuple` | Unsupported type annotation: Ellipsis |
| `ellipsis.toml` | `ellipsis_slice` | Unsupported type annotation: Ellipsis |
| `ellipsis.toml` | `ellipsis_expand` | Unsupported type annotation: integer constant |
| `examples-marco-polo-cli.toml` | `marco_polo` | Unsupported type annotation: argparse.Namespace |
| `examples-marco-polo-cli.toml` | `marco_polo_annotated` | Unsupported type annotation: argparse.Namespace |
| `examples-mathematical.toml` | `geometry` | Unsupported type annotation: string literal "Point" |
| `examples-tooling.toml` | `ast_converters_demo` | Unsupported type annotation: integer constant |
| `examples-tooling.toml` | `lsp_demo` | Unsupported type parameter slice: string literal |
| `examples-type-hints.toml` | `type_extraction_demo` | Unsupported type annotation: Ellipsis |
| `imports-basic.toml` | `import_collections` | Unsupported type annotation: integer constant |
| `modules-itertools-functools.toml` | `itertools_permutations` | Unsupported type annotation: Ellipsis |
| `new-vs-init.toml` | `factory_vs_new` | Unsupported type annotation: string literal |
| `stdlib-misc.toml` | `uuid_uuid3` | Unsupported type annotation: uuid.UUID |
| `stdlib-misc.toml` | `uuid_uuid5` | Unsupported type annotation: uuid.UUID |
| `type-inference.toml` | `type_mapper_unsupported_function_type` | Unsupported type annotation: List |

### Unsupported Features

| File | Test | Reason |
|------|------|--------|
| `assignment.toml` | `multi_assign_same_value` | Multiple assignment targets not supported |
| `async-comprehensions.toml` | `async_dict_comp` | Complex comprehension targets not yet supported |
| `async-comprehensions.toml` | `async_comp_nested` | Nested list comprehensions not yet supported |
| `asyncio-tasks.toml` | `asyncio_queue` | get() requires 1 or 2 arguments |
| `comprehensions.toml` | `comprehension_flatten` | Nested list comprehensions not yet supported |
| `comprehensions.toml` | `dict_comprehension_from_list` | Complex comprehension targets not yet supported |
| `control-flow.toml` | `for_loop_dict_subscript_target` | Unsupported for loop target type |
| `control-flow.toml` | `nested_tuple_unpacking` | Nested tuple unpacking not supported |
| `dunder-context.toml` | `exit_suppress` | Invalid operator Is for comparison conversion |
| `dunder-reflected.toml` | `dunder_rmatmul` | Unsupported binary operator |
| `ellipsis.toml` | `ellipsis_function_placeholder` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_abstract` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_vs_pass` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_assignment` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_singleton` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_in_dict` | Unsupported constant type |
| `ellipsis.toml` | `ellipsis_default_arg` | Unsupported constant type |
| `iterators.toml` | `zip_into_iter_consumption` | Complex comprehension targets not yet supported |
| `lambdas.toml` | `lambda_two_params` | Complex comprehension targets not yet supported |
| `modules-itertools-functools.toml` | `zip_iteration` | Complex comprehension targets not yet supported |
| `nested-context-managers.toml` | `multi_with_comma` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_with_as` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_mixed_as` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_enter_order` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_exit_order` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_enter_fail` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_body_exception` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_one_suppresses` | Invalid operator Is for comparison conversion |
| `nested-context-managers.toml` | `multi_files` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_locks` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_db_file` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_same_cm` | Multiple context managers not yet supported |
| `nested-context-managers.toml` | `multi_many` | Multiple context managers not yet supported |

### Unsupported String/Method Operations

| File | Test | Reason |
|------|------|--------|
| `format-spec.toml` | `format_str` | Unknown string method: format |
| `format-spec.toml` | `format_left` | Unknown string method: format |
| `format-spec.toml` | `format_right` | Unknown string method: format |
| `format-spec.toml` | `format_center` | Unknown string method: format |
| `format-spec.toml` | `format_fill` | Unknown string method: format |
| `format-spec.toml` | `format_numeric_align` | Unknown string method: format |
| `format-spec.toml` | `format_plus` | Unknown string method: format |
| `format-spec.toml` | `format_minus` | Unknown string method: format |
| `format-spec.toml` | `format_space` | Unknown string method: format |
| `format-spec.toml` | `format_hex_alt` | Unknown string method: format |
| `format-spec.toml` | `format_bin_alt` | Unknown string method: format |
| `format-spec.toml` | `format_oct_alt` | Unknown string method: format |
| `format-spec.toml` | `format_width` | Unknown string method: format |
| `format-spec.toml` | `format_precision` | Unknown string method: format |
| `format-spec.toml` | `format_width_prec` | Unknown string method: format |
| `format-spec.toml` | `format_str_prec` | Unknown string method: format |
| `format-spec.toml` | `format_comma` | Unknown string method: format |
| `format-spec.toml` | `format_underscore` | Unknown string method: format |
| `format-spec.toml` | `format_percent` | Unknown string method: format |
| `format-spec.toml` | `format_char` | Unknown string method: format |
| `format-spec.toml` | `format_datetime` | Unknown string method: format |
| `format-spec.toml` | `format_zero_pad` | Unknown string method: format |
| `format-spec.toml` | `format_empty` | Unknown string method: format |
| `format-spec.toml` | `format_nested` | Unknown string method: format |
| `functions.toml` | `function_kwargs_method_call` | Unknown string method: format |
| `lifetimes.toml` | `format_call_returns_string` | Unknown string method: format |
| `modules-regex.toml` | `re_fullmatch` | re.fullmatch not implemented yet |
| `modules-regex.toml` | `re_split` | split() with maxsplit not supported in V1 |
| `string-methods-all.toml` | `str_strip_chars` | strip() with arguments not supported in V1 |
| `string-methods-all.toml` | `str_split_max` | split() with maxsplit not supported in V1 |
| `string-methods-all.toml` | `str_format` | Unknown string method: format |
| `string-operations.toml` | `str_format` | Unknown string method: format |

### Unsupported Module Functions

| File | Test | Reason |
|------|------|--------|
| `crypto.toml` | `base64_b32encode` | base64.b32encode requires data-encoding crate (not yet integrated) |
| `crypto.toml` | `base64_b32decode` | base64.b32decode requires data-encoding crate (not yet integrated) |
| `crypto.toml` | `hashlib_sha3_256` | hashlib.sha3_256 not implemented yet |
| `crypto.toml` | `hmac_update` | hmac.new() requires at least 2 arguments |
| `examples-tooling.toml` | `module_mapping_demo` | itertools.combinations not implemented yet |
| `imports-basic.toml` | `import_sys_modules` | sys.modules is not a recognized attribute |
| `modules-os-sys.toml` | `os_path_join` | join() requires exactly one argument |

### Unsupported Python Constructs

| File | Test | Reason |
|------|------|--------|
| `closures.toml` | `closure_basic` | Unsupported function call type: nested function call |
| `closures.toml` | `closure_nested` | Unsupported function call type: nested function call |
| `closures.toml` | `closure_multi_level` | Unsupported function call type: nested function call |
| `closures-late-binding.toml` | `late_binding_walrus` | Expression type not yet supported |
| `decorators-basic.toml` | `decorator_method` | Python variable 'self' conflicts with Rust keyword |
| `decorators-basic.toml` | `decorator_self` | Python variable 'self' conflicts with Rust keyword |
| `dunder-callable.toml` | `call_nested` | Unsupported function call type: object call |
| `metaclasses.toml` | `type_as_metaclass` | Python variable 'self' conflicts with Rust keyword |

## Summary by Error Category

| Category | Count |
|----------|-------|
| Statement type not supported (Global/Nonlocal/AsyncFor/AsyncWith/Delete/Import) | ~65 |
| Unknown string method: format | ~28 |
| PANIC errors (empty Ident, invalid Ident) | ~33 |
| Unsupported type annotations | ~26 |
| Expression type not supported (FString, Slice, Yield) | ~30 |
| Complex comprehension targets | ~6 |
| Multiple context managers | ~14 |
| Other unsupported features | ~31 |
