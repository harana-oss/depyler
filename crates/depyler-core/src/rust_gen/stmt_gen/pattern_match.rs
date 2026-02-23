//! Pattern Match code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn codegen_match_stmt(
    subject: &HirExpr,
    cases: &[MatchCase],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Check if any case has a mapping pattern - requires special handling
    let has_mapping = cases
        .iter()
        .any(|case| matches!(case.pattern, HirPattern::Mapping { .. }));

    if has_mapping {
        return codegen_match_stmt_with_mapping(subject, cases, ctx);
    }

    let subject_expr = subject.to_rust_expr(ctx)?;

    let arms: Vec<proc_macro2::TokenStream> = cases
        .iter()
        .map(|case| codegen_match_arm(case, ctx))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        match #subject_expr {
            #(#arms)*
        }
    })
}

pub(crate) fn codegen_match_stmt_with_mapping(
    subject: &HirExpr,
    cases: &[MatchCase],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let subject_expr = subject.to_rust_expr(ctx)?;

    // Collect all unique keys from all mapping patterns
    let mut all_keys = Vec::new();
    for case in cases {
        if let HirPattern::Mapping { keys, .. } = &case.pattern {
            for key in keys {
                // Only add if not already present
                if !all_keys
                    .iter()
                    .any(|k: &HirExpr| format!("{:?}", k) == format!("{:?}", key))
                {
                    all_keys.push(key.clone());
                }
            }
        }
    }

    // If we have multiple keys across cases, generate tuple match
    if all_keys.len() > 1 {
        let get_calls: Vec<proc_macro2::TokenStream> = all_keys
            .iter()
            .map(|key| {
                let key_expr = key.to_rust_expr(ctx)?;
                Ok(quote! { #subject_expr.get(&#key_expr) })
            })
            .collect::<Result<Vec<_>>>()?;

        let arms: Vec<proc_macro2::TokenStream> = cases
            .iter()
            .map(|case| codegen_match_arm_mapping(case, &all_keys, ctx))
            .collect::<Result<Vec<_>>>()?;

        Ok(quote! {
            match (#(#get_calls),*) {
                #(#arms)*
            }
        })
    } else if all_keys.len() == 1 {
        // Single key - simpler match on single .get()
        let key_expr = all_keys[0].to_rust_expr(ctx)?;

        let arms: Vec<proc_macro2::TokenStream> = cases
            .iter()
            .map(|case| codegen_match_arm_mapping(case, &all_keys, ctx))
            .collect::<Result<_>>()?;

        Ok(quote! {
            match #subject_expr.get(&#key_expr) {
                #(#arms)*
            }
        })
    } else {
        // No keys found - fallback to regular match
        let arms: Vec<proc_macro2::TokenStream> = cases
            .iter()
            .map(|case| codegen_match_arm(case, ctx))
            .collect::<Result<Vec<_>>>()?;

        Ok(quote! {
            match #subject_expr {
                #(#arms)*
            }
        })
    }
}

pub(crate) fn codegen_match_arm(
    case: &MatchCase,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let pattern = codegen_pattern(&case.pattern)?;
    let body_stmts: Vec<proc_macro2::TokenStream> = case
        .body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    if let Some(guard) = &case.guard {
        let guard_expr = guard.to_rust_expr(ctx)?;
        Ok(quote! {
            #pattern if #guard_expr => {
                #(#body_stmts)*
            }
        })
    } else {
        Ok(quote! {
            #pattern => {
                #(#body_stmts)*
            }
        })
    }
}

