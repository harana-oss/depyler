//! Handle Python's min() and max() builtin functions
//!
//! Python: max(a, b) → Rust: f64::max(a, b) for floats, std::cmp::max(a, b) for integers
//! Python: max(a, b, c) → Rust: f64::max(f64::max(a, b), c) for floats
//! Python: max([1, 2, 3]) → Rust: *list.iter().max().unwrap()
//! Python: min() follows the same patterns

use crate::hir::{HirExpr, Type};
use crate::rust_gen::context::{CodeGenContext, ToRustExpr};
use anyhow::Result;
use syn::parse_quote;

pub fn handle_max(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    if args.is_empty() {
        anyhow::bail!("max expected at least 1 argument, got 0");
    }

    if args.len() == 1 {
        // max(iterable) → iterable.iter().max().unwrap() or use fold for floats
        return handle_max_iterable(&args[0], ctx);
    }

    if args.len() == 2 {
        // max(a, b) → std::cmp::max(a, b) or a.max(b) for floats
        return handle_max_two(args, ctx);
    }

    // max(a, b, c, ...) → chain std::cmp::max or .max() calls
    handle_max_multiple(args, ctx)
}

pub fn handle_min(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    if args.is_empty() {
        anyhow::bail!("min expected at least 1 argument, got 0");
    }

    if args.len() == 1 {
        // min(iterable) → iterable.iter().min().unwrap() or use fold for floats
        return handle_min_iterable(&args[0], ctx);
    }

    if args.len() == 2 {
        // min(a, b) → std::cmp::min(a, b) or a.min(b) for floats
        return handle_min_two(args, ctx);
    }

    // min(a, b, c, ...) → chain std::cmp::min or .min() calls
    handle_min_multiple(args, ctx)
}

