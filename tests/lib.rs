pub mod test_helpers;

#[cfg(test)]
mod test_annotation_aware_type_mapper_coverage;
#[cfg(test)]
mod test_argparse_type_inference;
#[cfg(test)]
mod test_argument_type_error;
#[cfg(test)]
mod test_ast_bridge_boolean_logics;
#[cfg(test)]
mod test_ast_bridge_comparisons;
#[cfg(test)]
mod test_ast_bridge_match_arms;
#[cfg(test)]
mod test_ast_bridge_return_values;
#[cfg(test)]
mod test_ast_bridge_type_inferences;
#[cfg(test)]
mod test_bug_regressions;
#[cfg(test)]
mod test_cli_flags_coverage;
#[cfg(test)]
mod test_clis;
#[cfg(test)]
mod test_codegen_coverage;
#[cfg(test)]
mod test_context_coverage;
#[cfg(test)]
mod test_convert_stmts;
#[cfg(test)]
mod test_converters_propertys;
#[cfg(test)]
mod test_coverage_analysis;
#[cfg(test)]
mod test_dataflow_type_casting;
#[cfg(test)]
mod test_debug_coverage;
#[cfg(test)]
mod test_default_parameters;
#[cfg(test)]
mod test_direct_rules_coverage;
#[cfg(test)]
mod test_direct_rules_simples;
#[cfg(test)]
mod test_edge_case_coverage;
#[cfg(test)]
mod test_error_coverage;
#[cfg(test)]
mod test_error_gen_coverage;
#[cfg(test)]
mod test_error_handlings;
#[cfg(test)]
mod test_error_path_coverage;
#[cfg(test)]
mod test_example_validation;
#[cfg(test)]
mod test_expr_gen_coverage;
#[cfg(test)]
mod test_expr_gen_extended_coverage;
#[cfg(test)]
mod test_expr_gen_methods_coverage;
#[cfg(test)]
mod test_expr_gen_untested_builtins;
#[cfg(test)]
mod test_expr_to_rusts;
#[cfg(test)]
mod test_formatting;
#[cfg(test)]
mod test_functionals;
#[cfg(test)]
mod test_fuzzings;
#[cfg(test)]
mod test_generate_rust_files;
#[cfg(test)]
mod test_generator_compilations;
#[cfg(test)]
mod test_generator_yield_analysis_coverage;
#[cfg(test)]
mod test_generic_inference;
#[cfg(test)]
mod test_hir_kwargs;
#[cfg(test)]
mod test_ide_coverage;
#[cfg(test)]
mod test_import_gen_coverage;
#[cfg(test)]
mod test_inlining_coverage;
#[cfg(test)]
mod test_lambda_integration;
#[cfg(test)]
mod test_lifetime_analysis_coverage;
#[cfg(test)]
mod test_lifetime_analysis_integration;
#[cfg(test)]
mod test_lsp_propertys;
#[cfg(test)]
mod test_marco_polo_integration;
#[cfg(test)]
mod test_mega;
#[cfg(test)]
mod test_method_ownership;
#[cfg(test)]
mod test_migration_suggestions_propertys;
#[cfg(test)]
mod test_module_mapper_propertys;
#[cfg(test)]
mod test_mutationing;
#[cfg(test)]
mod test_operators;
#[cfg(test)]
mod test_option_type_mismatch;
#[cfg(test)]
mod test_ownership_patterns;
#[cfg(test)]
mod test_phase2;
#[cfg(test)]
mod test_phase3;
#[cfg(test)]
mod test_process_module_imports;
#[cfg(test)]
mod test_property;
#[cfg(test)]
mod test_property_based_generation;
#[cfg(test)]
mod test_propertys;
#[cfg(test)]
mod test_propertys_ast_roundtrip;
#[cfg(test)]
mod test_propertys_memory_safety;
#[cfg(test)]
mod test_propertys_type_inference;
#[cfg(test)]
mod test_quality_assurance_automation;
#[cfg(test)]
mod test_quality_gatess;
#[cfg(test)]
mod test_rust_gen_coverage;
#[cfg(test)]
mod test_rust_type_to_syns;
#[cfg(test)]
mod test_semantic_equivalence;
#[cfg(test)]
mod test_show_types;
#[cfg(test)]
mod test_simplified_hir_coverage;
#[cfg(test)]
mod test_specialized_coverageing;
#[cfg(test)]
mod test_stmt_gen_assign_coverage;
#[cfg(test)]
mod test_stmt_gen_assign_index_coverage;
#[cfg(test)]
mod test_stmt_gen_assign_symbol_coverage;
#[cfg(test)]
mod test_stmt_gen_coverage;
#[cfg(test)]
mod test_stmt_gen_extended_coverage;
#[cfg(test)]
mod test_stmt_gen_if_coverage;
#[cfg(test)]
mod test_stmt_gen_raise_coverage;
#[cfg(test)]
mod test_stmt_gen_return_coverage;
#[cfg(test)]
mod test_stmt_gen_try_coverage;
#[cfg(test)]
mod test_subcommand_field_access;
#[cfg(test)]
mod test_toml_runner;
#[cfg(test)]
mod test_transpilations;
#[cfg(test)]
mod test_type_gen_coverage;
#[cfg(test)]
mod test_type_mapper_coverage;
#[cfg(test)]
mod test_type_mapper_extended_coverage;
#[cfg(test)]
mod test_type_mapper_propertys;
#[cfg(test)]
mod test_type_system;
#[cfg(test)]
mod test_uninitialized_declarations;
#[cfg(test)]
mod test_union_enum_gen_coverage;
#[cfg(test)]
mod test_union_types;
#[cfg(test)]
mod test_unnecessary_casts;
#[cfg(test)]
mod test_unused_loop_vars;
#[cfg(test)]
mod test_v3_17_coverages;
#[cfg(test)]
mod test_validator_return_type;
