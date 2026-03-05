//! Helpers code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

use super::*;
pub(crate) fn extract_nested_indices_tokens(
    expr: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<(syn::Expr, Vec<syn::Expr>)> {
    let mut indices = Vec::new();
    let mut current = expr;

    // Walk up the chain collecting indices
    loop {
        match current {
            HirExpr::Index { base, index } => {
                let index_expr = index.to_rust_expr(ctx)?;
                indices.push(index_expr);
                current = base;
            }
            _ => {
                // We've reached the base
                let base_expr = current.to_rust_expr(ctx)?;
                indices.reverse(); // We collected from inner to outer, need outer to inner
                return Ok((base_expr, indices));
            }
        }
    }
}

pub(crate) fn extract_nested_indices_tokens_no_clone(
    expr: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<(syn::Expr, Vec<syn::Expr>)> {
    let mut indices = Vec::new();
    let mut current = expr;

    // Walk up the chain collecting indices
    loop {
        match current {
            HirExpr::Index { base, index } => {
                let index_expr = index.to_rust_expr(ctx)?;
                indices.push(index_expr);
                current = base;
            }
            _ => {
                // We've reached the base - build it without clone
                let base_expr = build_expr_no_clone(current);
                indices.reverse(); // We collected from inner to outer, need outer to inner
                return Ok((base_expr, indices));
            }
        }
    }
}

pub(crate) fn build_expr_no_clone(expr: &HirExpr) -> syn::Expr {
    match expr {
        HirExpr::Var(name) => {
            let ident = format_ident!("{}", name);
            parse_quote! { #ident }
        }
        HirExpr::Attribute { value, attr } => {
            let base = build_expr_no_clone(value);
            let attr_ident = format_ident!("{}", attr);
            parse_quote! { #base.#attr_ident }
        }
        HirExpr::Index { base, index } => {
            // For nested index expressions in the base, build without clone
            let base_expr = build_expr_no_clone(base);
            // Note: index is converted separately, just use a placeholder pattern
            // This shouldn't be reached in normal flow since indices are collected above
            parse_quote! { #base_expr }
        }
        _ => {
            // Fallback - shouldn't happen often
            parse_quote! { () }
        }
    }
}

pub(crate) fn is_attribute_sourced_expr(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { .. } => true,
        HirExpr::Index { base, .. } => is_attribute_sourced_expr(base),
        HirExpr::IfExpr { body, orelse, .. } => {
            is_attribute_sourced_expr(body) && is_attribute_sourced_expr(orelse)
        }
        _ => false,
    }
}

/// Check if an IfExpr branch is a Copy-type attribute access.
pub(crate) fn is_copy_type_branch(expr: &HirExpr, ctx: &CodeGenContext) -> bool {
    match expr {
        HirExpr::Attribute { value, attr } => ctx.is_attribute_copy_type(value, attr),
        HirExpr::IfExpr { body, orelse, .. } => {
            is_copy_type_branch(body, ctx) && is_copy_type_branch(orelse, ctx)
        }
        _ => false,
    }
}

/// Like `is_empty_collection_init_expr` but excludes primitive literal defaults (0, false, "").
/// Used to gate borrowing decisions — primitive defaults should not trigger reference types.
pub(crate) fn is_empty_collection_init_no_primitives(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::List(items) => items.is_empty(),
        HirExpr::Dict(pairs) => pairs.is_empty(),
        HirExpr::Set(items) => items.is_empty(),
        HirExpr::Call { func, args, .. } => {
            let is_empty_constructor = matches!(
                func.as_str(),
                "list" | "dict" | "set" | "Vec" | "HashMap" | "HashSet"
            );
            is_empty_constructor && args.is_empty()
        }
        _ => false,
    }
}

pub(crate) fn is_empty_collection_init_expr(expr: &HirExpr) -> bool {
    use crate::hir::Literal;
    match expr {
        // Empty list literal: []
        HirExpr::List(items) => items.is_empty(),
        // Empty dict literal: {}
        HirExpr::Dict(pairs) => pairs.is_empty(),
        // Empty set literal: set()
        HirExpr::Set(items) => items.is_empty(),
        // Built-in constructors: list(), dict(), set(), Vec::new(), etc.
        HirExpr::Call { func, args, .. } => {
            let is_empty_constructor = matches!(
                func.as_str(),
                "list" | "dict" | "set" | "Vec" | "HashMap" | "HashSet"
            );
            is_empty_constructor && args.is_empty()
        }
        // Default values that are common placeholder initializations
        HirExpr::Literal(lit) => match lit {
            Literal::Int(0) => true,
            Literal::Float(f) => *f == 0.0,
            Literal::String(s) => s.is_empty(),
            Literal::Bool(false) | Literal::None => true,
            _ => false,
        },
        _ => false,
    }
}

pub(crate) fn is_enum_variant_expr(expr: &HirExpr, ctx: &CodeGenContext) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } => {
            if let HirExpr::Var(type_name) = value.as_ref() {
                // Check if it's a known enum type
                if ctx.enum_names.contains(type_name) {
                    return true;
                }
                // Heuristic: PascalCase name that's not a known struct parameter
                let first_char = type_name.chars().next().unwrap_or('a');
                if first_char.is_uppercase()
                    && !ctx.current_func_ref_params.contains(type_name)
                    && !ctx.current_func_mut_ref_params.contains(type_name)
                {
                    return true;
                }
            }
            false
        }
        HirExpr::IfExpr { body, orelse, .. } => {
            // Both branches must be enum variants
            is_enum_variant_expr(body, ctx) && is_enum_variant_expr(orelse, ctx)
        }
        _ => false,
    }
}