fn handle_max_two(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    let arg1_expr = args[0].to_rust_expr(ctx)?;
    let arg2_expr = args[1].to_rust_expr(ctx)?;

    // Check if arguments are floats (which don't implement Ord)
    let arg1_is_float = ctx.is_expr_float_type(&args[0]);
    let arg2_is_float = ctx.is_expr_float_type(&args[1]);
    let is_float = arg1_is_float || arg2_is_float;

    if is_float {
        // Use f64::max function syntax to avoid operator precedence issues
        // Cast non-float arguments to f64 for type compatibility
        let arg1: syn::Expr = if arg1_is_float {
            arg1_expr
        } else {
            parse_quote! { (#arg1_expr) as f64 }
        };
        let arg2: syn::Expr = if arg2_is_float {
            arg2_expr
        } else {
            parse_quote! { (#arg2_expr) as f64 }
        };
        Ok(parse_quote! { f64::max(#arg1, #arg2) })
    } else {
        // Use std::cmp::max for types implementing Ord
        Ok(parse_quote! { std::cmp::max(#arg1_expr, #arg2_expr) })
    }
}

fn handle_min_two(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    let arg1_expr = args[0].to_rust_expr(ctx)?;
    let arg2_expr = args[1].to_rust_expr(ctx)?;

    // Check if arguments are floats (which don't implement Ord)
    let arg1_is_float = ctx.is_expr_float_type(&args[0]);
    let arg2_is_float = ctx.is_expr_float_type(&args[1]);
    let is_float = arg1_is_float || arg2_is_float;

    if is_float {
        // Use f64::min function syntax to avoid operator precedence issues
        // Cast non-float arguments to f64 for type compatibility
        let arg1: syn::Expr = if arg1_is_float {
            arg1_expr
        } else {
            parse_quote! { (#arg1_expr) as f64 }
        };
        let arg2: syn::Expr = if arg2_is_float {
            arg2_expr
        } else {
            parse_quote! { (#arg2_expr) as f64 }
        };
        Ok(parse_quote! { f64::min(#arg1, #arg2) })
    } else {
        // Use std::cmp::min for types implementing Ord
        Ok(parse_quote! { std::cmp::min(#arg1_expr, #arg2_expr) })
    }
}

fn handle_max_multiple(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    // Check if any argument is a float
    let is_float = args.iter().any(|arg| ctx.is_expr_float_type(arg));

    if is_float {
        // Chain f64::max calls: f64::max(f64::max(a, b), c)
        // Cast non-float arguments to f64
        let first_is_float = ctx.is_expr_float_type(&args[0]);
        let first_expr = args[0].to_rust_expr(ctx)?;
        let mut result: syn::Expr = if first_is_float {
            first_expr
        } else {
            parse_quote! { (#first_expr) as f64 }
        };

        for arg in &args[1..] {
            let arg_is_float = ctx.is_expr_float_type(arg);
            let arg_expr = arg.to_rust_expr(ctx)?;
            let cast_arg: syn::Expr = if arg_is_float {
                arg_expr
            } else {
                parse_quote! { (#arg_expr) as f64 }
            };
            result = parse_quote! { f64::max(#result, #cast_arg) };
        }
        Ok(result)
    } else {
        // Chain std::cmp::max calls: std::cmp::max(std::cmp::max(a, b), c)
        let mut result = args[0].to_rust_expr(ctx)?;
        for arg in &args[1..] {
            let arg_expr = arg.to_rust_expr(ctx)?;
            result = parse_quote! { std::cmp::max(#result, #arg_expr) };
        }
        Ok(result)
    }
}

fn handle_min_multiple(args: &[HirExpr], ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    // Check if any argument is a float
    let is_float = args.iter().any(|arg| ctx.is_expr_float_type(arg));

    if is_float {
        // Chain f64::min calls: f64::min(f64::min(a, b), c)
        // Cast non-float arguments to f64
        let first_is_float = ctx.is_expr_float_type(&args[0]);
        let first_expr = args[0].to_rust_expr(ctx)?;
        let mut result: syn::Expr = if first_is_float {
            first_expr
        } else {
            parse_quote! { (#first_expr) as f64 }
        };

        for arg in &args[1..] {
            let arg_is_float = ctx.is_expr_float_type(arg);
            let arg_expr = arg.to_rust_expr(ctx)?;
            let cast_arg: syn::Expr = if arg_is_float {
                arg_expr
            } else {
                parse_quote! { (#arg_expr) as f64 }
            };
            result = parse_quote! { f64::min(#result, #cast_arg) };
        }
        Ok(result)
    } else {
        // Chain std::cmp::min calls: std::cmp::min(std::cmp::min(a, b), c)
        let mut result = args[0].to_rust_expr(ctx)?;
        for arg in &args[1..] {
            let arg_expr = arg.to_rust_expr(ctx)?;
            result = parse_quote! { std::cmp::min(#result, #arg_expr) };
        }
        Ok(result)
    }
}

fn handle_max_iterable(arg: &HirExpr, ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    let iter_expr = arg.to_rust_expr(ctx)?;

    // Check if we're dealing with floats (which don't implement Ord)
    let is_float = match arg {
        HirExpr::Var(var_name) => {
            if let Some(Type::List(element_type)) = ctx.var_types.get(var_name) {
                matches!(element_type.as_ref(), Type::Float)
            } else {
                false
            }
        }
        _ => false,
    };

    if is_float {
        // For float lists, use fold with f64::NEG_INFINITY
        Ok(parse_quote! {
            #iter_expr.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        })
    } else {
        // For other types, use standard .max()
        Ok(parse_quote! { *#iter_expr.iter().max().unwrap() })
    }
}

fn handle_min_iterable(arg: &HirExpr, ctx: &mut CodeGenContext) -> Result<syn::Expr> {
    let iter_expr = arg.to_rust_expr(ctx)?;

    // Check if we're dealing with floats (which don't implement Ord)
    let is_float = match arg {
        HirExpr::Var(var_name) => {
            if let Some(Type::List(element_type)) = ctx.var_types.get(var_name) {
                matches!(element_type.as_ref(), Type::Float)
            } else {
                false
            }
        }
        _ => false,
    };

    if is_float {
        // For float lists, use fold with f64::INFINITY
        Ok(parse_quote! {
            #iter_expr.iter().fold(f64::INFINITY, |a, &b| a.min(b))
        })
    } else {
        // For other types, use standard .min()
        Ok(parse_quote! { *#iter_expr.iter().min().unwrap() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Literal;

    fn create_test_context() -> CodeGenContext<'static> {
        static TYPE_MAPPER: std::sync::OnceLock<crate::type_mapper::TypeMapper> = std::sync::OnceLock::new();
        let type_mapper = TYPE_MAPPER.get_or_init(crate::type_mapper::TypeMapper::default);

        CodeGenContext {
            type_mapper,
            annotation_aware_mapper: crate::annotation_aware_type_mapper::AnnotationAwareTypeMapper::with_base_mapper(
                type_mapper.clone(),
            ),
            string_optimizer: crate::string_optimization::StringOptimizer::new(),
            union_enum_generator: crate::union_enum_gen::UnionEnumGenerator::new(),
            generated_enums: Vec::new(),
            needs_hashmap: false,
            needs_hashset: false,
            needs_vecdeque: false,
            needs_fnv_hashmap: false,
            needs_ahash_hashmap: false,
            needs_arc: false,
            needs_rc: false,
            needs_cow: false,
            needs_smallvec: false,
            needs_rand: false,
            needs_small_rng: false,
            needs_slice_random: false,
            needs_serde_json: false,
            needs_regex: false,
            needs_chrono: false,
            needs_clap: false,
            needs_csv: false,
            needs_rust_decimal: false,
            needs_num_rational: false,
            needs_base64: false,
            needs_md5: false,
            needs_sha2: false,
            needs_sha3: false,
            needs_blake2: false,
            needs_hex: false,
            needs_uuid: false,
            needs_hmac: false,
            needs_crc32: false,
            needs_url_encoding: false,
            needs_lazy_static: false,
            declared_vars: vec![std::collections::HashSet::new()],
            current_function_can_fail: false,
            current_return_type: None,
            module_mapper: crate::module_mapper::ModuleMapper::new(),
            imported_modules: std::collections::HashMap::new(),
            imported_items: std::collections::HashMap::new(),
            mutable_vars: std::collections::HashSet::new(),
            needs_zerodivisionerror: false,
            needs_indexerror: false,
            needs_valueerror: false,
            needs_argumenttypeerror: false,
            in_generator: false,
            is_classmethod: false,
            generator_state_vars: std::collections::HashSet::new(),
            var_types: std::collections::HashMap::new(),
            class_names: std::collections::HashSet::new(),
            enum_names: std::collections::HashSet::new(),
            class_field_types: std::collections::HashMap::new(),
            mutating_methods: std::collections::HashMap::new(),
            function_return_types: std::collections::HashMap::new(),
            function_param_borrows: std::collections::HashMap::new(),
            function_param_muts: std::collections::HashMap::new(),
            tuple_iter_vars: std::collections::HashSet::new(),
            is_final_statement: false,
            result_bool_functions: std::collections::HashSet::new(),
            result_returning_functions: std::collections::HashSet::new(),
            current_error_type: None,
            exception_scopes: Vec::new(),
            argparser_tracker: crate::rust_gen::ArgParserTracker::new(),
            generated_args_struct: None,
            generated_commands_enum: None,
            current_subcommand_fields: None,
            validator_functions: std::collections::HashSet::new(),
            stdlib_mappings: crate::stdlib_mappings::StdlibMappings::new(),
            current_func_mut_ref_params: std::collections::HashSet::new(),
            current_func_ref_params: std::collections::HashSet::new(),
            shadowed_ref_params: std::collections::HashSet::new(),
            function_param_names: std::collections::HashMap::new(),
            function_param_types: std::collections::HashMap::new(),
            var_usage_counts: std::collections::HashMap::new(),
            var_usage_current: std::collections::HashMap::new(),
            optional_vars: std::collections::HashSet::new(),
            lazy_static_constants: std::collections::HashSet::new(),
            is_assignment_target: false,
            prevent_clone: false,
            returns_reference: false,
            borrowable_vars: std::collections::HashSet::new(),
            generate_borrow: false,
            clone_already_applied: false,
        }
    }

    #[test]
    fn test_max_mixed_int_float_casts_int_to_f64() {
        let mut ctx = create_test_context();
        // max(0, 1.5) should cast the integer 0 to f64
        let args = vec![
            HirExpr::Literal(Literal::Int(0)),
            HirExpr::Literal(Literal::Float(1.5)),
        ];
        let result = handle_max(&args, &mut ctx).unwrap();
        let code = quote::quote!(#result).to_string();
        assert!(code.contains("f64 :: max"), "Expected f64::max, got: {}", code);
        assert!(code.contains("as f64"), "Expected cast to f64, got: {}", code);
    }

    #[test]
    fn test_min_mixed_int_float_casts_int_to_f64() {
        let mut ctx = create_test_context();
        // min(0, 1.5) should cast the integer 0 to f64
        let args = vec![
            HirExpr::Literal(Literal::Int(0)),
            HirExpr::Literal(Literal::Float(1.5)),
        ];
        let result = handle_min(&args, &mut ctx).unwrap();
        let code = quote::quote!(#result).to_string();
        assert!(code.contains("f64 :: min"), "Expected f64::min, got: {}", code);
        assert!(code.contains("as f64"), "Expected cast to f64, got: {}", code);
    }

    #[test]
    fn test_max_two_floats_no_extra_cast() {
        let mut ctx = create_test_context();
        // max(1.0, 2.0) should not need extra casts
        let args = vec![
            HirExpr::Literal(Literal::Float(1.0)),
            HirExpr::Literal(Literal::Float(2.0)),
        ];
        let result = handle_max(&args, &mut ctx).unwrap();
        let code = quote::quote!(#result).to_string();
        assert!(code.contains("f64 :: max"), "Expected f64::max, got: {}", code);
        // Should have exactly two occurrences (the literals), no extra casts
        let cast_count = code.matches("as f64").count();
        assert_eq!(cast_count, 0, "Expected no casts for two floats, got: {}", code);
    }

    #[test]
    fn test_min_two_integers_uses_cmp() {
        let mut ctx = create_test_context();
        // min(1, 2) should use std::cmp::min
        let args = vec![
            HirExpr::Literal(Literal::Int(1)),
            HirExpr::Literal(Literal::Int(2)),
        ];
        let result = handle_min(&args, &mut ctx).unwrap();
        let code = quote::quote!(#result).to_string();
        assert!(code.contains("std :: cmp :: min"), "Expected std::cmp::min, got: {}", code);
    }

    #[test]
    fn test_max_three_mixed_types_casts_all_ints() {
        let mut ctx = create_test_context();
        // max(0, 1.5, 2) should cast both integers to f64
        let args = vec![
            HirExpr::Literal(Literal::Int(0)),
            HirExpr::Literal(Literal::Float(1.5)),
            HirExpr::Literal(Literal::Int(2)),
        ];
        let result = handle_max(&args, &mut ctx).unwrap();
        let code = quote::quote!(#result).to_string();
        assert!(code.contains("f64 :: max"), "Expected f64::max, got: {}", code);
        // Should have two casts (for the two integers)
        let cast_count = code.matches("as f64").count();
        assert_eq!(cast_count, 2, "Expected two casts for mixed types, got: {}", code);
    }
}