pub(crate) fn codegen_match_arm_mapping(
    case: &MatchCase,
    all_keys: &[HirExpr],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let body_stmts: Vec<proc_macro2::TokenStream> = case
        .body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    match &case.pattern {
        HirPattern::Mapping { keys, patterns, .. } => {
            if all_keys.len() > 1 {
                // Multi-key tuple pattern
                let tuple_patterns: Vec<proc_macro2::TokenStream> = all_keys
                    .iter()
                    .map(|key| {
                        // Find if this key is in the case's keys
                        if let Some(pos) = keys
                            .iter()
                            .position(|k| format!("{:?}", k) == format!("{:?}", key))
                        {
                            let pat = &patterns[pos];
                            codegen_pattern_for_option(pat)
                        } else {
                            // This key is not matched in this case - use wildcard
                            Ok(quote! { _ })
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;

                if let Some(guard) = &case.guard {
                    let guard_expr = guard.to_rust_expr(ctx)?;
                    Ok(quote! {
                        (#(#tuple_patterns),*) if #guard_expr => {
                            #(#body_stmts)*
                        }
                    })
                } else {
                    Ok(quote! {
                        (#(#tuple_patterns),*) => {
                            #(#body_stmts)*
                        }
                    })
                }
            } else {
                // Single key pattern
                let pattern = if !patterns.is_empty() {
                    codegen_pattern_for_option(&patterns[0])?
                } else {
                    quote! { Some(_) }
                };

                if let Some(guard) = &case.guard {
                    let guard_expr = guard.to_rust_expr(ctx)?;
                    Ok(quote! {
                        #pattern if #guard_expr => {
                            #(#body_stmts)*
                        }
                    })
                } else {
                    Ok(quote! {
                        #pattern => {
                            #(#body_stmts)*
                        }
                    })
                }
            }
        }
        HirPattern::Wildcard => {
            // Wildcard pattern - match anything
            if all_keys.len() > 1 {
                Ok(quote! {
                    _ => {
                        #(#body_stmts)*
                    }
                })
            } else {
                Ok(quote! {
                    _ => {
                        #(#body_stmts)*
                    }
                })
            }
        }
        _ => {
            // Non-mapping pattern in a mapping context - shouldn't happen often
            let pattern = codegen_pattern(&case.pattern)?;
            if let Some(guard) = &case.guard {
                let guard_expr = guard.to_rust_expr(ctx)?;
                Ok(quote! {
                    #pattern if #guard_expr => {
                        #(#body_stmts)*
                    }
                })
            } else {
                Ok(quote! {
                    #pattern => {
                        #(#body_stmts)*
                    }
                })
            }
        }
    }
}

