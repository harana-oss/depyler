//! Type Helpers code generation

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::keywords::safe_ident;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};


use super::*;
pub(crate) fn expr_returns_usize(expr: &HirExpr) -> bool {
    match expr {
        // Method calls that return usize
        HirExpr::MethodCall { method, .. } => {
            matches!(method.as_str(), "len" | "count" | "capacity")
        }
        // Builtin functions that return usize
        HirExpr::Call { func, .. } => {
            matches!(func.as_str(), "len" | "range")
        }
        // Binary operations might contain usize expressions
        HirExpr::Binary { left, right, .. } => {
            expr_returns_usize(left) || expr_returns_usize(right)
        }
        // All other expressions (Var, Literal, etc.) don't return usize in our HIR
        _ => false,
    }
}

pub(crate) fn needs_type_conversion(target_type: &Type, expr: &HirExpr) -> bool {
    match target_type {
        Type::Int => {
            // Only convert if expression actually returns usize
            // This prevents unnecessary casts like `(x: i32) as i32`
            expr_returns_usize(expr)
        }
        Type::String => {
            // Convert string literals (&str) to String when return type is String
            if matches!(expr, HirExpr::Literal(Literal::String(_))) {
                return true;
            }
            // Methods that return &str need .to_string() when String is expected
            if let HirExpr::MethodCall { method, .. } = expr {
                // These methods return &str in Rust
                let str_ref_methods = [
                    "strip",
                    "trim",
                    "trim_start",
                    "trim_end",
                    "lstrip",
                    "rstrip",
                ];
                if str_ref_methods.contains(&method.as_str()) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

pub(crate) fn apply_type_conversion(value_expr: syn::Expr, target_type: &Type) -> syn::Expr {
    match target_type {
        Type::Int => {
            // Convert to i32 using 'as' cast
            // This handles usize->i32 conversions
            parse_quote! { #value_expr as i32 }
        }
        Type::String => {
            // Convert &str literals to String using .to_string()
            parse_quote! { #value_expr.to_string() }
        }
        _ => value_expr,
    }
}

pub(crate) fn infer_binary_expr_type(
    ctx: &CodeGenContext,
    op: &BinOp,
    left: &HirExpr,
    right: &HirExpr,
) -> Type {
    // For arithmetic ops, if either operand is float, result is float
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod => {
            let left_is_float = is_expr_float(ctx, left);
            let right_is_float = is_expr_float(ctx, right);
            if left_is_float || right_is_float {
                Type::Float
            } else {
                Type::Int
            }
        }
        // Division always returns float in Python
        BinOp::Div => Type::Float,
        // Floor division returns int
        BinOp::FloorDiv => Type::Int,
        // Power can return either, but default to float for safety
        BinOp::Pow => Type::Float,
        // Matrix multiplication - result depends on operands
        BinOp::MatMul => Type::Unknown,
        // Comparison operators return bool
        BinOp::Eq
        | BinOp::NotEq
        | BinOp::Lt
        | BinOp::LtEq
        | BinOp::Gt
        | BinOp::GtEq
        | BinOp::And
        | BinOp::Or
        | BinOp::In
        | BinOp::NotIn
        | BinOp::Is
        | BinOp::IsNot => Type::Bool,
        // Bitwise operators return int
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::LShift | BinOp::RShift => Type::Int,
    }
}

pub(crate) fn is_expr_float(ctx: &CodeGenContext, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Literal(Literal::Float(_)) => true,
        HirExpr::Var(name) => matches!(ctx.var_types.get(name), Some(Type::Float)),
        HirExpr::Binary { op, left, right } => {
            // Division always returns float
            if matches!(op, BinOp::Div | BinOp::Pow) {
                return true;
            }
            // For other ops, check if either operand is float
            is_expr_float(ctx, left) || is_expr_float(ctx, right)
        }
        HirExpr::Call { func, .. } => {
            // Common float-returning functions
            matches!(
                func.as_str(),
                "float" | "sqrt" | "sin" | "cos" | "tan" | "log" | "exp"
            )
        }
        _ => false,
    }
}

pub(crate) fn expr_is_optional(expr: &HirExpr, ctx: &CodeGenContext) -> bool {
    match expr {
        // Variable - check its type in context
        HirExpr::Var(name) => {
            matches!(ctx.var_types.get(name), Some(Type::Optional(_)))
        }
        // Attribute access - check class field type
        HirExpr::Attribute { value, attr } => {
            if let HirExpr::Var(base_name) = value.as_ref() {
                // Look up the base variable's type
                if let Some(base_type) = ctx.var_types.get(base_name) {
                    if let Type::Custom(class_name) = base_type {
                        // Look up the field type in the class
                        if let Some(fields) = ctx.class_field_types.get(class_name) {
                            return matches!(fields.get(attr), Some(Type::Optional(_)));
                        }
                    }
                }
            }
            false
        }
        // Method calls that return Optional
        HirExpr::MethodCall { method, args, .. } => {
            // dict.get(key) returns Option, but dict.get(key, default) returns T
            // list.get(index) returns Option
            if method == "get" {
                // Only consider it optional if there's exactly 1 argument (no default)
                args.len() == 1
            } else {
                false
            }
        }
        // next(iterator, None) returns Option<T>
        HirExpr::Call { func, args, .. } => {
            if func == "next" && args.len() == 2 {
                matches!(&args[1], HirExpr::Literal(crate::hir::Literal::None))
            } else {
                false
            }
        }
        // Ternary expression with None in else branch is Optional
        // e.g., `x.field if x else None` produces Option<T>
        HirExpr::IfExpr { orelse, .. } => {
            matches!(orelse.as_ref(), HirExpr::Literal(crate::hir::Literal::None))
        }
        _ => false,
    }
}

pub(crate) fn apply_optional_truthiness(field_type: &Type, cond_expr: syn::Expr) -> syn::Expr {
    match field_type {
        Type::Optional(inner) => match inner.as_ref() {
            Type::String => parse_quote! { #cond_expr.as_ref().is_some_and(|s| !s.is_empty()) },
            Type::List(_) | Type::Dict(_, _) | Type::Set(_) => {
                parse_quote! { #cond_expr.as_ref().is_some_and(|v| !v.is_empty()) }
            }
            Type::Int => parse_quote! { #cond_expr.is_some_and(|n| n != 0) },
            Type::Float => parse_quote! { #cond_expr.is_some_and(|n| n != 0.0) },
            _ => parse_quote! { #cond_expr.is_some() },
        },
        // Non-optional types fall through to simple truthiness
        Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) => {
            parse_quote! { !#cond_expr.is_empty() }
        }
        Type::Int => parse_quote! { #cond_expr != 0 },
        Type::Float => parse_quote! { #cond_expr != 0.0 },
        Type::Bool => cond_expr,
        _ => cond_expr,
    }
}

pub(crate) fn apply_truthiness_conversion(
    condition: &HirExpr,
    cond_expr: syn::Expr,
    ctx: &CodeGenContext,
) -> syn::Expr {
    // `x is None` and `x is not None` are already converted to `.is_none()`/`.is_some()`
    // in convert_binary, so we don't need to apply truthiness conversion to them
    if let HirExpr::Binary { op, left, right } = condition {
        if matches!(op, BinOp::Is | BinOp::IsNot) {
            let is_left_none = matches!(left.as_ref(), HirExpr::Literal(Literal::None));
            let is_right_none = matches!(right.as_ref(), HirExpr::Literal(Literal::None));
            if is_left_none || is_right_none {
                return cond_expr;
            }
        }
    }

    // Check if this is a variable reference that needs truthiness conversion
    if let HirExpr::Var(var_name) = condition {
        if let Some(var_type) = ctx.var_types.get(var_name) {
            return apply_optional_truthiness(var_type, cond_expr);
        }
    }

    // Python: if args.output (where output is optional)
    // Rust: if args.output.is_some()
    if let HirExpr::Attribute { value, attr } = condition {
        if let HirExpr::Var(obj_name) = value.as_ref() {
            // Check class field types for Optional fields (dataclasses)
            if let Some(var_type) = ctx.var_types.get(obj_name) {
                if let Type::Custom(class_name) = var_type {
                    if let Some(fields) = ctx.class_field_types.get(class_name) {
                        if let Some(field_type) = fields.get(attr) {
                            // For Optional fields, generate the expression without .clone()
                            // since is_some()/is_some_and()/as_ref() only need a reference
                            let obj_ident = safe_ident(obj_name);
                            let attr_ident = safe_ident(attr);
                            let field_expr: syn::Expr = parse_quote! { #obj_ident.#attr_ident };
                            return apply_optional_truthiness(field_type, field_expr);
                        }
                    }
                }
            }

            // Check if this is accessing an args variable from ArgumentParser
            let is_args_var = ctx.argparser_tracker.parsers.values().any(|parser_info| {
                parser_info
                    .args_var
                    .as_ref()
                    .is_some_and(|args_var| args_var == obj_name)
            });

            if is_args_var {
                // Check if this field is optional (Option<T> type, not boolean)
                let is_optional_field = ctx.argparser_tracker.parsers.values().any(|parser_info| {
                    parser_info.arguments.iter().any(|arg| {
                        let field_name = arg.rust_field_name();
                        if field_name != *attr {
                            return false;
                        }

                        // Argument is NOT an Option if it has action="store_true" or "store_false"
                        if matches!(
                            arg.action.as_deref(),
                            Some("store_true") | Some("store_false")
                        ) {
                            return false;
                        }

                        // Argument is an Option<T> if: not required AND no default value AND not positional
                        // Positional arguments are always required (Vec for nargs)
                        !arg.is_positional
                            && !arg.required.unwrap_or(false)
                            && arg.default.is_none()
                    })
                });

                if is_optional_field {
                    // Convert Option<T> to boolean using .is_some()
                    return parse_quote! { #cond_expr.is_some() };
                }
            }
        }
    }

    // Check if this looks like an Option<T> based on common patterns:
    // - Variable from `env::var(...).ok()` call
    // - Method calls that return Option (dict.get(), etc.)
    if looks_like_option_expr(condition) {
        return parse_quote! { #cond_expr.is_some() };
    }

    // Not a variable or no type info - use as-is
    cond_expr
}

pub(crate) fn looks_like_option_expr(expr: &HirExpr) -> bool {
    match expr {
        // Check if variable was assigned from an Option-returning call
        HirExpr::Var(_) => {
            // Type should be checked via ctx.var_types (handled above)
            // This is just a fallback
            false
        }
        // Method call ending in .ok() → definitely Option
        HirExpr::MethodCall { method, .. } if method == "ok" => true,
        // Method call to .get() → usually Option (dict/map lookup)
        HirExpr::MethodCall { method, .. } if method == "get" => true,
        _ => false,
    }
}

pub(crate) fn hir_type_to_tokens(ty: &Type, _ctx: &CodeGenContext) -> proc_macro2::TokenStream {
    use quote::quote;

    match ty {
        Type::Int => quote! { i64 },
        Type::Float => quote! { f64 },
        Type::String => quote! { String },
        Type::Bool => quote! { bool },
        Type::None => quote! { () },
        Type::Unknown => quote! { () }, // Default to () for unknown types
        Type::List(elem) => {
            let elem_ty = hir_type_to_tokens(elem, _ctx);
            quote! { Vec<#elem_ty> }
        }
        Type::Dict(key, value) => {
            let key_ty = hir_type_to_tokens(key, _ctx);
            let val_ty = hir_type_to_tokens(value, _ctx);
            quote! { std::collections::HashMap<#key_ty, #val_ty> }
        }
        Type::Tuple(types) => {
            let elem_types: Vec<_> = types.iter().map(|t| hir_type_to_tokens(t, _ctx)).collect();
            quote! { (#(#elem_types),*) }
        }
        Type::Optional(inner) => {
            let inner_ty = hir_type_to_tokens(inner, _ctx);
            quote! { Option<#inner_ty> }
        }
        Type::Custom(name) => {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
        _ => quote! { () }, // Fallback for other types (Set, Function, Generic, Union, Array, etc.)
    }
}

