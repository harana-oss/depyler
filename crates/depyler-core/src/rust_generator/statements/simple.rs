//! Simple code generation

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_generator::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

use super::*;
pub(crate) fn codegen_pass_stmt() -> Result<proc_macro2::TokenStream> {
    Ok(quote! {})
}

pub(crate) fn codegen_assert_stmt(
    test: &HirExpr,
    msg: &Option<HirExpr>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let test_expr = test.to_rust_expr(ctx)?;

    if let Some(message_expr) = msg {
        let msg_tokens = message_expr.to_rust_expr(ctx)?;
        Ok(quote! { assert!(#test_expr, "{}", #msg_tokens); })
    } else {
        Ok(quote! { assert!(#test_expr); })
    }
}

pub(crate) fn codegen_expr_stmt(
    expr: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Pattern: parser.add_argument("files", nargs="+", type=Path, action="store_true", help="...")
    if let HirExpr::MethodCall {
        object,
        method,
        args,
        kwargs,
        type_params: _,
    } = expr
    {
        // ArgumentParser methods that should be ignored:
        // - add_argument() → accumulated into Args struct
        // - add_argument_group() → not needed with clap (uses struct fields)
        // - set_defaults() → not needed (use field defaults)
        // - add_mutually_exclusive_group() → use clap group attributes
        if let HirExpr::Var(var_name) = object.as_ref() {
            if let Some(subcommand_info) = ctx.argparser_tracker.get_subcommand_mut(var_name) {
                // This is a subcommand parser - route add_argument to subcommand
                if method == "add_argument" {
                    // Extract argument details (same as main parser)
                    if let Some(HirExpr::Literal(crate::hir::Literal::String(first_arg))) =
                        args.first()
                    {
                        let mut arg = crate::rust_generator::argparse_transform::ArgParserArgument::new(
                            first_arg.clone(),
                        );

                        // Check for second argument (long flag)
                        if let Some(HirExpr::Literal(crate::hir::Literal::String(second_arg))) =
                            args.get(1)
                        {
                            if second_arg.starts_with("--") {
                                arg.long = Some(second_arg.clone());
                            }
                        }

                        // Extract kwargs (same extraction logic as main parser)
                        for (kw_name, kw_value) in kwargs {
                            match kw_name.as_str() {
                                "help" => {
                                    if let HirExpr::Literal(crate::hir::Literal::String(help_val)) =
                                        kw_value
                                    {
                                        arg.help = Some(help_val.clone());
                                    }
                                }
                                "type" => {
                                    if let HirExpr::Var(type_name) = kw_value {
                                        match type_name.as_str() {
                                            "str" => arg.arg_type = Some(crate::hir::Type::String),
                                            "int" => arg.arg_type = Some(crate::hir::Type::Int),
                                            "float" => arg.arg_type = Some(crate::hir::Type::Float),
                                            "Path" => {
                                                arg.arg_type = Some(crate::hir::Type::Custom(
                                                    "PathBuf".to_string(),
                                                ))
                                            }
                                            _ => {
                                                // e.g., type=email_address → track "email_address"
                                                ctx.validator_functions.insert(type_name.clone());
                                            }
                                        }
                                    }
                                }
                                "action" => {
                                    if let HirExpr::Literal(crate::hir::Literal::String(
                                        action_val,
                                    )) = kw_value
                                    {
                                        arg.action = Some(action_val.clone());
                                    }
                                }
                                "required" => {
                                    if let HirExpr::Literal(crate::hir::Literal::Bool(req)) =
                                        kw_value
                                    {
                                        arg.required = Some(*req);
                                    }
                                }
                                _ => {}
                            }
                        }

                        subcommand_info.arguments.push(arg);
                    }
                    return Ok(quote! {});
                }
            }

            // If it's a group, resolve to the parent parser (recursively for nested groups)
            let parser_var = if ctx.argparser_tracker.get_parser(var_name).is_some() {
                var_name.clone()
            } else if let Some(parent_parser) = ctx.argparser_tracker.get_parser_for_group(var_name)
            {
                parent_parser // Already returns owned String
            } else {
                // Not a parser, group, or subcommand - fall through to normal code generation
                let expr_tokens = expr.to_rust_expr(ctx)?;
                return Ok(quote! { #expr_tokens; });
            };

            // Check if this is a parser configuration method
            if ctx.argparser_tracker.get_parser(&parser_var).is_some() {
                match method.as_str() {
                    "add_argument" => {
                        // Process add_argument to extract argument details
                        if let Some(_parser_info) =
                            ctx.argparser_tracker.get_parser_mut(&parser_var)
                        {
                            // First arg is required, second is optional (for dual short+long flags)
                            if let Some(HirExpr::Literal(crate::hir::Literal::String(first_arg))) =
                                args.first()
                            {
                                let mut arg =
                                    crate::rust_generator::argparse_transform::ArgParserArgument::new(
                                        first_arg.clone(),
                                    );

                                // Check for second argument (long flag name in dual short+long pattern)
                                if let Some(HirExpr::Literal(crate::hir::Literal::String(
                                    second_arg,
                                ))) = args.get(1)
                                {
                                    // Pattern: add_argument("-o", "--output")
                                    // First is short, second is long
                                    if second_arg.starts_with("--") {
                                        arg.long = Some(second_arg.clone());
                                    }
                                }

                                for (kw_name, kw_value) in kwargs {
                                    match kw_name.as_str() {
                                        "nargs" => match kw_value {
                                            HirExpr::Literal(crate::hir::Literal::String(
                                                nargs_val,
                                            )) => {
                                                arg.nargs = Some(nargs_val.clone());
                                            }
                                            HirExpr::Literal(crate::hir::Literal::Int(n)) => {
                                                arg.nargs = Some(n.to_string());
                                            }
                                            _ => {}
                                        },
                                        "type" => {
                                            if let HirExpr::Var(type_name) = kw_value {
                                                match type_name.as_str() {
                                                    "str" => {
                                                        arg.arg_type =
                                                            Some(crate::hir::Type::String)
                                                    }
                                                    "int" => {
                                                        arg.arg_type = Some(crate::hir::Type::Int)
                                                    }
                                                    "float" => {
                                                        arg.arg_type = Some(crate::hir::Type::Float)
                                                    }
                                                    "Path" => {
                                                        // Path needs to map to PathBuf
                                                        arg.arg_type =
                                                            Some(crate::hir::Type::Custom(
                                                                "PathBuf".to_string(),
                                                            ));
                                                    }
                                                    _ => {
                                                        // e.g., type=email_address → track "email_address"
                                                        ctx.validator_functions
                                                            .insert(type_name.clone());
                                                    }
                                                }
                                            }
                                        }
                                        "action" => {
                                            if let HirExpr::Literal(crate::hir::Literal::String(
                                                action_val,
                                            )) = kw_value
                                            {
                                                arg.action = Some(action_val.clone());
                                            }
                                        }
                                        "help" => {
                                            if let HirExpr::Literal(crate::hir::Literal::String(
                                                help_val,
                                            )) = kw_value
                                            {
                                                arg.help = Some(help_val.clone());
                                            }
                                        }
                                        "default" => {
                                            arg.default = Some(kw_value.clone());
                                        }
                                        "required" => {
                                            if let HirExpr::Literal(crate::hir::Literal::Bool(
                                                req,
                                            )) = kw_value
                                            {
                                                arg.required = Some(*req);
                                            }
                                        }
                                        "dest" => {
                                            if let HirExpr::Literal(crate::hir::Literal::String(
                                                dest_name,
                                            )) = kw_value
                                            {
                                                arg.dest = Some(dest_name.clone());
                                            }
                                        }
                                        "metavar" => {
                                            if let HirExpr::Literal(crate::hir::Literal::String(
                                                metavar_name,
                                            )) = kw_value
                                            {
                                                arg.metavar = Some(metavar_name.clone());
                                            }
                                        }
                                        "choices" => {
                                            if let HirExpr::List(items) = kw_value {
                                                let mut choices = Vec::new();
                                                for item in items {
                                                    if let HirExpr::Literal(
                                                        crate::hir::Literal::String(s),
                                                    ) = item
                                                    {
                                                        choices.push(s.clone());
                                                    }
                                                }
                                                if !choices.is_empty() {
                                                    arg.choices = Some(choices);
                                                }
                                            }
                                        }
                                        "const" => {
                                            arg.const_value = Some(kw_value.clone());
                                        }
                                        _ => {
                                            // Ignore other kwargs (e.g., prog, formatter_class)
                                        }
                                    }
                                }

                                _parser_info.add_argument(arg);
                            }

                            // Skip generating this statement - arguments will be in Args struct
                            return Ok(quote! {});
                        }
                    }
                    "add_argument_group" | "add_mutually_exclusive_group" | "set_defaults" => {
                        // With clap derive, argument groups are handled by struct field organization
                        // Mutually exclusive groups use #[group] attributes
                        // Defaults use field default values
                        return Ok(quote! {});
                    }
                    _ => {
                        // Other parser methods - fall through to normal code generation
                    }
                }
            }
        }
    }

    let expr_tokens = expr.to_rust_expr(ctx)?;
    Ok(quote! { #expr_tokens; })
}

pub(crate) fn codegen_delete_stmt(
    targets: &[AssignTarget],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let delete_stmts: Vec<proc_macro2::TokenStream> = targets
        .iter()
        .map(|target| match target {
            AssignTarget::Symbol(name) => {
                let ident = safe_ident(name);
                Ok(quote! { drop(#ident); })
            }
            AssignTarget::Index { base, index } => {
                let base_expr = base.to_rust_expr(ctx)?;
                let index_expr = index.to_rust_expr(ctx)?;
                Ok(quote! { #base_expr.remove(&#index_expr); })
            }
            AssignTarget::Attribute { value, attr } => {
                let value_expr = value.to_rust_expr(ctx)?;
                let attr_ident = safe_ident(attr);
                Ok(quote! { drop(#value_expr.#attr_ident); })
            }
            _ => Ok(quote! {}),
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(quote! { #(#delete_stmts)* })
}

pub(crate) fn codegen_import_stmt() -> Result<proc_macro2::TokenStream> {
    // Import statement inside a function is a no-op in Rust.
    // The module imports are handled at the module level.
    Ok(quote! {})
}

pub(crate) fn codegen_import_from_stmt() -> Result<proc_macro2::TokenStream> {
    // Import-from statement inside a function is a no-op in Rust.
    // The module imports are handled at the module level.
    Ok(quote! {})
}

pub(crate) fn codegen_async_for_stmt(
    target: &AssignTarget,
    iter: &HirExpr,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let iter_expr = iter.to_rust_expr(ctx)?;
    let target_pattern = convert_assign_target_to_pattern(target)?;

    let body_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        while let Some(#target_pattern) = #iter_expr.next().await {
            #(#body_stmts)*
        }
    })
}