pub(crate) fn codegen_pattern(pattern: &HirPattern) -> Result<proc_macro2::TokenStream> {
    match pattern {
        HirPattern::Value(expr) => {
            let lit = expr_to_pattern_literal(expr)?;
            Ok(lit)
        }
        HirPattern::Singleton(lit) => match lit {
            Literal::None => Ok(quote! { None }),
            Literal::Bool(true) => Ok(quote! { true }),
            Literal::Bool(false) => Ok(quote! { false }),
            _ => bail!("Unsupported singleton literal in pattern"),
        },
        HirPattern::Sequence(patterns) => {
            let inner: Vec<proc_macro2::TokenStream> = patterns
                .iter()
                .map(codegen_pattern)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { [#(#inner),*] })
        }
        HirPattern::Mapping {
            keys,
            patterns,
            rest,
        } => {
            // Rust doesn't have built-in destructuring for HashMaps
            // Generate a guard-based approach or use custom extractors
            // For now, just match on the whole structure
            let _ = (keys, patterns, rest);
            bail!("Map pattern matching requires custom implementation")
        }
        HirPattern::Class {
            cls,
            patterns,
            kwd_attrs,
            kwd_patterns,
        } => {
            let cls_ident = format_ident!("{}", cls);
            if patterns.is_empty() && kwd_attrs.is_empty() {
                // Simple struct match: case Point():
                Ok(quote! { #cls_ident { .. } })
            } else if !kwd_attrs.is_empty() {
                // Named field match: case Point(x=x, y=y):
                let field_patterns: Vec<proc_macro2::TokenStream> = kwd_attrs
                    .iter()
                    .zip(kwd_patterns.iter())
                    .map(|(attr, pat)| {
                        let attr_ident = format_ident!("{}", attr);
                        let pat_tokens = codegen_pattern(pat)?;
                        Ok(quote! { #attr_ident: #pat_tokens })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! { #cls_ident { #(#field_patterns),*, .. } })
            } else {
                // Positional match: case Point(x, y):
                let pos_patterns: Vec<proc_macro2::TokenStream> = patterns
                    .iter()
                    .map(codegen_pattern)
                    .collect::<Result<Vec<_>>>()?;
                Ok(quote! { #cls_ident(#(#pos_patterns),*) })
            }
        }
        HirPattern::Star(name) => {
            if let Some(n) = name {
                let ident = format_ident!("{}", n);
                Ok(quote! { #ident @ .. })
            } else {
                Ok(quote! { .. })
            }
        }
        HirPattern::As { pattern, name } => {
            match (pattern, name) {
                (Some(inner), Some(n)) => {
                    let inner_pat = codegen_pattern(inner)?;
                    let ident = format_ident!("{}", n);
                    Ok(quote! { #inner_pat @ #ident })
                }
                (None, Some(n)) => {
                    // Just a binding: case _ as x:
                    let ident = format_ident!("{}", n);
                    Ok(quote! { #ident })
                }
                (Some(inner), None) => {
                    // Pattern without binding
                    codegen_pattern(inner)
                }
                (None, None) => {
                    // Wildcard
                    Ok(quote! { _ })
                }
            }
        }
        HirPattern::Or(patterns) => {
            let inner: Vec<proc_macro2::TokenStream> = patterns
                .iter()
                .map(codegen_pattern)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { #(#inner)|* })
        }
        HirPattern::Wildcard => Ok(quote! { _ }),
    }
}

pub(crate) fn codegen_pattern_for_option(pattern: &HirPattern) -> Result<proc_macro2::TokenStream> {
    match pattern {
        HirPattern::As {
            pattern: Some(inner),
            name: Some(n),
        } => {
            // case {'key': value as x}: -> Some(&value) (binding to variable)
            let ident = format_ident!("{}", n);
            Ok(quote! { Some(&#ident) })
        }
        HirPattern::As {
            pattern: None,
            name: Some(n),
        } => {
            // Just a binding
            let ident = format_ident!("{}", n);
            Ok(quote! { Some(&#ident) })
        }
        HirPattern::Value(expr) => {
            // Match a specific value
            let lit = expr_to_pattern_literal(expr)?;
            Ok(quote! { Some(&#lit) })
        }
        HirPattern::Wildcard => Ok(quote! { Some(_) }),
        _ => {
            // For other patterns, wrap in Some
            let inner_pat = codegen_pattern(pattern)?;
            Ok(quote! { Some(&#inner_pat) })
        }
    }
}

pub(crate) fn expr_to_pattern_literal(expr: &HirExpr) -> Result<proc_macro2::TokenStream> {
    match expr {
        HirExpr::Literal(lit) => {
            match lit {
                Literal::Int(i) => Ok(quote! { #i }),
                Literal::Float(f) => {
                    // Floats can't be matched directly in Rust, need guard
                    bail!(
                        "Float patterns require guard-based matching: use 'x if x == {}'",
                        f
                    )
                }
                Literal::String(s) => Ok(quote! { #s }),
                Literal::Bool(b) => Ok(quote! { #b }),
                Literal::None => Ok(quote! { None }),
                _ => bail!("Unsupported literal type in pattern"),
            }
        }
        HirExpr::Var(name) => {
            // In Python patterns, a bare name is a binding, not a reference
            let ident = format_ident!("{}", name);
            Ok(quote! { #ident })
        }
        HirExpr::Tuple(elems) => {
            let inner: Vec<proc_macro2::TokenStream> = elems
                .iter()
                .map(expr_to_pattern_literal)
                .collect::<Result<Vec<_>>>()?;
            Ok(quote! { (#(#inner),*) })
        }
        _ => bail!("Unsupported expression type in pattern"),
    }
}

pub(crate) fn convert_assign_target_to_pattern(target: &AssignTarget) -> Result<syn::Pat> {
    match target {
        AssignTarget::Symbol(name) => {
            let ident = safe_ident(name);
            Ok(parse_quote! { #ident })
        }
        AssignTarget::Tuple(targets) => {
            let patterns: Vec<syn::Pat> = targets
                .iter()
                .map(convert_assign_target_to_pattern)
                .collect::<Result<Vec<_>>>()?;
            Ok(parse_quote! { (#(#patterns),*) })
        }
        _ => bail!("Unsupported pattern in async for target"),
    }
}