pub(crate) fn is_var_used_in_expr(var_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(name) => name == var_name,
        HirExpr::Binary { left, right, .. } => {
            is_var_used_in_expr(var_name, left) || is_var_used_in_expr(var_name, right)
        }
        HirExpr::Unary { operand, .. } => is_var_used_in_expr(var_name, operand),
        HirExpr::Call { func: _, args, .. } => {
            args.iter().any(|arg| is_var_used_in_expr(var_name, arg))
        }
        HirExpr::MethodCall { object, args, .. } => {
            is_var_used_in_expr(var_name, object)
                || args.iter().any(|arg| is_var_used_in_expr(var_name, arg))
        }
        HirExpr::Index { base, index } => {
            is_var_used_in_expr(var_name, base) || is_var_used_in_expr(var_name, index)
        }
        HirExpr::Attribute { value, .. } => is_var_used_in_expr(var_name, value),
        HirExpr::List(elements)
        | HirExpr::Tuple(elements)
        | HirExpr::Set(elements)
        | HirExpr::FrozenSet(elements) => elements.iter().any(|e| is_var_used_in_expr(var_name, e)),
        HirExpr::Dict(pairs) => pairs
            .iter()
            .any(|(k, v)| is_var_used_in_expr(var_name, k) || is_var_used_in_expr(var_name, v)),
        HirExpr::IfExpr { test, body, orelse } => {
            is_var_used_in_expr(var_name, test)
                || is_var_used_in_expr(var_name, body)
                || is_var_used_in_expr(var_name, orelse)
        }
        HirExpr::Lambda { params: _, body } => is_var_used_in_expr(var_name, body),
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            is_var_used_in_expr(var_name, base)
                || start
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
                || stop
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
                || step
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
        }
        HirExpr::FString { parts } => parts.iter().any(|part| match part {
            crate::hir::FStringPart::Expr(expr) => is_var_used_in_expr(var_name, expr),
            crate::hir::FStringPart::Literal(_) => false,
        }),
        HirExpr::ListComp {
            element,
            target: _,
            iter,
            condition,
        }
        | HirExpr::SetComp {
            element,
            target: _,
            iter,
            condition,
        } => {
            // Check if variable is used in the element expression, iterator, or condition
            // Note: We intentionally skip checking the target (comprehension variable)
            // since it's scoped to the comprehension
            is_var_used_in_expr(var_name, element)
                || is_var_used_in_expr(var_name, iter)
                || condition
                    .as_ref()
                    .is_some_and(|cond| is_var_used_in_expr(var_name, cond))
        }
        HirExpr::DictComp {
            key,
            value,
            target: _,
            iter,
            condition,
        } => {
            // Check if variable is used in key, value, iterator, or condition
            is_var_used_in_expr(var_name, key)
                || is_var_used_in_expr(var_name, value)
                || is_var_used_in_expr(var_name, iter)
                || condition
                    .as_ref()
                    .is_some_and(|cond| is_var_used_in_expr(var_name, cond))
        }
        HirExpr::GeneratorExp {
            element,
            generators,
        } => {
            // Check element and all generators
            is_var_used_in_expr(var_name, element)
                || generators.iter().any(|generator| {
                    is_var_used_in_expr(var_name, &generator.iter)
                        || generator
                            .conditions
                            .iter()
                            .any(|cond| is_var_used_in_expr(var_name, cond))
                })
        }
        _ => false, // Literals and other expressions don't reference variables
    }
}

