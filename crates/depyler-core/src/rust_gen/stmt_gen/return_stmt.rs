//! Return Stmt code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

use super::*;
pub(crate) fn codegen_return_stmt(
    expr: &Option<HirExpr>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    if let Some(e) = expr {
        // Handle tuple returns with element-level Optional wrapping
        // When the return type is Tuple with Optional elements, wrap non-None values in Some()
        if let HirExpr::Tuple(elems) = e {
            if let Some(Type::Tuple(elem_types)) = &ctx.effective_return_type {
                if elems.len() == elem_types.len()
                    && elem_types.iter().any(|t| matches!(t, Type::Optional(_)))
                {
                    let elem_types = elem_types.clone();
                    return codegen_tuple_return_with_optional(elems, &elem_types, ctx);
                }
            }
        }

        let mut expr_tokens = e.to_rust_expr(ctx)?;

        // When function returns a reference, wrap the expression in &
        // This handles cases like: return state.home_players → return &state.home_players
        // BUT: Skip wrapping for expressions that create owned values (constructors, literals, etc.)
        // These already return owned values, so adding & would create a type mismatch.
        // NOTE: IfExpr handles its own & wrapping for each branch, so skip it here
        if ctx.returns_reference
            && !matches!(e, HirExpr::IfExpr { .. })
            && !expr_creates_owned_value(e)
        {
            expr_tokens = parse_quote! { &#expr_tokens };
        }

        if let Some(return_type) = &ctx.current_return_type {
            // Unwrap Optional to get the underlying type
            let target_type = match return_type {
                Type::Optional(inner) => inner.as_ref(),
                other => other,
            };

            if needs_type_conversion(target_type, e) {
                expr_tokens = apply_type_conversion(expr_tokens, target_type);
            }
        }

        // Check if return type is Optional and wrap value in Some()
        let is_optional_return =
            matches!(ctx.current_return_type.as_ref(), Some(Type::Optional(_)));

        // Original logic: Unwrap Option-typed variables when returning from non-Optional function
        // Problem: Can't distinguish between:
        //   1. result = d.get(key)  # Option<T> - needs unwrap
        //   2. result = 0           # i32 - breaks with unwrap
        // NOTE: Re-enable unwrap_or optimization when HIR has type tracking ()

        // if !is_optional_return {
        //     if let HirExpr::Var(var_name) = e {
        //         let is_primitive_return = matches!(
        //             ctx.current_return_type.as_ref(),
        //             Some(Type::Int | Type::Float | Type::Bool | Type::String)
        //         );
        //         if ctx.is_final_statement && var_name == "result" && is_primitive_return {
        //             expr_tokens = parse_quote! { #expr_tokens.unwrap() };
        //         }
        //     }
        // }

        // Check if the expression is None literal
        let is_none_literal = matches!(e, HirExpr::Literal(Literal::None));

        // Unwrap Option-typed variables when returning from a non-Optional function
        // This handles the pattern: if x is None: return default; return x
        // After the None check, x must be Some, so we unwrap
        if !is_optional_return && !is_none_literal {
            let expr_is_opt = expr_is_optional(e, ctx);
            if expr_is_opt {
                // Check if this is a reference parameter - need different unwrap pattern
                let is_ref_param = if let HirExpr::Var(var_name) = e {
                    ctx.current_func_ref_params.contains(var_name)
                } else {
                    false
                };
                if is_ref_param {
                    // For &Option<T> parameters, use as_ref().unwrap().clone()
                    expr_tokens = parse_quote! { #expr_tokens.as_ref().unwrap().clone() };
                } else {
                    expr_tokens = parse_quote! { #expr_tokens.unwrap() };
                }
            }
        }

        // Always use explicit return keyword for clarity and Python-like behavior
        let use_return_keyword = true;

        // Must check this BEFORE is_optional_return to avoid false positive
        // Python `-> None` maps to Rust `()`, not `Option<T>`
        let is_void_return = matches!(ctx.current_return_type.as_ref(), Some(Type::None));

        if is_void_return {
            // Void functions (Python -> None): no return value
            if use_return_keyword {
                Ok(quote! { return; })
            } else {
                Ok(quote! { () })
            }
        } else if is_optional_return && !is_none_literal {
            // Check if expression is already Optional to avoid double-wrapping
            let expr_already_optional = expr_is_optional(e, ctx);
            if expr_already_optional {
                if use_return_keyword {
                    if is_block_expr(&expr_tokens) {
                        Ok(quote! { return #expr_tokens })
                    } else {
                        Ok(quote! { return #expr_tokens; })
                    }
                } else {
                    Ok(quote! { #expr_tokens })
                }
            } else {
                if use_return_keyword {
                    if is_block_expr(&expr_tokens) {
                        Ok(quote! { return Some(#expr_tokens) })
                    } else {
                        Ok(quote! { return Some(#expr_tokens); })
                    }
                } else {
                    Ok(quote! { Some(#expr_tokens) })
                }
            }
        } else if is_optional_return && is_none_literal {
            if use_return_keyword {
                Ok(quote! { return None; })
            } else {
                Ok(quote! { None })
            }
        } else if use_return_keyword {
            if is_block_expr(&expr_tokens) {
                Ok(quote! { return #expr_tokens })
            } else {
                Ok(quote! { return #expr_tokens; })
            }
        } else {
            Ok(quote! { #expr_tokens })
        }
    } else {
        // Always use explicit return keyword
        let use_return_keyword = true;
        if use_return_keyword {
            Ok(quote! { return; })
        } else {
            Ok(quote! {})
        }
    }
}

pub(crate) fn expr_creates_owned_value(expr: &HirExpr) -> bool {
    match expr {
        // Constructor calls: ClassName(...) or Type::new(...)
        HirExpr::Call { func, .. } => {
            // Check if func is a type name (starts with uppercase) - likely a constructor
            if let Some(first_char) = func.chars().next() {
                if first_char.is_uppercase() {
                    return true;
                }
            }
            // Common owned-value-creating functions
            matches!(
                func.as_str(),
                "list" | "dict" | "set" | "str" | "Vec" | "HashMap" | "HashSet" | "String"
            )
        }
        // Method calls like Type::new(...) or Type::from(...)
        HirExpr::MethodCall { method, .. } => {
            matches!(method.as_str(), "new" | "from" | "default" | "clone")
        }
        // List, Dict, Set literals create owned values
        HirExpr::List(_) | HirExpr::Dict(_) | HirExpr::Set(_) | HirExpr::Tuple(_) => true,
        // Literals (except None) create owned values
        HirExpr::Literal(lit) => !matches!(lit, Literal::None),
        // Binary operations create new values
        HirExpr::Binary { .. } => true,
        // Unary operations create new values
        HirExpr::Unary { .. } => true,
        // Format strings create owned strings
        HirExpr::FString { .. } => true,
        // Comprehensions create owned collections
        HirExpr::ListComp { .. } | HirExpr::SetComp { .. } | HirExpr::DictComp { .. } => true,
        _ => false,
    }
}

/// Generate a return statement for a tuple where some elements are Optional.
/// Wraps non-None, non-Optional values in Some() for the Optional element positions.
fn codegen_tuple_return_with_optional(
    elems: &[HirExpr],
    elem_types: &[Type],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let mut elem_tokens: Vec<proc_macro2::TokenStream> = Vec::new();
    for (elem, ty) in elems.iter().zip(elem_types.iter()) {
        let tokens = elem.to_rust_expr(ctx)?;
        match ty {
            Type::Optional(_) => {
                if matches!(elem, HirExpr::Literal(Literal::None)) {
                    elem_tokens.push(quote! { None });
                } else if expr_is_optional(elem, ctx) {
                    elem_tokens.push(quote! { #tokens });
                } else {
                    elem_tokens.push(quote! { Some(#tokens) });
                }
            }
            _ => {
                elem_tokens.push(quote! { #tokens });
            }
        }
    }
    Ok(quote! { return (#(#elem_tokens),*); })
}

pub(crate) fn is_block_expr(expr: &syn::Expr) -> bool {
    matches!(
        expr,
        syn::Expr::Block(_)
            | syn::Expr::If(_)
            | syn::Expr::Match(_)
            | syn::Expr::Loop(_)
            | syn::Expr::ForLoop(_)
            | syn::Expr::While(_)
            | syn::Expr::Unsafe(_)
            | syn::Expr::Async(_)
    )
}
