//! Expression code generation - literals

#![allow(unused_imports)]

use crate::hir::*;
use crate::rust_generator::context::{CodeGenContext, ToRustExpr};
use crate::rust_generator::direct_rules::to_pascal_case;
use crate::rust_generator::return_type_expects_float;
use crate::rust_generator::type_gen::convert_binop;
use crate::optimizations::string_optimization::{StringContext, StringOptimizer};
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::{self, parse_quote};
use super::ExpressionConverter;

impl<'a, 'b> ExpressionConverter<'a, 'b> {
    /// Convert Python format string to Rust format string
    /// Python uses {} or {:spec} placeholders, Rust uses similar but with some differences
    pub(super) fn convert_python_format_to_rust(&self, format_str: &str) -> String {
        // Simple conversion: Python format specs are largely compatible with Rust
        // Key differences:
        // - Python {:d} for decimal -> Rust {:} or no format spec needed
        // - Python {:s} for string -> Rust {:} or no format spec needed
        // - Python {:.2f} for float precision -> Rust {:.2} (same)
        // - Python {:>10} for alignment -> Rust {:>10} (same)
        // - Python {0} positional -> Rust {0} (same)
        // - Python {name} named -> Rust {name} (same)

        let mut result = String::with_capacity(format_str.len());
        let mut chars = format_str.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    // {{ is an escaped brace in Python, same in Rust
                    result.push('{');
                    result.push('{');
                    chars.next();
                } else {
                    // Start of a format placeholder
                    result.push('{');
                    // Copy until we find the closing brace
                    let mut in_spec = false;
                    while let Some(&inner) = chars.peek() {
                        chars.next();
                        if inner == '}' {
                            result.push('}');
                            break;
                        }
                        if inner == ':' {
                            in_spec = true;
                        }
                        // Convert Python-specific format specs
                        if in_spec {
                            // Skip 'd' and 's' type specifiers as Rust infers them
                            if inner == 'd' || inner == 's' {
                                // Check if this is at the end (before })
                                if chars.peek() == Some(&'}') {
                                    continue;
                                }
                            }
                        }
                        result.push(inner);
                    }
                }
            } else if c == '}' {
                if chars.peek() == Some(&'}') {
                    // }} is an escaped brace
                    result.push('}');
                    result.push('}');
                    chars.next();
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    pub(super) fn convert_fstring(&mut self, parts: &[FStringPart]) -> Result<syn::Expr> {
        // Handle empty f-strings
        if parts.is_empty() {
            return Ok(parse_quote! { "".to_string() });
        }

        // Check if it's just a plain string (no expressions)
        let has_expressions = parts.iter().any(|p| matches!(p, FStringPart::Expr(_)));

        if !has_expressions {
            // Just literal parts - concatenate them
            let mut result = String::new();
            for part in parts {
                if let FStringPart::Literal(s) = part {
                    result.push_str(s);
                }
            }
            return Ok(parse_quote! { #result.to_string() });
        }

        // Build format string template and collect arguments
        let mut template = String::new();
        let mut args = Vec::new();

        for part in parts {
            match part {
                FStringPart::Literal(s) => {
                    template.push_str(s);
                }
                FStringPart::Expr(expr) => {
                    // - Collections (Vec, HashMap, HashSet): Use {:?} debug formatting
                    // - Scalars (String, i32, f64, bool): Use {} Display formatting
                    // - Option types: Display "None" or unwrapped value
                    // This matches Python semantics where lists/dicts have their own repr
                    let arg_expr = expr.to_rust_expr(self.ctx)?;

                    // Use get_optional_inner_type to detect Optional types for both
                    // variables and class field attributes (dataclass fields)
                    let optional_inner = self.ctx.get_optional_inner_type(expr.as_ref());
                    let is_option = optional_inner.is_some()
                        || match expr.as_ref() {
                            HirExpr::Attribute { value, attr } => {
                                if let HirExpr::Var(obj_name) = value.as_ref() {
                                    let is_args_var =
                                        self.ctx.argparser_tracker.parsers.values().any(
                                            |parser_info| {
                                                parser_info
                                                    .args_var
                                                    .as_ref()
                                                    .is_some_and(|args_var| args_var == obj_name)
                                            },
                                        );

                                    if is_args_var {
                                        // Check if this argument is optional (Option<T> type, not boolean)
                                        self.ctx.argparser_tracker.parsers.values().any(
                                            |parser_info| {
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
                                                    !arg.is_positional
                                                        && !arg.required.unwrap_or(false)
                                                        && arg.default.is_none()
                                                })
                                            },
                                        )
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        };

                    // Determine if this expression is a collection type
                    let is_collection = match expr.as_ref() {
                        // Case 1: Simple variable (e.g., targets)
                        HirExpr::Var(var_name) => {
                            if let Some(var_type) = self.ctx.var_types.get(var_name) {
                                matches!(var_type, Type::List(_) | Type::Dict(_, _) | Type::Set(_))
                            } else {
                                false
                            }
                        }
                        // Case 2: Attribute access (e.g., args.targets)
                        HirExpr::Attribute { value, attr } => {
                            // Check if this is accessing a field from argparse Args struct
                            if let HirExpr::Var(obj_name) = value.as_ref() {
                                // Check if obj_name is the args variable from ArgumentParser
                                let is_args_var = self.ctx.argparser_tracker.parsers.values().any(
                                    |parser_info| {
                                        parser_info
                                            .args_var
                                            .as_ref()
                                            .is_some_and(|args_var| args_var == obj_name)
                                    },
                                );

                                if is_args_var {
                                    // Look up the field type in argparse arguments
                                    self.ctx
                                        .argparser_tracker
                                        .parsers
                                        .values()
                                        .any(|parser_info| {
                                            parser_info.arguments.iter().any(|arg| {
                                                // Match field name (normalized from Python argument name)
                                                let field_name = arg.rust_field_name();
                                                if field_name == *attr {
                                                    // Check if this field is a collection type
                                                    // Either explicit type annotation OR inferred from nargs
                                                    let is_vec_from_nargs = matches!(
                                                        arg.nargs.as_deref(),
                                                        Some("+") | Some("*")
                                                    );
                                                    let is_collection_type =
                                                        if let Some(ref arg_type) = arg.arg_type {
                                                            matches!(
                                                                arg_type,
                                                                Type::List(_)
                                                                    | Type::Dict(_, _)
                                                                    | Type::Set(_)
                                                            )
                                                        } else {
                                                            false
                                                        };
                                                    is_vec_from_nargs || is_collection_type
                                                } else {
                                                    false
                                                }
                                            })
                                        })
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    let final_arg = if is_option {
                        // Option<T> doesn't implement Display, so we need to unwrap it
                        // For string-like types, display the value or "None"
                        parse_quote! {
                            {
                                match &#arg_expr {
                                    Some(v) => format!("{}", v),
                                    None => "None".to_string(),
                                }
                            }
                        }
                    } else {
                        arg_expr
                    };

                    if is_collection {
                        // Use debug formatting for collections (matches Python's list/dict repr)
                        template.push_str("{:?}");
                    } else {
                        // Use Display formatting for scalars (and wrapped Options)
                        template.push_str("{}");
                    }

                    args.push(final_arg);
                }
            }
        }

        // Generate format!() macro call
        if args.is_empty() {
            // No arguments (shouldn't happen but be safe)
            Ok(parse_quote! { #template.to_string() })
        } else {
            // Build the format! call with template and arguments
            Ok(parse_quote! { format!(#template, #(#args),*) })
        }
    }

    pub(super) fn convert_ifexpr(
        &mut self,
        test: &HirExpr,
        body: &HirExpr,
        orelse: &HirExpr,
    ) -> Result<syn::Expr> {
        // Python: `args.include if args.include else []` (check if list is non-empty)
        // Rust: Just `args.include` (clap initializes Vec to empty, so redundant check)
        // This pattern is common with argparse + Vec/Option fields
        if test == body {
            // Pattern: `x if x else y` → just use `x` (the condition is redundant)
            // This avoids type errors where Vec/Option can't be used as bool
            return body.to_rust_expr(self.ctx);
        }

        // Pattern: `x if x is not None else default` → `x.unwrap_or(default)`
        // This handles Optional types correctly by unwrapping the Some value
        if let HirExpr::Binary { op, left, right } = test {
            if matches!(op, BinOp::IsNot)
                && matches!(right.as_ref(), HirExpr::Literal(Literal::None))
            {
                // Test is `x is not None`
                if left.as_ref() == body {
                    // Body is the same variable being tested
                    let var_expr = body.to_rust_expr(self.ctx)?;
                    let default_expr = orelse.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! { #var_expr.unwrap_or(#default_expr) });
                }
            } else if matches!(op, BinOp::Is)
                && matches!(right.as_ref(), HirExpr::Literal(Literal::None))
            {
                // Test is `x is None` - inverted logic
                if left.as_ref() == body {
                    // Pattern: `x if x is None else default` → `x.unwrap_or(default)`
                    // This is unusual but handle it for completeness
                    let var_expr = body.to_rust_expr(self.ctx)?;
                    let default_expr = orelse.to_rust_expr(self.ctx)?;
                    return Ok(parse_quote! { #var_expr.unwrap_or(#default_expr) });
                }
            }
        }

        let mut test_expr = test.to_rust_expr(self.ctx)?;

        // When both branches are attribute accesses (e.g., state.home_x if ... else state.away_x),
        // avoid cloning ONLY when the base is not a reference parameter.
        // If the base is a reference (&State), we must clone to get an owned value.
        // EXCEPTION: If generate_borrow is set, the caller has determined borrowing is safe.
        let both_attrs = matches!(body, HirExpr::Attribute { .. })
            && matches!(orelse, HirExpr::Attribute { .. });

        // Check if both branches are enum variants (e.g., Team.Home if ... else Team.Away)
        // Enum variants are Copy types and should never be borrowed.
        let both_enum_variants =
            self.is_enum_variant_expr(body) && self.is_enum_variant_expr(orelse);

        // Check if both branches produce Copy types (primitives like i32, f64, bool).
        // Copy types should never be wrapped in references.
        let both_copy_types = self.is_copy_type_expr(body) && self.is_copy_type_expr(orelse);

        let base_is_ref_param = if both_attrs {
            // Check if the base of either attribute is a reference parameter
            let body_base_is_ref = if let HirExpr::Attribute { value, .. } = body {
                self.is_ref_param_base(value)
            } else {
                false
            };
            let orelse_base_is_ref = if let HirExpr::Attribute { value, .. } = orelse {
                self.is_ref_param_base(value)
            } else {
                false
            };
            body_base_is_ref || orelse_base_is_ref
        } else {
            false
        };

        // Skip cloning if:
        // 1. Both branches are attributes AND the base is not a reference parameter, OR
        // 2. generate_borrow is set (caller has determined borrowing is safe), OR
        // 3. Both branches produce Copy types (no clone needed for Copy)
        let skip_clone =
            (both_attrs && !base_is_ref_param) || self.ctx.generate_borrow || both_copy_types;
        let was_prevent_clone = self.ctx.prevent_clone;
        let was_clone_already_applied = self.ctx.clone_already_applied;
        if skip_clone {
            self.ctx.prevent_clone = true;
        }
        // Reset clone_already_applied before each branch to ensure consistent cloning
        self.ctx.clone_already_applied = false;
        let mut body_expr = body.to_rust_expr(self.ctx)?;
        self.ctx.clone_already_applied = false;
        let mut orelse_expr = orelse.to_rust_expr(self.ctx)?;
        self.ctx.clone_already_applied = was_clone_already_applied;
        if skip_clone {
            self.ctx.prevent_clone = was_prevent_clone;
        }

        // When generate_mut_borrow is set, wrap each branch in &mut for mutable field access
        // This handles: `team_stats = state.home_stats if cond else state.away_stats`
        // → `if cond { &mut state.home_stats } else { &mut state.away_stats }`
        // Skip for enum variants, Copy types, and primitive cast contexts - they should not be borrowed.
        if self.ctx.generate_mut_borrow
            && both_attrs
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &mut #body_expr };
            orelse_expr = parse_quote! { &mut #orelse_expr };
        }
        // When generate_borrow is set, wrap each branch in & for immutable borrow
        // This handles: `players = state.home_players if cond else state.away_players`
        // → `if cond { &state.home_players } else { &state.away_players }`
        // Skip for enum variants, Copy types, and primitive cast contexts - they should not be borrowed.
        else if self.ctx.generate_borrow
            && both_attrs
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &#body_expr };
            orelse_expr = parse_quote! { &#orelse_expr };
        }
        // When function returns a mutable reference, wrap each branch in &mut
        // This handles: `return state.home_stats if ... else state.away_stats`
        // → `if ... { &mut state.home_stats } else { &mut state.away_stats }`
        // Skip for enum variants, Copy types, and primitive cast contexts.
        else if self.ctx.returns_mutable_reference
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &mut #body_expr };
            orelse_expr = parse_quote! { &mut #orelse_expr };
        }
        // When function returns an immutable reference, wrap each branch in &
        // This handles: `return state.home_players if ... else state.away_players`
        // → `if ... { &state.home_players } else { &state.away_players }`
        // Skip for enum variants, Copy types, and primitive cast contexts.
        else if self.ctx.returns_reference
            && !both_enum_variants
            && !both_copy_types
            && !self.ctx.in_primitive_cast
        {
            body_expr = parse_quote! { &#body_expr };
            orelse_expr = parse_quote! { &#orelse_expr };
        }

        // Ensure type consistency: if either branch is a string literal, wrap with .to_string()
        let body_is_string_lit = matches!(body, HirExpr::Literal(Literal::String(_)));
        let orelse_is_string_lit = matches!(orelse, HirExpr::Literal(Literal::String(_)));
        if body_is_string_lit || orelse_is_string_lit {
            if body_is_string_lit {
                body_expr = parse_quote! { #body_expr.to_string() };
            }
            if orelse_is_string_lit {
                // orelse_expr was made immutable, so this case is handled below
            }
        }

        // Ensure numeric type consistency: promote int to float when branches have mixed types
        // Python: `1 if cond else 1.0` → Rust: `if cond { 1.0 } else { 1.0 }`
        let body_is_int_lit = matches!(body, HirExpr::Literal(Literal::Int(_)));
        let body_is_float_lit = matches!(body, HirExpr::Literal(Literal::Float(_)));
        let orelse_is_int_lit = matches!(orelse, HirExpr::Literal(Literal::Int(_)));
        let orelse_is_float_lit = matches!(orelse, HirExpr::Literal(Literal::Float(_)));

        if body_is_int_lit && orelse_is_float_lit {
            // Promote body (int) to float with proper decimal format
            if let HirExpr::Literal(Literal::Int(i)) = body {
                let float_str = format!("{}.0", i);
                let float_lit: syn::LitFloat =
                    syn::LitFloat::new(&float_str, proc_macro2::Span::call_site());
                body_expr = syn::Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: syn::Lit::Float(float_lit),
                });
            }
        } else if body_is_float_lit && orelse_is_int_lit {
            // Promote orelse (int) to float with proper decimal format
            if let HirExpr::Literal(Literal::Int(i)) = orelse {
                let float_str = format!("{}.0", i);
                let float_lit: syn::LitFloat =
                    syn::LitFloat::new(&float_str, proc_macro2::Span::call_site());
                orelse_expr = syn::Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: syn::Lit::Float(float_lit),
                });
            }
        }

        // Python: `val if val else default` where val is String/List/Dict/Set/Optional/Int/Float
        // Without conversion: `if val` fails (expected bool, found Vec/String/etc)
        // With conversion: `if !val.is_empty()` / `if val.is_some()` / `if val != 0`
        test_expr = Self::apply_truthiness_conversion(test, test_expr, self.ctx);

        // Python: `x[0] if x else None` → Rust: `if !x.is_empty() { Some(x.get(0).cloned().unwrap()) } else { None }`
        // When else branch is None, wrap body in Some() for correct Option<T> type
        let orelse_is_none = matches!(orelse, HirExpr::Literal(Literal::None));

        // If body is an Optional type, we need to unwrap it since the condition guarantees it's Some
        // Python: `x if x is not None else 0` → Rust: `if x.is_some() { x.unwrap() } else { 0 }`
        // Python: `x if x else 0` where x is Optional → Rust: `if x.is_some() { x.unwrap() } else { 0 }`
        let body_is_optional = self.ctx.get_optional_inner_type(body).is_some();
        if body_is_optional && !orelse_is_none {
            // Check if test is checking the same variable for is_some()
            let test_checks_body = match test {
                // Pattern: `x.is_some()` (HIR has already converted `x is not None` to this)
                HirExpr::MethodCall { object, method, .. } if method == "is_some" => {
                    object.as_ref() == body
                }
                // Pattern: `x if x is not None else default` (original Python, before HIR transformation)
                HirExpr::Binary { op, left, right }
                    if matches!(op, BinOp::IsNot)
                        && matches!(right.as_ref(), HirExpr::Literal(Literal::None)) =>
                {
                    left.as_ref() == body
                }
                // Pattern: `x if x else default` (direct truthiness check on Optional)
                _ => test == body,
            };

            if test_checks_body {
                // Unwrap the Optional body since we know it's Some
                body_expr = parse_quote! { #body_expr.unwrap() };
            }
        }
        if orelse_is_none {
            Ok(parse_quote! {
                if #test_expr { Some(#body_expr) } else { None }
            })
        } else if orelse_is_string_lit {
            let orelse_str_expr: syn::Expr = parse_quote! { #orelse_expr.to_string() };
            Ok(parse_quote! {
                if #test_expr { #body_expr } else { #orelse_str_expr }
            })
        } else {
            Ok(parse_quote! {
                if #test_expr { #body_expr } else { #orelse_expr }
            })
        }
    }

}