pub(crate) fn is_var_used_in_stmt(var_name: &str, stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            // Check both target (e.g., d[k]) and value (e.g., v)
            is_var_used_in_assign_target(var_name, target) || is_var_used_in_expr(var_name, value)
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            is_var_used_in_expr(var_name, condition)
                || then_body.iter().any(|s| is_var_used_in_stmt(var_name, s))
                || else_body
                    .as_ref()
                    .is_some_and(|body| body.iter().any(|s| is_var_used_in_stmt(var_name, s)))
        }
        HirStmt::While { condition, body } => {
            is_var_used_in_expr(var_name, condition)
                || body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::For { iter, body, .. } => {
            is_var_used_in_expr(var_name, iter)
                || body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::Return(Some(expr)) => is_var_used_in_expr(var_name, expr),
        HirStmt::Expr(expr) => is_var_used_in_expr(var_name, expr),
        HirStmt::Raise { exception, .. } => exception
            .as_ref()
            .is_some_and(|e| is_var_used_in_expr(var_name, e)),
        HirStmt::Assert { test, msg, .. } => {
            is_var_used_in_expr(var_name, test)
                || msg
                    .as_ref()
                    .is_some_and(|m| is_var_used_in_expr(var_name, m))
        }
        _ => false,
    }
}

pub(crate) fn is_var_used_in_assign_target(var_name: &str, target: &AssignTarget) -> bool {
    match target {
        AssignTarget::Symbol(s) => s == var_name,
        AssignTarget::Index { base, index } => {
            is_var_used_in_expr(var_name, base) || is_var_used_in_expr(var_name, index)
        }
        AssignTarget::Slice {
            base,
            start,
            stop,
            step,
        } => {
            is_var_used_in_expr(var_name, base)
                || start
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
                || stop
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
                || step
                    .as_ref()
                    .is_some_and(|s| is_var_used_in_expr(var_name, s))
        }
        AssignTarget::Attribute { value, .. } => is_var_used_in_expr(var_name, value),
        AssignTarget::Tuple(targets) => targets
            .iter()
            .any(|t| is_var_used_in_assign_target(var_name, t)),
        AssignTarget::Starred(s) => s == var_name,
    }
}

pub(crate) fn is_var_mutated_in_stmts(var_name: &str, stmts: &[HirStmt]) -> bool {
    for stmt in stmts {
        if is_var_mutated_in_stmt(var_name, stmt) {
            return true;
        }
    }
    false
}

pub(crate) fn is_var_mutated_in_stmt(var_name: &str, stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { target, .. } => {
            match target {
                AssignTarget::Symbol(name) => name == var_name,
                AssignTarget::Attribute { value, .. } => {
                    // Check if it's an attribute of the variable (e.g., x.field = ...)
                    matches!(value.as_ref(), HirExpr::Var(v) if v == var_name)
                }
                AssignTarget::Index { base, .. } => {
                    // Check if it's an index of the variable (e.g., x[i] = ...)
                    matches!(base.as_ref(), HirExpr::Var(v) if v == var_name)
                }
                AssignTarget::Tuple(targets) => {
                    // Check if variable is in any tuple element
                    targets.iter().any(|t| match t {
                        AssignTarget::Symbol(name) => name == var_name,
                        _ => false,
                    })
                }
                _ => false,
            }
        }
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            is_var_mutated_in_stmts(var_name, then_body)
                || else_body
                    .as_ref()
                    .map(|body| is_var_mutated_in_stmts(var_name, body))
                    .unwrap_or(false)
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            is_var_mutated_in_stmts(var_name, body)
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            is_var_mutated_in_stmts(var_name, body)
                || handlers
                    .iter()
                    .any(|h| is_var_mutated_in_stmts(var_name, &h.body))
                || orelse
                    .as_ref()
                    .map(|o| is_var_mutated_in_stmts(var_name, o))
                    .unwrap_or(false)
                || finalbody
                    .as_ref()
                    .map(|f| is_var_mutated_in_stmts(var_name, f))
                    .unwrap_or(false)
        }
        _ => false,
    }
}

pub(crate) fn exprs_are_equivalent(expr1: &HirExpr, expr2: &HirExpr) -> bool {
    match (expr1, expr2) {
        // Variables with same name
        (HirExpr::Var(v1), HirExpr::Var(v2)) => v1 == v2,
        // String literals with same value
        (HirExpr::Literal(Literal::String(s1)), HirExpr::Literal(Literal::String(s2))) => s1 == s2,
        // Integer literals with same value
        (HirExpr::Literal(Literal::Int(i1)), HirExpr::Literal(Literal::Int(i2))) => i1 == i2,
        // Attribute access with same base and attr
        (
            HirExpr::Attribute {
                value: v1,
                attr: a1,
            },
            HirExpr::Attribute {
                value: v2,
                attr: a2,
            },
        ) => a1 == a2 && exprs_are_equivalent(v1.as_ref(), v2.as_ref()),
        // Index access with same base and index
        (
            HirExpr::Index {
                base: b1,
                index: i1,
            },
            HirExpr::Index {
                base: b2,
                index: i2,
            },
        ) => {
            exprs_are_equivalent(b1.as_ref(), b2.as_ref())
                && exprs_are_equivalent(i1.as_ref(), i2.as_ref())
        }
        // Other cases - not equivalent
        _ => false,
    }
}
