//! Context Mgr code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn codegen_with_stmt(
    context: &HirExpr,
    target: &Option<String>,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use crate::hir::HirExpr;
    use crate::hir::HirStmt;

    // Pattern detection: with open(path, 'r') as f: return f.read()
    // Optimize to: fs::read_to_string(path).unwrap_or_default()
    if let HirExpr::Call { func, args, .. } = context {
        if func.as_str() == "open" && !args.is_empty() {
            if let Some(var_name) = target {
                // Check if body is a single return statement with f.read()
                if body.len() == 1 {
                    if let HirStmt::Return(Some(return_expr)) = &body[0] {
                        if let HirExpr::MethodCall {
                            object,
                            method,
                            args: method_args,
                            ..
                        } = return_expr
                        {
                            if let HirExpr::Var(obj_name) = &**object {
                                if obj_name == var_name
                                    && method == "read"
                                    && method_args.is_empty()
                                {
                                    // Pattern matched! Optimize to fs::read_to_string
                                    let path_expr = args[0].to_rust_expr(ctx)?;
                                    return Ok(quote! {
                                        return std::fs::read_to_string(#path_expr).unwrap_or_default();
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Default non-optimized path
    // Convert context expression
    let context_expr = context.to_rust_expr(ctx)?;

    // in with blocks get the explicit 'return' keyword (not treated as final statement)
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    // Convert body statements
    let body_stmts: Vec<_> = body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<_>>()?;

    // Restore is_final_statement flag
    ctx.is_final_statement = saved_is_final;

    // open() returns std::fs::File which doesn't have __enter__() method
    // For File objects, bind directly; for custom context managers, call __enter__()
    let is_file_open = matches!(
        context,
        HirExpr::Call { func, .. } if func.as_str() == "open"
    );

    // Generate code that calls __enter__() for custom context managers
    // or binds File directly for open() calls
    // Note: __exit__() is not yet called (Drop trait implementation pending)
    if let Some(var_name) = target {
        let var_ident = safe_ident(var_name);
        ctx.declare_var(var_name);

        if is_file_open {
            // Files are often mutated; bind as mutable to match previous codegen expectations
            Ok(quote! {
                let mut #var_ident = #context_expr;
                #(#body_stmts)*
            })
        } else {
            // For custom context managers, call __enter__()
            Ok(quote! {
                let _context = #context_expr;
                let mut #var_ident = _context.__enter__();
                #(#body_stmts)*
            })
        }
    } else {
        Ok(quote! {
            let _context = #context_expr;
            #(#body_stmts)*
        })
    }
}

pub(crate) fn codegen_async_with_stmt(
    context: &HirExpr,
    target: &Option<Symbol>,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let context_expr = context.to_rust_expr(ctx)?;

    let body_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    if let Some(var_name) = target {
        let var_ident = safe_ident(var_name);
        Ok(quote! {
            {
                let #var_ident = #context_expr;
                #(#body_stmts)*
            }
        })
    } else {
        Ok(quote! {
            {
                let _ctx = #context_expr;
                #(#body_stmts)*
            }
        })
    }
}

