//! Statement code generation
//!
//! This module handles converting HIR statements to Rust token streams.
//! It includes all statement conversion helpers and the HirStmt RustCodeGen trait implementation.

use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen, ToRustExpr};
use crate::rust_gen::func_gen::infer_expr_type_with_env;
use crate::rust_gen::keywords::safe_ident; // Keyword escaping
use crate::rust_gen::type_gen::rust_type_to_syn;
use anyhow::{Result, bail};
use quote::{ToTokens, format_ident, quote};
use syn::{self, parse_quote};

/// Helper to build nested dictionary access for assignment
/// Returns (base_expr, access_chain) where access_chain is a vec of index expressions
fn extract_nested_indices_tokens(
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

/// Helper to build nested dictionary access for assignment WITHOUT clone
/// Used for dict insert operations on field accesses where we need mutable access
fn extract_nested_indices_tokens_no_clone(
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

/// Build an expression without adding .clone()
/// Used for mutation operations where we need mutable access
fn build_expr_no_clone(expr: &HirExpr) -> syn::Expr {
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

/// Check if an expression is "attribute-sourced" - either a direct attribute access
/// or a conditional expression where both branches are attribute-sourced.
fn is_attribute_sourced_expr(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { .. } => true,
        HirExpr::IfExpr { body, orelse, .. } => {
            is_attribute_sourced_expr(body) && is_attribute_sourced_expr(orelse)
        }
        _ => false,
    }
}

/// Check if an expression is an empty collection initialization.
/// These are placeholder values that will be reassigned later.
fn is_empty_collection_init_expr(expr: &HirExpr) -> bool {
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

/// Check if an expression is an enum variant (e.g., Team.Home, Play.WonPenalty).
/// Enum variants are Copy types and should not be borrowed.
fn is_enum_variant_expr(expr: &HirExpr, ctx: &CodeGenContext) -> bool {
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

/// Check if an HIR expression returns usize (needs cast to i32)
///
/// This prevents unnecessary casts like `(a: i32) as i32`.
fn expr_returns_usize(expr: &HirExpr) -> bool {
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

/// Check if a type annotation requires explicit conversion
///
/// Only adds cast when expression returns usize (from len(), count(), etc.)
fn needs_type_conversion(target_type: &Type, expr: &HirExpr) -> bool {
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

/// Apply type conversion to value expression
///
/// Wraps the expression with appropriate conversion (e.g., `as i32`)
fn apply_type_conversion(value_expr: syn::Expr, target_type: &Type) -> syn::Expr {
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

/// Infer the result type of a binary expression.
///
/// Used to track variable types for expressions like `c = a - b * 4`.
fn infer_binary_expr_type(
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

/// Check if an expression evaluates to float type (recursive helper).
fn is_expr_float(ctx: &CodeGenContext, expr: &HirExpr) -> bool {
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

/// Check if an expression already returns an Optional type.
/// Used to avoid double-wrapping in Some() when returning Optional values.
fn expr_is_optional(expr: &HirExpr, ctx: &CodeGenContext) -> bool {
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
        HirExpr::MethodCall { method, .. } => {
            matches!(method.as_str(), "get")
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

// ============================================================================
// Statement Code Generation Helpers
// Extracted to reduce complexity of HirStmt::to_rust_tokens
// ============================================================================

/// Generate code for Pass statement (no-op)
#[inline]
pub(crate) fn codegen_pass_stmt() -> Result<proc_macro2::TokenStream> {
    Ok(quote! {})
}

/// Generate code for Assert statement
#[inline]
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

/// Generate code for Break statement with optional label
#[inline]
pub(crate) fn codegen_break_stmt(label: &Option<String>) -> Result<proc_macro2::TokenStream> {
    if let Some(label_name) = label {
        let label_ident =
            syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
        Ok(quote! { break #label_ident; })
    } else {
        Ok(quote! { break; })
    }
}

/// Generate code for Continue statement with optional label
#[inline]
pub(crate) fn codegen_continue_stmt(label: &Option<String>) -> Result<proc_macro2::TokenStream> {
    if let Some(label_name) = label {
        let label_ident =
            syn::Lifetime::new(&format!("'{}", label_name), proc_macro2::Span::call_site());
        Ok(quote! { continue #label_ident; })
    } else {
        Ok(quote! { continue; })
    }
}

/// Generate code for expression statement
#[inline]
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
                        let mut arg = crate::rust_gen::argparse_transform::ArgParserArgument::new(
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
                                    crate::rust_gen::argparse_transform::ArgParserArgument::new(
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

// ============================================================================
// Statement Code Generation Helpers
// Medium-complexity handlers extracted from HirStmt::to_rust_tokens
// ============================================================================

/// Check if an expression creates an owned value (constructor call, struct literal, etc.)
/// These expressions should NOT have `&` prepended when returning.
fn expr_creates_owned_value(expr: &HirExpr) -> bool {
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

/// Generate code for Return statement with optional expression
#[inline]
pub(crate) fn codegen_return_stmt(
    expr: &Option<HirExpr>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    if let Some(e) = expr {
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

        if ctx.current_function_can_fail {
            if is_void_return && is_none_literal {
                // Void function with can_fail: return Ok(()) for `return None`
                if use_return_keyword {
                    Ok(quote! { return Ok(()); })
                } else {
                    Ok(quote! { Ok(()) })
                }
            } else if is_optional_return && !is_none_literal {
                // Check if expression is already Optional to avoid double-wrapping
                let expr_already_optional = expr_is_optional(e, ctx);
                if expr_already_optional {
                    // Expression is already Option<T>, don't wrap in Some()
                    if use_return_keyword {
                        Ok(quote! { return Ok(#expr_tokens); })
                    } else {
                        Ok(quote! { Ok(#expr_tokens) })
                    }
                } else {
                    // Wrap value in Some() for Optional return types
                    if use_return_keyword {
                        Ok(quote! { return Ok(Some(#expr_tokens)); })
                    } else {
                        Ok(quote! { Ok(Some(#expr_tokens)) })
                    }
                }
            } else if is_optional_return && is_none_literal {
                if use_return_keyword {
                    Ok(quote! { return Ok(None); })
                } else {
                    Ok(quote! { Ok(None) })
                }
            } else if use_return_keyword {
                Ok(quote! { return Ok(#expr_tokens); })
            } else {
                Ok(quote! { Ok(#expr_tokens) })
            }
        } else if is_void_return {
            // Void functions (Python -> None): no return value (non-fallible)
            if use_return_keyword {
                // Early return from void function: use empty return
                Ok(quote! { return; })
            } else {
                // Final statement in void function: use unit value ()
                Ok(quote! { () })
            }
        } else if is_optional_return && !is_none_literal {
            // Check if expression is already Optional to avoid double-wrapping
            let expr_already_optional = expr_is_optional(e, ctx);
            if expr_already_optional {
                // Expression is already Option<T>, don't wrap in Some()
                if use_return_keyword {
                    Ok(quote! { return #expr_tokens; })
                } else {
                    Ok(quote! { #expr_tokens })
                }
            } else {
                // Wrap value in Some() for Optional return types
                if use_return_keyword {
                    Ok(quote! { return Some(#expr_tokens); })
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
            Ok(quote! { return #expr_tokens; })
        } else {
            Ok(quote! { return #expr_tokens; })
        }
    } else if ctx.current_function_can_fail {
        // No expression - check if return type is Optional
        let is_optional_return =
            matches!(ctx.current_return_type.as_ref(), Some(Type::Optional(_)));
        // Always use explicit return keyword
        let use_return_keyword = true;

        if is_optional_return {
            if use_return_keyword {
                Ok(quote! { return Ok(None); })
            } else {
                Ok(quote! { Ok(None) })
            }
        } else if use_return_keyword {
            Ok(quote! { return Ok(()); })
        } else {
            Ok(quote! { Ok(()) })
        }
    } else {
        // Always use explicit return keyword
        let use_return_keyword = true;
        if use_return_keyword {
            Ok(quote! { return; })
        } else {
            // Final bare return becomes unit value (implicit)
            Ok(quote! {})
        }
    }
}

/// Generate code for While loop statement
///
#[inline]
pub(crate) fn codegen_while_stmt(
    condition: &HirExpr,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let mut cond = condition.to_rust_expr(ctx)?;

    // Convert non-boolean expressions to boolean (e.g., `while queue` where queue: VecDeque)
    cond = apply_truthiness_conversion(condition, cond, ctx);

    // Return statements inside loops must use explicit `return` keyword
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();
    let body_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    ctx.is_final_statement = saved_is_final;

    Ok(quote! {
        while #cond {
            #(#body_stmts)*
        }
    })
}

/// Generate code for Raise (exception) statement
///
#[inline]
pub(crate) fn codegen_raise_stmt(
    exception: &Option<HirExpr>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // For V1, we'll implement basic error handling
    if let Some(exc) = exception {
        // Pattern: raise argparse.ArgumentTypeError("message")
        // Extract message and use directly in panic!/error

        // Pattern: raise ValueError("message")
        // Extract the message to avoid double-wrapping ValueError::new(ValueError::new(...))
        let exc_expr = match exc {
            // Pattern 1: argparse.ArgumentTypeError(msg)
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } if matches!(object.as_ref(), HirExpr::Var(v) if v == "argparse")
                && method == "ArgumentTypeError"
                && !args.is_empty() =>
            {
                // Extract the message argument and use it directly
                args[0].to_rust_expr(ctx)?
            }
            // Pattern 2: ArgumentTypeError(msg) - if imported
            HirExpr::Call { func, args, .. } if func == "ArgumentTypeError" && !args.is_empty() => {
                args[0].to_rust_expr(ctx)?
            }
            // Pattern 3: ValueError(msg), TypeError(msg), etc. - extract message to avoid double-wrapping
            HirExpr::Call { func, args, .. }
                if (func == "ValueError"
                    || func == "TypeError"
                    || func == "KeyError"
                    || func == "IndexError"
                    || func == "ZeroDivisionError")
                    && !args.is_empty() =>
            {
                args[0].to_rust_expr(ctx)?
            }
            // Default: use exception as-is
            _ => exc.to_rust_expr(ctx)?,
        };

        let exception_type = extract_exception_type(exc);

        match exception_type.as_str() {
            "ValueError" => ctx.needs_valueerror = true,
            "ArgumentTypeError" => ctx.needs_argumenttypeerror = true,
            "ZeroDivisionError" => ctx.needs_zerodivisionerror = true,
            "IndexError" => ctx.needs_indexerror = true,
            _ => {}
        }

        if ctx.is_exception_handled(&exception_type) {
            // Exception is caught - for now use panic! (control flow jump is complex)
            // NOTE: Implement proper exception control flow to jump to handler ()
            Ok(quote! { panic!("{}", #exc_expr); })
        } else if ctx.current_function_can_fail {
            // Exception propagates to caller - use return Err
            let needs_boxing = matches!(
                ctx.current_error_type,
                Some(crate::rust_gen::context::ErrorType::DynBox)
            );

            if needs_boxing {
                // format!() returns String which doesn't implement std::error::Error
                // Need to wrap in ValueError::new(), ArgumentTypeError::new(), etc.
                if exception_type == "ValueError"
                    || exception_type == "ArgumentTypeError"
                    || exception_type == "TypeError"
                    || exception_type == "KeyError"
                    || exception_type == "IndexError"
                    || exception_type == "ZeroDivisionError"
                {
                    let exc_type = safe_ident(&exception_type);
                    Ok(quote! { return Err(Box::new(#exc_type::new(#exc_expr))); })
                } else {
                    Ok(quote! { return Err(Box::new(#exc_expr)); })
                }
            } else {
                // Without this, `return Err(format!(...))` returns String instead of ExceptionType
                if exception_type == "ValueError"
                    || exception_type == "ArgumentTypeError"
                    || exception_type == "TypeError"
                    || exception_type == "KeyError"
                    || exception_type == "IndexError"
                    || exception_type == "ZeroDivisionError"
                {
                    let exc_type = safe_ident(&exception_type);
                    Ok(quote! { return Err(#exc_type::new(#exc_expr)); })
                } else {
                    Ok(quote! { return Err(#exc_expr); })
                }
            }
        } else {
            // Function doesn't return Result - use panic!
            Ok(quote! { panic!("{}", #exc_expr); })
        }
    } else {
        // Re-raise or bare raise - use generic error
        Ok(quote! { return Err("Exception raised".into()); })
    }
}

///
/// # Complexity
/// 2 (match + clone)
fn extract_exception_type(exception: &HirExpr) -> String {
    match exception {
        HirExpr::Call { func, .. } => func.clone(),
        HirExpr::Var(name) => name.clone(),
        HirExpr::MethodCall { method, .. } => method.clone(),
        _ => "Exception".to_string(),
    }
}

/// Generate code for With (context manager) statement
#[inline]
pub(crate) fn codegen_with_stmt(
    context: &HirExpr,
    target: &Option<String>,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
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

// ============================================================================
// Statement Code Generation Helpers
// Complex handlers extracted from HirStmt::to_rust_tokens
// ============================================================================

/// Extract variable name from `var is None` or `var is not None` patterns.
/// Returns (var_name, is_not_none) where is_not_none is true for `is not None`.
fn extract_none_check(condition: &HirExpr) -> Option<(String, bool)> {
    if let HirExpr::Binary { op, left, right } = condition {
        // Check for: var is not None  OR  None is not var
        if *op == BinOp::IsNot {
            if let (HirExpr::Var(var_name), HirExpr::Literal(Literal::None)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), true));
            }
            if let (HirExpr::Literal(Literal::None), HirExpr::Var(var_name)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), true));
            }
        }
        // Check for: var is None  OR  None is var
        if *op == BinOp::Is {
            if let (HirExpr::Var(var_name), HirExpr::Literal(Literal::None)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), false));
            }
            if let (HirExpr::Literal(Literal::None), HirExpr::Var(var_name)) =
                (left.as_ref(), right.as_ref())
            {
                return Some((var_name.clone(), false));
            }
        }
    }
    None
}

/// Generate `if let Some(var) = var { ... }` for type narrowing of Optional variables.
/// This is used for `if var is not None:` patterns.
fn codegen_if_let_some(
    var_name: String,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let var_ident = safe_ident(&var_name);

    // Temporarily remove from optional_vars so inner code doesn't double-unwrap
    ctx.optional_vars.remove(&var_name);

    ctx.enter_scope();
    // Declare the narrowed (unwrapped) variable in the inner scope
    ctx.declare_var(&var_name);

    let then_stmts: Vec<_> = then_body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Restore optional_vars status
    ctx.optional_vars.insert(var_name.clone());

    if let Some(else_stmts) = else_body {
        ctx.enter_scope();
        let else_tokens: Vec<_> = else_stmts
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        ctx.exit_scope();
        Ok(quote! {
            if let Some(mut #var_ident) = #var_ident {
                #(#then_stmts)*
            } else {
                #(#else_tokens)*
            }
        })
    } else {
        Ok(quote! {
            if let Some(mut #var_ident) = #var_ident {
                #(#then_stmts)*
            }
        })
    }
}

/// Apply Python truthiness semantics to an Optional type in a condition.
///
/// Python treats `None` as falsy, and for container types inside Optional,
/// empty containers are also falsy:
/// - Optional[str]: `opt.as_ref().is_some_and(|s| !s.is_empty())`
/// - Optional[List]: `opt.as_ref().is_some_and(|v| !v.is_empty())`
/// - Optional[int]: `opt.is_some_and(|n| n != 0)`
/// - Optional[float]: `opt.is_some_and(|n| n != 0.0)`
/// - Optional[T] (other): `opt.is_some()`
fn apply_optional_truthiness(field_type: &Type, cond_expr: syn::Expr) -> syn::Expr {
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

/// Apply Python truthiness conversion to a condition expression
///
/// In Python, any value can be used in a boolean context. This function
/// converts non-boolean expressions to boolean using Python semantics:
/// - String: !expr.is_empty()
/// - List/Dict/Set: !expr.is_empty()
/// - Optional: expr.is_some()
/// - Int: expr != 0
/// - Float: expr != 0.0
/// - Bool: expr (no conversion)
///
/// #
/// Fixes: `if val` where `val: String` failing to compile
fn apply_truthiness_conversion(
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

///
/// Checks for common patterns that return Option:
/// - Calls to methods ending with .ok() (Result → Option conversion)
/// - Calls to .get() methods (dict/map lookups)
/// - os.environ.get() / std::env::var().ok()
fn looks_like_option_expr(expr: &HirExpr) -> bool {
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

///
/// Returns a set of variable names that are assigned (not reassigned) in the block.
/// Only captures simple symbol assignments like `x = value`, not `x[i] = value` or `x.attr = value`.
///
/// # Complexity
/// 4 (recursive traversal with set operations)
fn extract_assigned_symbols(stmts: &[HirStmt]) -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let mut symbols = HashSet::new();

    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                ..
            } => {
                symbols.insert(name.clone());
            }
            // Recursively check nested if/else, while, for, try blocks
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                symbols.extend(extract_assigned_symbols(then_body));
                if let Some(else_stmts) = else_body {
                    symbols.extend(extract_assigned_symbols(else_stmts));
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                symbols.extend(extract_assigned_symbols(body));
            }
            HirStmt::Try {
                body,
                handlers,
                finalbody,
                ..
            } => {
                symbols.extend(extract_assigned_symbols(body));
                for handler in handlers {
                    symbols.extend(extract_assigned_symbols(&handler.body));
                }
                if let Some(finally) = finalbody {
                    symbols.extend(extract_assigned_symbols(finally));
                }
            }
            _ => {}
        }
    }

    symbols
}

/// Generate code for If statement with optional else clause
///
/// Variables assigned in BOTH if and else branches are hoisted before the if statement.
#[inline]
pub(crate) fn codegen_if_stmt(
    condition: &HirExpr,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use std::collections::HashSet;

    if ctx.argparser_tracker.has_subcommands() {
        if let Some(match_stmt) =
            try_generate_subcommand_match(condition, then_body, else_body, ctx)?
        {
            return Ok(match_stmt);
        }
    }

    // Check for `if var is not None:` pattern - use `if let Some(var) = var` for type narrowing
    if let Some((var_name, is_not_none)) = extract_none_check(condition) {
        if is_not_none && ctx.optional_vars.contains(&var_name) {
            return codegen_if_let_some(var_name, then_body, else_body, ctx);
        }
    }

    let mut cond = condition.to_rust_expr(ctx)?;

    // When a function returns Result<bool, E> (like is_even with modulo),
    // we need to unwrap it for use in boolean context
    // Check if the condition is a Call to a function that returns Result<bool>
    if let HirExpr::Call { func, .. } = condition {
        if ctx.result_bool_functions.contains(func) {
            // This function returns Result<bool>, so unwrap it
            // Use .unwrap_or(false) to handle potential errors gracefully
            cond = parse_quote! { #cond.unwrap_or(false) };
        }
    }

    // Convert non-boolean expressions to boolean (e.g., `if val` where val: String)
    cond = apply_truthiness_conversion(condition, cond, ctx);

    let hoisted_vars: HashSet<String> = if let Some(else_stmts) = else_body {
        let then_vars = extract_assigned_symbols(then_body);
        let else_vars = extract_assigned_symbols(else_stmts);
        then_vars.intersection(&else_vars).cloned().collect()
    } else {
        HashSet::new()
    };

    let mut hoisted_decls = Vec::new();
    for var_name in &hoisted_vars {
        if ctx.is_declared(var_name) {
            continue;
        }

        // Find the variable's type from the first assignment in either branch
        let var_type = find_variable_type(var_name, then_body).or_else(|| {
            if let Some(else_stmts) = else_body {
                find_variable_type(var_name, else_stmts)
            } else {
                None
            }
        });

        let var_ident = safe_ident(var_name);
        let needs_mut = ctx.mutable_vars.contains(var_name);

        if let Some(ty) = var_type {
            let rust_type = ctx.type_mapper.map_type(&ty);
            let syn_type = rust_type_to_syn(&rust_type)?;
            if needs_mut {
                hoisted_decls.push(quote! { let mut #var_ident: #syn_type; });
            } else {
                hoisted_decls.push(quote! { let #var_ident: #syn_type; });
            }
        } else {
            // No type annotation - use type inference placeholder
            // Rust will infer the type from the assignments in the branches
            if needs_mut {
                hoisted_decls.push(quote! { let mut #var_ident; });
            } else {
                hoisted_decls.push(quote! { let #var_ident; });
            }
        }

        // Mark variable as declared so assignments use `var = value` not `let var = value`
        ctx.declare_var(var_name);
    }

    ctx.enter_scope();
    let then_stmts: Vec<_> = then_body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    if let Some(else_stmts) = else_body {
        ctx.enter_scope();
        let else_tokens: Vec<_> = else_stmts
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        ctx.exit_scope();
        Ok(quote! {
            #(#hoisted_decls)*
            if #cond {
                #(#then_stmts)*
            } else {
                #(#else_tokens)*
            }
        })
    } else {
        Ok(quote! {
            if #cond {
                #(#then_stmts)*
            }
        })
    }
}

///
/// Searches for the first Assign statement that assigns to the given variable
/// and returns its type annotation if present.
///
/// # Complexity
/// 3 (linear search with recursive check)
fn find_variable_type(var_name: &str, stmts: &[HirStmt]) -> Option<Type> {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                type_annotation,
                ..
            } if name == var_name => {
                return type_annotation.clone();
            }
            _ => {}
        }
    }
    None
}

/// Check if a variable is used in an expression
fn is_var_used_in_expr(var_name: &str, expr: &HirExpr) -> bool {
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

/// Check if a variable is used in an assignment target
fn is_var_used_in_assign_target(var_name: &str, target: &AssignTarget) -> bool {
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
    }
}

/// Check if a variable is used in a statement
fn is_var_used_in_stmt(var_name: &str, stmt: &HirStmt) -> bool {
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

/// Generate field access expression without adding .clone()
/// Used for iteration contexts where we want to borrow, not clone
fn generate_field_access_without_clone(
    iter: &HirExpr,
    ctx: &mut CodeGenContext,
) -> Result<syn::Expr> {
    match iter {
        HirExpr::Attribute { value, attr } => {
            let value_expr = generate_field_access_without_clone(value, ctx)?;
            let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
            Ok(parse_quote! { #value_expr.#attr_ident })
        }
        HirExpr::Var(name) => {
            let ident = safe_ident(name);
            Ok(parse_quote! { #ident })
        }
        HirExpr::Call { func, args, .. } if func == "enumerate" || func == "reversed" => {
            // Handle enumerate(field) and reversed(field)
            if !args.is_empty() {
                let inner = generate_field_access_without_clone(&args[0], ctx)?;
                if func == "enumerate" {
                    Ok(parse_quote! { #inner.iter().enumerate() })
                } else {
                    Ok(parse_quote! { #inner.iter().rev() })
                }
            } else {
                iter.to_rust_expr(ctx)
            }
        }
        _ => iter.to_rust_expr(ctx),
    }
}

/// Check if an iterator expression is accessing a field on a parameter
/// Returns Some((root_var, is_field_access)) if it's a field access pattern
fn is_field_access_iter(iter: &HirExpr) -> Option<(String, bool)> {
    match iter {
        // Direct field access: state.items
        HirExpr::Attribute { value, .. } => {
            if let Some(root_var) = crate::expr_utils::extract_root_var(value) {
                Some((root_var, true))
            } else {
                None
            }
        }
        // enumerate(state.items) or reversed(state.items)
        HirExpr::Call { func, args, .. }
            if (func == "enumerate" || func == "reversed") && !args.is_empty() =>
        {
            if let HirExpr::Attribute { value, .. } = &args[0] {
                if let Some(root_var) = crate::expr_utils::extract_root_var(value) {
                    Some((root_var, true))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if loop body mutates the loop variable (indicating need for &mut)
fn does_loop_body_mutate_items(target: &AssignTarget, body: &[HirStmt]) -> bool {
    // Get the loop variable name
    let loop_var = match target {
        AssignTarget::Symbol(name) => name,
        AssignTarget::Tuple(targets) => {
            // For tuples like (i, item), check the second element
            if targets.len() >= 2 {
                if let AssignTarget::Symbol(name) = &targets[1] {
                    name
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        _ => return false,
    };

    // Check if the loop variable is mutated in the body
    for stmt in body {
        if is_loop_var_mutated(loop_var, stmt) {
            return true;
        }
    }
    false
}

/// Helper to check if a loop variable is mutated in a statement
fn is_loop_var_mutated(var_name: &str, stmt: &HirStmt) -> bool {
    match stmt {
        // Direct assignment to loop variable or its fields
        HirStmt::Assign { target, .. } => {
            match target {
                AssignTarget::Symbol(name) if name == var_name => true,
                AssignTarget::Attribute { value, .. } => {
                    // Check if assigning to var_name.field (including nested like var_name.a.b.c)
                    crate::expr_utils::extract_root_var(value).is_some_and(|root| root == var_name)
                }
                AssignTarget::Index { base, .. } => {
                    // Check if assigning to var_name[index] (including nested like var_name.a[i])
                    crate::expr_utils::extract_root_var(base).is_some_and(|root| root == var_name)
                }
                _ => false,
            }
        }
        // Check nested statements
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            then_body.iter().any(|s| is_loop_var_mutated(var_name, s))
                || else_body
                    .as_ref()
                    .is_some_and(|body| body.iter().any(|s| is_loop_var_mutated(var_name, s)))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            body.iter().any(|s| is_loop_var_mutated(var_name, s))
        }
        _ => false,
    }
}

/// Generate code for For loop statement
#[inline]
pub(crate) fn codegen_for_stmt(
    target: &AssignTarget,
    iter: &HirExpr,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // If unused, prefix with _ to avoid unused variable warnings with -D warnings

    // Check if loop variable is mutated to determine if we need `mut` keyword
    let needs_mut_pattern = does_loop_body_mutate_items(target, body);

    // Generate target pattern based on AssignTarget type
    let target_pattern: syn::Pat = match target {
        AssignTarget::Symbol(name) => {
            // Check if this variable is used in the loop body
            let is_used = body.iter().any(|stmt| is_var_used_in_stmt(name, stmt));

            // If unused, prefix with underscore
            let var_name = if is_used {
                name.clone()
            } else {
                format!("_{}", name)
            };

            let ident = safe_ident(&var_name);
            if needs_mut_pattern {
                parse_quote! { mut #ident }
            } else {
                parse_quote! { #ident }
            }
        }
        AssignTarget::Tuple(targets) => {
            // For tuple unpacking, check each variable individually
            let idents: Vec<syn::Ident> = targets
                .iter()
                .map(|t| match t {
                    AssignTarget::Symbol(s) => {
                        // Check if this specific tuple element is used
                        let is_used = body.iter().any(|stmt| is_var_used_in_stmt(s, stmt));
                        let var_name = if is_used {
                            s.clone()
                        } else {
                            format!("_{}", s)
                        };
                        safe_ident(&var_name)
                    }
                    _ => panic!("Nested tuple unpacking not supported in for loops"),
                })
                .collect();
            if needs_mut_pattern {
                // Add mut to each element of the tuple
                parse_quote! { (mut #(#idents),*) }
            } else {
                parse_quote! { (#(#idents),*) }
            }
        }
        _ => bail!("Unsupported for loop target type"),
    };

    // When iterating over field accesses (e.g., state.items), we MUST use borrows
    // because Rust doesn't allow moving out of struct fields.
    // Determine whether to use & or &mut based on loop body mutations.
    let (needs_field_borrow, is_special_call) = if let Some((_root_var, is_field)) =
        is_field_access_iter(iter)
    {
        if is_field {
            // This is a field access - we need borrowing
            // Determine if we need mutable or immutable borrow
            let needs_mut_borrow = does_loop_body_mutate_items(target, body);

            // Check if this is a special function call (enumerate, reversed)
            let is_special = matches!(iter, HirExpr::Call { func, .. } if func == "enumerate" || func == "reversed");

            (Some(needs_mut_borrow), is_special)
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    // Convert tuple to array for iteration (tuples aren't directly iterable in Rust)
    // For field accesses that will be borrowed, generate without .clone()
    let mut iter_expr = if let HirExpr::Tuple(elts) = iter {
        // Track loop variable as &str if iterating over string literals
        // This is needed to add .to_string() when passing to functions expecting String
        let is_string_tuple = elts
            .iter()
            .all(|e| matches!(e, HirExpr::Literal(crate::hir::Literal::String(_))));
        if is_string_tuple {
            if let AssignTarget::Symbol(var_name) = target {
                ctx.tuple_iter_vars.insert(var_name.clone());
            }
        }
        let elt_exprs: Vec<syn::Expr> = elts
            .iter()
            .map(|e| e.to_rust_expr(ctx))
            .collect::<Result<Vec<_>>>()?;
        parse_quote! { [#(#elt_exprs),*] }
    } else if needs_field_borrow.is_some() {
        // For field accesses being iterated, generate without .clone() since we'll borrow
        generate_field_access_without_clone(iter, ctx)?
    } else {
        // For simple variables that will get .iter().cloned(), prevent initial clone
        // to avoid redundant `var.clone().iter().cloned()` pattern
        let saved_prevent_clone = ctx.prevent_clone;
        if matches!(iter, HirExpr::Var(_)) {
            ctx.prevent_clone = true;
        }
        let expr = iter.to_rust_expr(ctx)?;
        ctx.prevent_clone = saved_prevent_clone;
        expr
    };

    // Check if the iterator is an Optional type (e.g., Optional[List[int]])
    // If so, unwrap it before iterating
    let iter_is_optional = expr_is_optional(iter, ctx);
    if iter_is_optional {
        iter_expr = parse_quote! { #iter_expr.as_ref().unwrap() };
    }

    // Python: for line in sys.stdin:
    // Rust: for line in std::io::stdin().lock().lines()
    let is_stdin_iter = matches!(iter, HirExpr::Attribute { value, attr }
        if matches!(&**value, HirExpr::Var(m) if m == "sys") && attr == "stdin");

    // Python: for line in f: (where f = open(...))
    // Rust: use BufReader for efficient line-by-line reading
    // Check if this variable might be a File object
    // Heuristic: variables named 'f', 'file', 'input', 'output', or ending in '_file'
    let is_file_iter = if let HirExpr::Var(var_name) = iter {
        var_name == "f"
            || var_name == "file"
            || var_name == "input"
            || var_name == "output"
            || var_name.ends_with("_file")
            || var_name.starts_with("file_")
    } else {
        false
    };

    if is_stdin_iter {
        // Wrap stdin with .lines() to get line iterator
        // Stdin::lines() method provides buffered line-by-line reading
        // Returns Iterator<Item = Result<String, io::Error>>
        // We map to unwrap_or_default() to handle errors gracefully
        iter_expr = parse_quote! { #iter_expr.lines().map(|l| l.unwrap_or_default()) };
    } else if is_file_iter {
        // This is the idiomatic Rust way to iterate over file lines
        // Method call syntax (.lines()) is preferred over trait syntax (BufRead::lines())
        iter_expr = parse_quote! {
            std::io::BufReader::new(#iter_expr).lines()
                .map(|l| l.unwrap_or_default())
        };
    }

    // Check if variable name suggests CSV reader (heuristic-based)
    let is_csv_reader = if let HirExpr::Var(var_name) = iter {
        var_name == "reader"
            || var_name.contains("csv")
            || var_name.ends_with("_reader")
            || var_name.starts_with("reader_")
    } else {
        false
    };

    // Track if CSV pattern yields Results (need to unwrap in loop)
    let mut csv_yields_results = false;

    if !is_stdin_iter && !is_file_iter && is_csv_reader {
        // Try to apply CSV iteration mapping from stdlib_mappings
        // This transforms: for row in reader
        // Into: for result in reader.deserialize::<HashMap<String, String>>()
        if let Some(pattern) = ctx
            .stdlib_mappings
            .get_iteration_pattern("csv", "DictReader")
        {
            // Check if pattern yields Results
            if let crate::stdlib_mappings::RustPattern::IterationPattern {
                yields_results, ..
            } = pattern
            {
                csv_yields_results = *yields_results;
            }

            let rust_code =
                pattern.generate_rust_code(&iter_expr.to_token_stream().to_string(), &[]);
            if let Ok(expr) = syn::parse_str::<syn::Expr>(&rust_code) {
                // Set needs_csv flag
                ctx.needs_csv = true;
                // Wrap in iteration that handles Results
                iter_expr = expr;
            }
        }
    }

    // If we determined that a borrow is needed (field access on a parameter),
    // wrap the iterator expression with & or &mut, OR use .iter()/.iter_mut() for special calls
    // Skip this if we already unwrapped an Optional, since .as_ref().unwrap() already returns a reference
    if let Some(needs_mut) = needs_field_borrow {
        if !iter_is_optional {
            if is_special_call {
                // For enumerate/reversed, we need to regenerate using .iter()/.iter_mut()
                // Extract the field access expression
                match iter {
                    HirExpr::Call { func, args, .. } if func == "enumerate" && !args.is_empty() => {
                        // Get the field access from inside enumerate - without clone
                        let field_expr = generate_field_access_without_clone(&args[0], ctx)?;
                        if needs_mut {
                            iter_expr = parse_quote! { #field_expr.iter_mut().enumerate() };
                        } else {
                            iter_expr = parse_quote! { #field_expr.iter().enumerate() };
                        }
                    }
                    HirExpr::Call { func, args, .. } if func == "reversed" && !args.is_empty() => {
                        // Get the field access from inside reversed - without clone
                        let field_expr = generate_field_access_without_clone(&args[0], ctx)?;
                        if needs_mut {
                            iter_expr = parse_quote! { #field_expr.iter_mut().rev() };
                        } else {
                            iter_expr = parse_quote! { #field_expr.iter().rev() };
                        }
                    }
                    _ => {}
                }
            } else {
                // For plain field access, just wrap with & or &mut
                if needs_mut {
                    iter_expr = parse_quote! { &mut #iter_expr };
                } else {
                    iter_expr = parse_quote! { &#iter_expr };
                }
            }
        }
    }

    // Check if we're iterating over a borrowed collection
    // If iter is a simple variable that refers to a borrowed collection (e.g., &Vec<T>),
    // we need to add .iter() to properly iterate over it
    // Skip this for stdin/file/csv iterators which are already properly wrapped
    // Also skip for field access iterators that we just added borrows to
    if !is_stdin_iter
        && !is_file_iter
        && !is_csv_reader
        && needs_field_borrow.is_none()
        && !is_special_call
    {
        if let HirExpr::Var(var_name) = iter {
            // This is more reliable than name heuristics
            let is_string_type = ctx
                .var_types
                .get(var_name)
                .is_some_and(|t| matches!(t, Type::String));

            // Strings use .chars() instead of .iter().cloned()
            let is_string_name = {
                let n = var_name.as_str();
                // Exact matches (singular forms only)
                (n == "s" || n == "string" || n == "text" || n == "word" || n == "line"
                || n == "char" || n == "character")
            // Prefixes (but not if followed by 's' for plural)
            || (n.starts_with("str") && !n.starts_with("strings"))
            || (n.starts_with("word") && !n.starts_with("words"))
            || (n.starts_with("text") && !n.starts_with("texts"))
            // Suffixes (but exclude plurals)
            || (n.ends_with("_str") && !n.ends_with("_strs"))
            || (n.ends_with("_string") && !n.ends_with("_strings"))
            || (n.ends_with("_word") && !n.ends_with("_words"))
            || (n.ends_with("_text") && !n.ends_with("_texts"))
            };

            if is_string_type || is_string_name {
                // For strings, use .chars() to iterate over characters
                iter_expr = parse_quote! { #iter_expr.chars() };
            } else {
                // For collections, use .iter().cloned()
                // This handles both Copy types (int, float, bool) and Clone types (String, Vec, etc.)
                // For Copy types, .cloned() is optimized to a simple bit-copy by the compiler.
                // For Clone types, it calls .clone() which is correct for Rust.
                // This matches Python semantics where loop variables are values, not references.
                iter_expr = parse_quote! { #iter_expr.iter().cloned() };
            }
        }
    }

    // Return statements inside loops must use explicit `return` keyword
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();

    // Extract element type from iterator and add to var_types
    let element_type = match iter {
        HirExpr::Var(var_name) => {
            // Simple case: for x in items
            // Look up items type, extract element type
            ctx.var_types.get(var_name).and_then(|t| match t {
                Type::List(elem_t) => Some(*elem_t.clone()),
                Type::Set(elem_t) => Some(*elem_t.clone()),
                Type::Dict(key_t, _) => Some(*key_t.clone()), // dict iteration yields keys
                _ => None,
            })
        }
        HirExpr::Call { func, args, .. } if func == "enumerate" => {
            // enumerate(items) yields (int, elem_type)
            if let Some(HirExpr::Var(var_name)) = args.first() {
                ctx.var_types.get(var_name).and_then(|t| match t {
                    Type::List(elem_t) => Some(Type::Tuple(vec![Type::Int, *elem_t.clone()])),
                    Type::Set(elem_t) => Some(Type::Tuple(vec![Type::Int, *elem_t.clone()])),
                    _ => None,
                })
            } else {
                None
            }
        }
        _ => None,
    };

    // Declare all variables from the target pattern and set their types
    // Also track if they shadow ref params (for proper dereference handling)
    match (target, element_type) {
        (AssignTarget::Symbol(name), Some(elem_type)) => {
            ctx.declare_var(name);
            ctx.var_types.insert(name.clone(), elem_type);
            // Track if this for-loop variable shadows a ref param
            if ctx.current_func_ref_params.contains(name) {
                ctx.shadowed_ref_params.insert(name.clone());
            }
        }
        (AssignTarget::Symbol(name), None) => {
            ctx.declare_var(name);
            // Track if this for-loop variable shadows a ref param
            if ctx.current_func_ref_params.contains(name) {
                ctx.shadowed_ref_params.insert(name.clone());
            }
        }
        (AssignTarget::Tuple(targets), Some(Type::Tuple(elem_types)))
            if targets.len() == elem_types.len() =>
        {
            // Tuple unpacking with type info: (i, val) from enumerate
            for (t, typ) in targets.iter().zip(elem_types.iter()) {
                if let AssignTarget::Symbol(s) = t {
                    ctx.declare_var(s);
                    ctx.var_types.insert(s.clone(), typ.clone());
                    // Track if this for-loop variable shadows a ref param
                    if ctx.current_func_ref_params.contains(s) {
                        ctx.shadowed_ref_params.insert(s.clone());
                    }
                }
            }
        }
        (AssignTarget::Tuple(targets), _) => {
            // Tuple unpacking without type info
            for t in targets {
                if let AssignTarget::Symbol(s) = t {
                    ctx.declare_var(s);
                    // Track if this for-loop variable shadows a ref param
                    if ctx.current_func_ref_params.contains(s) {
                        ctx.shadowed_ref_params.insert(s.clone());
                    }
                }
            }
        }
        _ => {}
    }

    // Collect variables we added to shadowed_ref_params so we can remove them after scope exit
    let shadowed_in_this_scope: Vec<String> = match target {
        AssignTarget::Symbol(name) if ctx.current_func_ref_params.contains(name) => {
            vec![name.clone()]
        }
        AssignTarget::Tuple(targets) => targets
            .iter()
            .filter_map(|t| {
                if let AssignTarget::Symbol(s) = t {
                    if ctx.current_func_ref_params.contains(s) {
                        Some(s.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    };

    let body_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Remove shadowed variables when exiting scope
    for var in &shadowed_in_this_scope {
        ctx.shadowed_ref_params.remove(var);
    }

    ctx.is_final_statement = saved_is_final;

    // When iterating with enumerate(), the first element of the tuple is usize
    // If we're destructuring a tuple and the iterator is enumerate(), cast the first variable to i32
    let needs_enumerate_cast = matches!(iter, HirExpr::Call { func, .. } if func == "enumerate")
        && matches!(target, AssignTarget::Tuple(targets) if !targets.is_empty());

    // When iterating over strings with .chars(), convert char to String for HashMap<String, _> compatibility
    // Check if we're iterating over a string (will use .chars()) AND target is a simple symbol
    let needs_char_to_string = matches!(iter, HirExpr::Var(name) if {
        let n = name.as_str();
        (n == "s" || n == "string" || n == "text" || n == "word" || n == "line")
            || (n.starts_with("str") && !n.starts_with("strings"))
            || (n.starts_with("word") && !n.starts_with("words"))
            || (n.starts_with("text") && !n.starts_with("texts"))
            || (n.ends_with("_str") && !n.ends_with("_strs"))
            || (n.ends_with("_string") && !n.ends_with("_strings"))
            || (n.ends_with("_word") && !n.ends_with("_words"))
            || (n.ends_with("_text") && !n.ends_with("_texts"))
    }) && matches!(target, AssignTarget::Symbol(_));

    // When iterating over field accesses with .iter(), loop variables are references
    // We need to dereference them for use in value comparisons/assignments
    // This applies when: needs_field_borrow is Some(false) (immutable borrow)
    // For mutable iteration (Some(true)), we DON'T add clone - we modify in place
    let needs_deref = matches!(needs_field_borrow, Some(false));

    if needs_enumerate_cast {
        // Get the first variable name from the tuple pattern (the index from enumerate)
        if let AssignTarget::Tuple(targets) = target {
            if let Some(AssignTarget::Symbol(index_var)) = targets.first() {
                // If unused, it will be prefixed with _ in target_pattern, so no cast needed
                let is_index_used = body.iter().any(|stmt| is_var_used_in_stmt(index_var, stmt));

                // Also check if there's a value variable (second element) that needs dereferencing
                // Use .clone() instead of * because it works for both Copy and non-Copy types
                let value_deref_stmt = if needs_deref && targets.len() >= 2 {
                    if let Some(AssignTarget::Symbol(value_var)) = targets.get(1) {
                        let is_value_used =
                            body.iter().any(|stmt| is_var_used_in_stmt(value_var, stmt));
                        if is_value_used {
                            let value_ident = safe_ident(value_var);
                            Some(quote! { let #value_ident = #value_ident.clone(); })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if is_index_used {
                    // Add a cast statement at the beginning of the loop body
                    let index_ident = safe_ident(index_var);
                    if let Some(deref_stmt) = value_deref_stmt {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                let #index_ident = #index_ident as i32;
                                #deref_stmt
                                #(#body_stmts)*
                            }
                        })
                    } else {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                let #index_ident = #index_ident as i32;
                                #(#body_stmts)*
                            }
                        })
                    }
                } else {
                    // Index is unused - don't generate cast statement
                    if let Some(deref_stmt) = value_deref_stmt {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                #deref_stmt
                                #(#body_stmts)*
                            }
                        })
                    } else {
                        Ok(quote! {
                            for #target_pattern in #iter_expr {
                                #(#body_stmts)*
                            }
                        })
                    }
                }
            } else {
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        #(#body_stmts)*
                    }
                })
            }
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if needs_char_to_string {
        // Python: for char in s: freq[char] = ...
        // Rust: for _char in s.chars() { let char = _char.to_string(); ... }
        if let AssignTarget::Symbol(var_name) = target {
            let var_ident = safe_ident(var_name);
            let temp_ident = safe_ident(&format!("_{}", var_name));
            Ok(quote! {
                for #temp_ident in #iter_expr {
                    let #var_ident = #temp_ident.to_string();
                    #(#body_stmts)*
                }
            })
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if csv_yields_results {
        // Python: for row in reader
        // Rust: for result in reader.deserialize() { let row = result?; ... }
        if let AssignTarget::Symbol(var_name) = target {
            let var_ident = safe_ident(var_name);
            let result_ident = safe_ident("result");
            Ok(quote! {
                for #result_ident in #iter_expr {
                    let #var_ident = #result_ident?;
                    #(#body_stmts)*
                }
            })
        } else {
            // Fallback if target is not a simple symbol
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else if needs_deref {
        // Iterating over field access with references - need to dereference
        if let AssignTarget::Symbol(var_name) = target {
            let is_used = body.iter().any(|stmt| is_var_used_in_stmt(var_name, stmt));
            if is_used {
                let var_ident = safe_ident(var_name);
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        let #var_ident = #var_ident.clone();
                        #(#body_stmts)*
                    }
                })
            } else {
                Ok(quote! {
                    for #target_pattern in #iter_expr {
                        #(#body_stmts)*
                    }
                })
            }
        } else {
            Ok(quote! {
                for #target_pattern in #iter_expr {
                    #(#body_stmts)*
                }
            })
        }
    } else {
        Ok(quote! {
            for #target_pattern in #iter_expr {
                #(#body_stmts)*
            }
        })
    }
}

/// Check if this is a dict augmented assignment pattern (dict[key] op= value)
/// Returns true if target is Index and value is Binary with left being an Index to same location
fn is_dict_augassign_pattern(target: &AssignTarget, value: &HirExpr) -> bool {
    if let AssignTarget::Index {
        base: target_base,
        index: target_index,
    } = target
    {
        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Index {
                base: value_base,
                index: value_index,
            } = left.as_ref()
            {
                // Check if both indices refer to the same dict[key] location
                // Simple heuristic: compare base and index expressions
                // (This is simplified - a full solution would do deeper structural comparison)
                return matches!((target_base.as_ref(), value_base.as_ref()),
                    (HirExpr::Var(t_var), HirExpr::Var(v_var)) if t_var == v_var)
                    && matches!((target_index.as_ref(), value_index.as_ref()),
                        (HirExpr::Var(t_idx), HirExpr::Var(v_idx)) if t_idx == v_idx);
            }
        }
    }
    false
}

/// Check if this is an augmented assignment on an Optional field (obj.field op= value)
/// Returns true if target is an Attribute and value is Binary with left being same Attribute
fn is_optional_attr_augassign_pattern(
    target: &AssignTarget,
    value: &HirExpr,
    ctx: &CodeGenContext,
) -> bool {
    if let AssignTarget::Attribute {
        value: target_base,
        attr: target_attr,
    } = target
    {
        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Attribute {
                value: left_base,
                attr: left_attr,
            } = left.as_ref()
            {
                // Check if target and left refer to the same attribute
                if target_attr == left_attr {
                    // Check if the bases refer to the same variable
                    if let (HirExpr::Var(t_var), HirExpr::Var(l_var)) =
                        (target_base.as_ref(), left_base.as_ref())
                    {
                        if t_var == l_var {
                            // Now check if this attribute is Optional
                            return expr_is_optional(left.as_ref(), ctx);
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if this is an augmented assignment on an Optional variable (var op= value)
/// Returns true if target is a Symbol and value is Binary with left being same variable and target is Optional
fn is_optional_var_augassign_pattern(
    target: &AssignTarget,
    value: &HirExpr,
    ctx: &CodeGenContext,
) -> bool {
    if let AssignTarget::Symbol(target_var) = target {
        if let HirExpr::Binary { left, .. } = value {
            if let HirExpr::Var(left_var) = left.as_ref() {
                // Check if target and left refer to the same variable
                if target_var == left_var {
                    // Check if this variable is Optional
                    return matches!(ctx.var_types.get(target_var), Some(Type::Optional(_)));
                }
            }
        }
    }
    false
}

/// Generate code for Assign statement (variable/index/attribute/tuple assignment)
#[inline]
pub(crate) fn codegen_assign_stmt(
    target: &AssignTarget,
    value: &HirExpr,
    type_annotation: &Option<Type>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // When we have subcommands, assignments like `_cse_temp_0 = args.command == "clone"`
    // would try to compare Commands enum to string (won't compile).
    // Transform into a match expression that returns bool:
    // let _cse_temp_0 = matches!(args.command, Commands::Clone { .. });
    if ctx.argparser_tracker.has_subcommands() {
        if let Some(cmd_name) = is_subcommand_check(value) {
            if let AssignTarget::Symbol(cse_var) = target {
                use quote::{format_ident, quote};
                let variant_name = format_ident!("{}", to_pascal_case_subcommand(&cmd_name));
                let var_ident = safe_ident(cse_var);

                return Ok(quote! {
                    let #var_ident = matches!(args.command, Commands::#variant_name { .. });
                });
            }
        }
    }

    // Pattern 1: parser = argparse.ArgumentParser(...) [MethodCall with object=argparse]
    // Pattern 2: args = parser.parse_args() [MethodCall with object=parser]
    if let AssignTarget::Symbol(var_name) = target {
        if let HirExpr::MethodCall {
            method,
            object,
            args,
            kwargs,
            ..
        } = value
        {
            // Pattern 1: ArgumentParser constructor
            if method == "ArgumentParser" {
                if let HirExpr::Var(module_name) = object.as_ref() {
                    if module_name == "argparse" {
                        // Register this as an ArgumentParser instance
                        let mut info = crate::rust_gen::argparse_transform::ArgParserInfo::new(
                            var_name.clone(),
                        );

                        // Extract description and epilog from kwargs
                        for (key, value_expr) in kwargs {
                            if key == "description" {
                                if let HirExpr::Literal(crate::hir::Literal::String(s)) = value_expr
                                {
                                    info.description = Some(s.clone());
                                }
                            } else if key == "epilog" {
                                if let HirExpr::Literal(crate::hir::Literal::String(s)) = value_expr
                                {
                                    info.epilog = Some(s.clone());
                                }
                            }
                        }

                        ctx.argparser_tracker
                            .register_parser(var_name.clone(), info);

                        // Skip generating this statement - it will be replaced by Args struct
                        return Ok(quote! {});
                    }
                }
            }

            // Pattern 2: args = parser.parse_args()
            if method == "parse_args" {
                if let HirExpr::Var(parser_var) = object.as_ref() {
                    // Check if this parser is tracked
                    if let Some(parser_info) = ctx.argparser_tracker.get_parser_mut(parser_var) {
                        // Set the args variable name
                        parser_info.set_args_var(var_name.clone());

                        // Generate Args::parse() instead
                        let var_ident = syn::Ident::new(var_name, proc_macro2::Span::call_site());
                        return Ok(quote! {
                            let #var_ident = Args::parse();
                        });
                    }
                }
            }

            // Pattern: group = parser.add_argument_group(...)
            //      OR: nested_group = group.add_mutually_exclusive_group(...)
            // These methods aren't needed with clap derive - skip the assignment
            if matches!(
                method.as_str(),
                "add_argument_group" | "add_mutually_exclusive_group" | "set_defaults"
            ) {
                if let HirExpr::Var(parent_var) = object.as_ref() {
                    // Check if parent_var is a parser OR a group
                    let is_parser_or_group = ctx.argparser_tracker.get_parser(parent_var).is_some()
                        || ctx
                            .argparser_tracker
                            .get_parser_for_group(parent_var)
                            .is_some();

                    if is_parser_or_group {
                        // add_argument() calls on it later (e.g., input_group.add_argument())
                        // This handles both:
                        //   - group = parser.add_argument_group() → register group → parser
                        //   - nested = group.add_mutually_exclusive_group() → register nested → group
                        // Recursive resolution will handle nested → group → parser chain
                        if let AssignTarget::Symbol(group_var) = target {
                            ctx.argparser_tracker
                                .register_group(group_var.clone(), parent_var.clone());
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }

            if method == "add_subparsers" {
                if let HirExpr::Var(parser_var) = object.as_ref() {
                    if ctx.argparser_tracker.get_parser(parser_var).is_some() {
                        // Extract dest and required from kwargs
                        let dest_field = extract_kwarg_string(kwargs, "dest")
                            .unwrap_or_else(|| "command".to_string());
                        let required = extract_kwarg_bool(kwargs, "required").unwrap_or(false);
                        let help = extract_kwarg_string(kwargs, "help");

                        if let AssignTarget::Symbol(subparsers_var) = target {
                            use crate::rust_gen::argparse_transform::SubparserInfo;
                            ctx.argparser_tracker.register_subparsers(
                                subparsers_var.clone(),
                                SubparserInfo {
                                    parser_var: parser_var.clone(),
                                    dest_field,
                                    required,
                                    help,
                                },
                            );
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }

            if method == "add_parser" {
                if let HirExpr::Var(subparsers_var) = object.as_ref() {
                    if ctx
                        .argparser_tracker
                        .get_subparsers(subparsers_var)
                        .is_some()
                    {
                        // Extract command name from first positional arg
                        if !args.is_empty() {
                            let command_name = extract_string_literal(&args[0]);
                            let help = extract_kwarg_string(kwargs, "help");

                            if let AssignTarget::Symbol(subcommand_var) = target {
                                use crate::rust_gen::argparse_transform::SubcommandInfo;
                                ctx.argparser_tracker.register_subcommand(
                                    subcommand_var.clone(),
                                    SubcommandInfo {
                                        name: command_name,
                                        help,
                                        arguments: vec![],
                                        subparsers_var: subparsers_var.clone(),
                                    },
                                );
                            }
                        }
                        // Skip this assignment - not needed with clap
                        return Ok(quote! {});
                    }
                }
            }
        }
    }

    // If we have dict[key] += value, avoid borrow-after-move by evaluating old value first
    if is_dict_augassign_pattern(target, value) {
        if let AssignTarget::Index { base, index } = target {
            if let HirExpr::Binary { op, left: _, right } = value {
                // Generate: let old_val = dict.get(&key).cloned().unwrap_or_default();
                //           dict.insert(key, old_val + right_value);
                let base_expr = base.to_rust_expr(ctx)?;
                let index_expr = index.to_rust_expr(ctx)?;
                let right_expr = right.to_rust_expr(ctx)?;
                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::Mod => quote! { % },
                    _ => bail!("Unsupported augmented assignment operator for dict"),
                };

                return Ok(quote! {
                    {
                        let _key = #index_expr;
                        let _old_val = #base_expr.get(&_key).cloned().unwrap_or_default();
                        #base_expr.insert(_key, _old_val #op_token #right_expr);
                    }
                });
            }
        }
    }

    // Handle augmented assignment on Optional field: obj.field += value
    // Generate: obj.field = Some(obj.field.unwrap() + value)
    if is_optional_attr_augassign_pattern(target, value, ctx) {
        if let AssignTarget::Attribute {
            value: base_value,
            attr,
        } = target
        {
            if let HirExpr::Binary { op, left: _, right } = value {
                let base_expr = base_value.to_rust_expr(ctx)?;
                let attr_ident = format_ident!("{}", attr);
                let right_expr = right.to_rust_expr(ctx)?;
                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::FloorDiv => quote! { / }, // Integer division in Rust is /
                    BinOp::Mod => quote! { % },
                    BinOp::BitAnd => quote! { & },
                    BinOp::BitOr => quote! { | },
                    BinOp::BitXor => quote! { ^ },
                    BinOp::LShift => quote! { << },
                    BinOp::RShift => quote! { >> },
                    BinOp::Pow => {
                        // Power operation needs special handling
                        return Ok(quote! {
                            #base_expr.#attr_ident = Some(#base_expr.#attr_ident.unwrap().pow(#right_expr as u32));
                        });
                    }
                    _ => bail!("Unsupported augmented assignment operator for Optional field"),
                };

                return Ok(quote! {
                    #base_expr.#attr_ident = Some(#base_expr.#attr_ident.unwrap() #op_token #right_expr);
                });
            }
        }
    }

    // Handle augmented assignment on Optional variable: x += value where x: Optional[int]
    // Generate: x = Some(x.unwrap() + value)
    if is_optional_var_augassign_pattern(target, value, ctx) {
        if let AssignTarget::Symbol(var_name) = target {
            if let HirExpr::Binary { op, left: _, right } = value {
                let var_ident = safe_ident(var_name);
                let right_expr = right.to_rust_expr(ctx)?;
                let op_token = match op {
                    BinOp::Add => quote! { + },
                    BinOp::Sub => quote! { - },
                    BinOp::Mul => quote! { * },
                    BinOp::Div => quote! { / },
                    BinOp::FloorDiv => quote! { / },
                    BinOp::Mod => quote! { % },
                    BinOp::BitAnd => quote! { & },
                    BinOp::BitOr => quote! { | },
                    BinOp::BitXor => quote! { ^ },
                    BinOp::LShift => quote! { << },
                    BinOp::RShift => quote! { >> },
                    BinOp::Pow => {
                        return Ok(quote! {
                            #var_ident = Some(#var_ident.unwrap().pow(#right_expr as u32));
                        });
                    }
                    _ => bail!("Unsupported augmented assignment operator for Optional variable"),
                };

                return Ok(quote! {
                    #var_ident = Some(#var_ident.unwrap() #op_token #right_expr);
                });
            }
        }
    }

    // This allows proper method dispatch for user-defined classes
    if let AssignTarget::Symbol(var_name) = target {
        // This enables correct {:?} vs {} selection in println! for collections
        // Example: result = merge(&a, &b) where merge returns Vec<i32>
        // Also track Optional types for proper Some() wrapping in reassignments
        if let Some(annot_type) = type_annotation {
            match annot_type {
                Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::Optional(_) => {
                    ctx.var_types.insert(var_name.clone(), annot_type.clone());
                    // Track variables declared as Option<T> for proper unwrapping in field access
                    if matches!(annot_type, Type::Optional(_)) {
                        ctx.optional_vars.insert(var_name.clone());
                    }
                }
                _ => {}
            }
        }

        match value {
            HirExpr::Call { func, args, .. } => {
                // Check if this is a user-defined class constructor
                if ctx.class_names.contains(func) {
                    ctx.var_types
                        .insert(var_name.clone(), Type::Custom(func.clone()));
                }
                // This enables correct HashSet.contains() vs HashMap.contains_key() selection
                else if func == "set" {
                    // Infer element type from type annotation or default to Int
                    let elem_type = if let Some(Type::Set(elem)) = type_annotation {
                        elem.as_ref().clone()
                    } else {
                        Type::Int // Default for untyped sets
                    };
                    ctx.var_types
                        .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
                }
                // Lookup function return type and track it for type inference
                // Enables: result = merge(&a, &b) where merge returns list[int]
                // Also tracks primitive types (Int, Float, Bool, String) for arithmetic/concat detection
                // And Custom types (structs/dataclasses) for clone analysis
                else if let Some(ret_type) = ctx.function_return_types.get(func) {
                    if matches!(
                        ret_type,
                        Type::List(_)
                            | Type::Dict(_, _)
                            | Type::Set(_)
                            | Type::Int
                            | Type::Float
                            | Type::Bool
                            | Type::String
                            | Type::Custom(_)
                    ) {
                        ctx.var_types.insert(var_name.clone(), ret_type.clone());
                    }
                    // Track if function returns Optional type
                    if matches!(ret_type, Type::Optional(_)) {
                        ctx.var_types.insert(var_name.clone(), ret_type.clone());
                        ctx.optional_vars.insert(var_name.clone());
                    }
                }
                // These all return Option<Match> in Rust
                else if matches!(func.as_str(), "search" | "match" | "find") {
                    // Only track if this looks like a regex call (needs more context to be sure)
                    // For now, track any call to search/match/find as Optional
                    // This is a heuristic - could be improved with module tracking
                    ctx.var_types
                        .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                    ctx.optional_vars.insert(var_name.clone());
                }
                // Track built-in functions that return int
                else if matches!(func.as_str(), "len" | "int" | "ord" | "round") {
                    ctx.var_types.insert(var_name.clone(), Type::Int);
                }
                // Track built-in functions that return float
                else if func == "float" {
                    ctx.var_types.insert(var_name.clone(), Type::Float);
                }
                // Track abs() - returns the same type as its argument
                else if func == "abs" {
                    if !args.is_empty() {
                        let arg_type = infer_expr_type_with_env(&args[0], &ctx.var_types);
                        if matches!(arg_type, Type::Int | Type::Float) {
                            ctx.var_types.insert(var_name.clone(), arg_type);
                        } else {
                            // Default to Int for abs() if we can't infer
                            ctx.var_types.insert(var_name.clone(), Type::Int);
                        }
                    }
                }
                // Track min() and max() - return Float if any argument is Float, else Int
                else if matches!(func.as_str(), "min" | "max") {
                    if !args.is_empty() {
                        let has_float = args.iter().any(|arg| {
                            matches!(infer_expr_type_with_env(arg, &ctx.var_types), Type::Float)
                        });
                        if has_float {
                            ctx.var_types.insert(var_name.clone(), Type::Float);
                        } else {
                            ctx.var_types.insert(var_name.clone(), Type::Int);
                        }
                    }
                }
                // Track next() builtin: with default (2 args) returns Option<T>, without default (1 arg) returns T
                else if func == "next" {
                    if args.len() == 2 {
                        // next(iter, default) returns Option<T> which unwraps to default if None
                        ctx.var_types
                            .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                        ctx.optional_vars.insert(var_name.clone());
                    }
                    // next(iter) without default uses .expect() and returns T directly (not Optional)
                }
            }
            HirExpr::List(elements) => {
                // When v = [1, 2], mark v as List(Int) so it gets borrowed when calling f(&v)
                // When v = [(1, 2), (3, 4)], mark v as List(Tuple(Int, Int)) for proper tuple indexing
                let elem_type = if let Some(Type::List(elem)) = type_annotation {
                    elem.as_ref().clone()
                } else if !elements.is_empty() {
                    // Infer from first element (handles tuples, nested lists, etc.)
                    infer_expr_type_with_env(&elements[0], &ctx.var_types)
                } else {
                    Type::Unknown
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            HirExpr::Dict(items) => {
                // When info = {"a": 1}, mark info as Dict(String, Int) so it gets borrowed
                let (key_type, val_type) = if let Some(Type::Dict(k, v)) = type_annotation {
                    (k.as_ref().clone(), v.as_ref().clone())
                } else if !items.is_empty() {
                    // Infer from first item (assume homogeneous dict)
                    // For string literal keys and int values
                    (Type::String, Type::Int)
                } else {
                    (Type::Unknown, Type::Unknown)
                };
                ctx.var_types.insert(
                    var_name.clone(),
                    Type::Dict(Box::new(key_type), Box::new(val_type)),
                );
            }
            HirExpr::Set(elements) | HirExpr::FrozenSet(elements) => {
                // Track set type from literal for proper method dispatch
                // Use type annotation if available, otherwise infer from elements
                let elem_type = if let Some(Type::Set(elem)) = type_annotation {
                    elem.as_ref().clone()
                } else if !elements.is_empty() {
                    // Infer from first element (assume homogeneous set)
                    // For int literals, use Int type
                    Type::Int
                } else {
                    Type::Unknown
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
            }
            HirExpr::Slice { base, .. } => {
                // When rest = numbers[1:], mark rest as List(Int) so it gets borrowed on call
                // Infer element type from base variable if available
                let elem_type = if let HirExpr::Var(base_var) = base.as_ref() {
                    if let Some(Type::List(elem)) = ctx.var_types.get(base_var) {
                        elem.as_ref().clone()
                    } else {
                        Type::Int // Default to Int for untyped slices
                    }
                } else {
                    Type::Int // Default to Int
                };
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            // E.g., value_str = data.get(...) where data: Vec<String> → value_str: String
            HirExpr::MethodCall { object, method, .. } => {
                // Track .get() on Vec<String> returning String
                if method == "get" {
                    if let HirExpr::Var(obj_var) = object.as_ref() {
                        if let Some(Type::List(elem_type)) = ctx.var_types.get(obj_var) {
                            // .get() returns Option<&T>, but after .cloned().unwrap_or_default()
                            // it becomes T, so track the element type
                            ctx.var_types
                                .insert(var_name.clone(), elem_type.as_ref().clone());
                        }
                    }
                }
                // Track .split() and .split_whitespace() as List(String) for truthiness conversion
                else if matches!(method.as_str(), "split" | "split_whitespace" | "splitlines") {
                    ctx.var_types
                        .insert(var_name.clone(), Type::List(Box::new(Type::String)));
                }
                // String methods that return String
                else if matches!(
                    method.as_str(),
                    "upper"
                        | "lower"
                        | "strip"
                        | "lstrip"
                        | "rstrip"
                        | "title"
                        | "replace"
                        | "format"
                ) {
                    ctx.var_types.insert(var_name.clone(), Type::String);
                }
                // Track .find(), .search(), .match() as Optional for truthiness conversion
                else if matches!(method.as_str(), "find" | "search" | "match") {
                    // Check if this is a regex method call (on compiled regex object)
                    // We don't have a specific regex type, so use Optional as a marker
                    ctx.var_types
                        .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                    ctx.optional_vars.insert(var_name.clone());
                }
                // Track .next() as Optional since it returns Option<T>
                else if method == "next" {
                    ctx.var_types
                        .insert(var_name.clone(), Type::Optional(Box::new(Type::Unknown)));
                    ctx.optional_vars.insert(var_name.clone());
                }
            }
            // When message = "hello", track message as String so it gets borrowed when calling f(&str)
            // But preserve Optional wrapper if the variable was declared as Optional[str]
            HirExpr::Literal(Literal::String(_)) => {
                // Only update if not already tracked, or if tracked but not Optional
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::String);
                }
            }
            // When match = 5, track match as Int so `if match:` becomes `if match != 0`
            // But preserve Optional wrapper if the variable was declared as Optional[int]
            HirExpr::Literal(Literal::Int(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Int);
                }
            }
            HirExpr::Literal(Literal::Float(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Float);
                }
            }
            HirExpr::Literal(Literal::Bool(_)) => {
                if !ctx.var_types.contains_key(var_name) {
                    ctx.var_types.insert(var_name.clone(), Type::Bool);
                }
            }
            // Track binary arithmetic expressions (e.g., c = a - b * 4)
            HirExpr::Binary { op, left, right } => {
                if !ctx.var_types.contains_key(var_name) {
                    let inferred_type = infer_binary_expr_type(ctx, op, left, right);
                    ctx.var_types.insert(var_name.clone(), inferred_type);
                }
            }
            // Track list comprehensions: exps = [math.exp(x) for x in logits]
            HirExpr::ListComp { element, .. } => {
                let elem_type = infer_expr_type_with_env(element, &ctx.var_types);
                ctx.var_types
                    .insert(var_name.clone(), Type::List(Box::new(elem_type)));
            }
            // Track set comprehensions: unique = {x.lower() for x in words}
            HirExpr::SetComp { element, .. } => {
                let elem_type = infer_expr_type_with_env(element, &ctx.var_types);
                ctx.var_types
                    .insert(var_name.clone(), Type::Set(Box::new(elem_type)));
            }
            // Track dict comprehensions: counts = {k: v * 2 for k, v in items}
            HirExpr::DictComp {
                key,
                value: val_expr,
                ..
            } => {
                let key_type = infer_expr_type_with_env(key, &ctx.var_types);
                let val_type = infer_expr_type_with_env(val_expr, &ctx.var_types);
                ctx.var_types.insert(
                    var_name.clone(),
                    Type::Dict(Box::new(key_type), Box::new(val_type)),
                );
            }
            // Track ternary expressions: win_factor = 500 if team_won else 0
            HirExpr::IfExpr { body, orelse, .. } => {
                if !ctx.var_types.contains_key(var_name) {
                    let body_type = infer_expr_type_with_env(body, &ctx.var_types);
                    let orelse_type = infer_expr_type_with_env(orelse, &ctx.var_types);
                    // If both branches have the same type, use that type
                    // Otherwise, if one is Float and one is Int, prefer Float (promotion)
                    let inferred_type = if body_type == orelse_type {
                        body_type
                    } else if matches!(
                        (&body_type, &orelse_type),
                        (Type::Float, Type::Int) | (Type::Int, Type::Float)
                    ) {
                        Type::Float
                    } else {
                        // Default to the body type if we can't unify
                        body_type
                    };
                    ctx.var_types.insert(var_name.clone(), inferred_type);
                }
            }
            // Propagate types from one variable to another: abs_margin = _cse_temp_0
            HirExpr::Var(source_var) => {
                if !ctx.var_types.contains_key(var_name) {
                    if let Some(source_type) = ctx.var_types.get(source_var) {
                        ctx.var_types.insert(var_name.clone(), source_type.clone());
                    }
                }
            }
            // Track attribute access types: x = obj.field where field might be Optional
            HirExpr::Attribute { value, attr } => {
                if !ctx.var_types.contains_key(var_name) {
                    if let Some(field_type) = ctx.get_attribute_field_type(value, attr) {
                        ctx.var_types.insert(var_name.clone(), field_type);
                    }
                }
            }
            // Track index access types: x = arr[i] where arr is List<T> means x is T
            HirExpr::Index { base, .. } => {
                if !ctx.var_types.contains_key(var_name) {
                    let base_type = infer_expr_type_with_env(base, &ctx.var_types);
                    let elem_type = match base_type {
                        Type::List(elem) => Some(*elem),
                        Type::Tuple(elems) => elems.first().cloned(),
                        Type::Dict(_, val) => Some(*val),
                        _ => None,
                    };
                    if let Some(t) = elem_type {
                        ctx.var_types.insert(var_name.clone(), t);
                    }
                }
            }
            _ => {}
        }
    }

    // Handle uninitialized annotated declarations (Python: `x: T`)
    let is_uninitialized = matches!(value, HirExpr::Uninitialized);

    // Track type for uninitialized annotated declarations
    // This ensures Optional types are tracked for truthiness conversion in conditionals
    if is_uninitialized {
        if let AssignTarget::Symbol(var_name) = target {
            if let Some(annot_type) = type_annotation {
                ctx.var_types.insert(var_name.clone(), annot_type.clone());
            }
        }
    }

    // Check if this is a field access assignment that can use borrowing
    // Pattern: `let players = state.all_players` where players is only used for iteration
    // Also handles: `let team_stats = (state.home_stats if cond else state.away_stats)`
    // Don't borrow Copy types (primitives like i32, f64, bool) - they should be copied directly
    // Don't borrow enum variants - they are Copy types
    let (should_borrow, should_mut_borrow) = if let AssignTarget::Symbol(var_name) = target {
        let is_attribute_sourced = is_attribute_sourced_expr(value);
        let is_copy_type = if let HirExpr::Attribute { value: base, attr } = value {
            ctx.is_attribute_copy_type(base, attr)
        } else {
            false
        };
        // Check if the value (or both branches of an IfExpr) are enum variants
        let is_enum_variant = is_enum_variant_expr(value, ctx);

        // Check if this is an empty collection initialization for a variable that will be
        // later assigned from an attribute source. In this case, we need to declare it
        // with a reference type even though the current value isn't attribute-sourced.
        let is_empty_init_for_borrowable = is_empty_collection_init_expr(value)
            && (ctx.should_borrow_var(var_name) || ctx.should_mut_borrow_var(var_name));

        if is_attribute_sourced && !is_copy_type && !is_enum_variant {
            if ctx.should_mut_borrow_var(var_name) {
                (false, true)
            } else if ctx.should_borrow_var(var_name) {
                (true, false)
            } else {
                (false, false)
            }
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    // For type annotations, use the same borrow flags as for values
    let (type_should_borrow, type_should_mut_borrow) = (should_borrow, should_mut_borrow);

    // Convert the value expression unless it's an Uninitialized marker
    let mut value_expr = if is_uninitialized {
        // Placeholder; won't be used when is_uninitialized is true
        parse_quote! { () }
    } else if should_mut_borrow {
        // Generate a mutable borrow for field access from &mut T source
        ctx.set_generate_borrow(true);
        ctx.set_generate_mut_borrow(true);
        let expr = value.to_rust_expr(ctx)?;
        ctx.set_generate_mut_borrow(false);
        ctx.set_generate_borrow(false);
        // For conditional expressions (IfExpr), the &mut is added inside each branch
        // For direct attribute access, wrap in &mut
        if matches!(value, HirExpr::IfExpr { .. }) {
            expr
        } else {
            parse_quote! { &mut #expr }
        }
    } else if should_borrow {
        // Generate an immutable borrow instead of clone for field access
        ctx.set_generate_borrow(true);
        let expr = value.to_rust_expr(ctx)?;
        ctx.set_generate_borrow(false);
        // For conditional expressions (IfExpr), the & is added inside each branch
        // For direct attribute access, wrap in &
        if matches!(value, HirExpr::IfExpr { .. }) {
            expr
        } else {
            parse_quote! { &#expr }
        }
    } else {
        value.to_rust_expr(ctx)?
    };

    // BORROW CONFLICT RESOLUTION:
    // If this variable is in vars_needing_clone_at_assign, it holds a reference
    // that would conflict with a later mutable borrow. Clone/to_vec to release
    // the borrow immediately.
    // Pattern detected: var1 = f(&state) returns &T, var2 = g(&mut state), use(var1)
    if let AssignTarget::Symbol(var_name) = target {
        if ctx.vars_needing_clone_at_assign.contains(var_name) {
            // Check if the value is a function call that likely returns &Vec<T>
            if let HirExpr::Call { .. } = value {
                // Use .to_vec() for Vec references, .clone() for others
                // Heuristic: if function name contains "get_players" or similar list getters
                // Actually, safer to use .clone() which works for both Vec and other types
                value_expr = parse_quote! { #value_expr.clone() };
            }
        }
    }

    // When assigning from a function that returns Result<T, E> in a non-Result context,
    // we need to unwrap it.

    // Fix #6 removed automatic `?` from expr_gen.rs, so we need to add it here at the
    // statement level where we know the variable type context.

    // Five-Whys Root Cause:
    // 1. Why: expected `i32`, found `Result<i32, Box<dyn Error>>`
    // 2. Why: Variable `position: i32` assigned Result-returning function without unwrap
    // 3. Why: Neither `?` nor `.unwrap()` added to function call
    // 4. Why: Fix #6 removed `?` from expr_gen, and only adds `.unwrap()` for non-Result callers
    // 5. ROOT CAUSE: Missing `?` for Result→Result propagation after Fix #6
    if let HirExpr::Call { func, .. } = value {
        if ctx.result_returning_functions.contains(func) {
            if ctx.current_function_can_fail {
                // Current function also returns Result - add ? to propagate error
                value_expr = parse_quote! { #value_expr? };
            } else {
                // Current function doesn't return Result - add .unwrap() to extract the value
                value_expr = parse_quote! { #value_expr.unwrap() };
            }
        }
    }

    // If there's a type annotation, handle type conversions
    let (type_annotation_tokens, is_final) = if let Some(target_type) = type_annotation {
        // Check if this is a Final type annotation
        let (actual_type, is_const) = match target_type {
            Type::Final(inner) => (inner.as_ref(), true),
            _ => (target_type, false),
        };

        let target_rust_type = ctx.type_mapper.map_type(actual_type);
        let target_syn_type = rust_type_to_syn(&target_rust_type)?;

        // When borrowing, wrap the type in a reference
        // Use type_should_borrow which accounts for empty collection inits
        let final_syn_type: syn::Type = if type_should_borrow {
            parse_quote! { &#target_syn_type }
        } else if type_should_mut_borrow {
            parse_quote! { &mut #target_syn_type }
        } else {
            target_syn_type
        };

        // Auto-unwrap Optional values when assigning to non-Optional annotated variables
        let value_is_optional = expr_is_optional(value, ctx);
        let target_is_optional = matches!(actual_type, Type::Optional(_));
        if value_is_optional && !target_is_optional {
            value_expr = parse_quote! { #value_expr.unwrap() };
        }

        // Pass the value expression to determine if cast is actually needed
        // NOTE: This handles string literals → String conversion via apply_type_conversion
        if needs_type_conversion(actual_type, value) {
            value_expr = apply_type_conversion(value_expr, actual_type);
        }

        (Some(quote! { : #final_syn_type }), is_const)
    } else {
        // No explicit type annotation, but we may still need conversions
        // When assigning string literals without type annotation,
        // they should become owned Strings (Python semantics)
        if matches!(value, HirExpr::Literal(Literal::String(_))) {
            value_expr = parse_quote! { #value_expr.to_string() };
        }
        // NOTE: Struct field cloning is handled in convert_attribute via field_needs_clone
        // which properly checks if the field type is Copy or not
        (None, false)
    };

    // When assigning to an Option<T> variable, wrap non-None values in Some()
    if let AssignTarget::Symbol(symbol) = target {
        // Check if the variable has an Optional type (either from annotation or previous declaration)
        let is_optional_type = if let Some(target_type) = type_annotation {
            matches!(target_type, Type::Optional(_))
        } else if ctx.is_declared(symbol) {
            // Check if variable was previously declared with Optional type
            ctx.var_types
                .get(symbol)
                .map_or(false, |ty| matches!(ty, Type::Optional(_)))
        } else {
            false
        };

        // Check if the value being assigned is already Optional (to avoid double-wrapping)
        let value_is_optional = expr_is_optional(value, ctx);

        // Wrap non-None values in Some() when assigning to Option<T>, unless the value is already Optional
        if is_optional_type
            && !value_is_optional
            && !matches!(value, HirExpr::Literal(Literal::None))
        {
            value_expr = parse_quote! { Some(#value_expr) };
        }
    }

    // When assigning to an Optional attribute (struct field), wrap non-None values in Some()
    // This handles augmented assignments like `c.value += 1` where `c.value: Optional[int]`
    // which get transformed by CSE into `let _cse_temp = c.value.unwrap() + 1; c.value = _cse_temp;`
    if let AssignTarget::Attribute {
        value: target_base,
        attr,
    } = target
    {
        // Build a temporary HirExpr::Attribute to check if the target field is Optional
        let target_attr_expr = HirExpr::Attribute {
            value: target_base.clone(),
            attr: attr.clone(),
        };
        let target_is_optional = expr_is_optional(&target_attr_expr, ctx);

        // Check if the value being assigned is already Optional (to avoid double-wrapping)
        let value_is_optional = expr_is_optional(value, ctx);

        // Wrap non-None values in Some() when assigning to Optional field
        if target_is_optional
            && !value_is_optional
            && !matches!(value, HirExpr::Literal(Literal::None))
        {
            value_expr = parse_quote! { Some(#value_expr) };
        }

        // Unwrap Optional values when assigning to non-Optional field
        // This handles cases like: state.field = optional_var.clone()
        // where state.field: T but optional_var: Option<T>
        if !target_is_optional && value_is_optional {
            value_expr = parse_quote! { #value_expr.unwrap() };
        }

        // Clone reference parameters when assigning to struct fields.
        // When assigning `&T` to a field expecting `T`, we need to clone.
        // Example: player.sin_bin_status = sin_bin_status where sin_bin_status: &SinBinStatus
        if let HirExpr::Var(var_name) = value {
            if ctx.current_func_ref_params.contains(var_name)
                && !ctx.shadowed_ref_params.contains(var_name)
            {
                value_expr = parse_quote! { #value_expr.clone() };
            }
        }
    }

    // If this is an annotated declaration without a value, emit a declaration
    // without initializer. Only supported for simple symbol targets.
    if is_uninitialized {
        if let AssignTarget::Symbol(symbol) = target {
            // Declare the variable in the current scope
            ctx.declare_var(symbol);
            let target_ident = safe_ident(symbol);

            // If mutable, include mut
            let mutability = if ctx.mutable_vars.contains(symbol) {
                quote! { mut }
            } else {
                quote! {}
            };

            if let Some(type_ann) = type_annotation_tokens {
                // let mut x: Type;
                return Ok(quote! { let #mutability #target_ident #type_ann; });
            } else {
                // No type annotation - emit simple declaration without initializer
                // This is not valid Rust, so fall back to declaring with a unit value
                // to avoid generating invalid code. Prefer explicit type annotations
                // in source to avoid this path.
                if ctx.mutable_vars.contains(symbol) {
                    return Ok(quote! { let mut #target_ident = (); });
                } else {
                    return Ok(quote! { let #target_ident = (); });
                }
            }
        } else {
            // Annotated declarations without values for complex targets are unsupported
            bail!("Annotated assignment without value for non-symbol target not supported")
        }
    }

    match target {
        AssignTarget::Symbol(symbol) => {
            codegen_assign_symbol(symbol, value_expr, type_annotation_tokens, is_final, ctx)
        }
        AssignTarget::Index { base, index } => codegen_assign_index(base, index, value_expr, ctx),
        AssignTarget::Slice { base, .. } => codegen_assign_slice(base, value_expr, ctx),
        AssignTarget::Attribute { value, attr } => {
            codegen_assign_attribute(value, attr, value_expr, ctx)
        }
        AssignTarget::Tuple(targets) => {
            codegen_assign_tuple(targets, value_expr, type_annotation_tokens, ctx)
        }
    }
}

/// Generate code for symbol (variable) assignment
#[inline]
pub(crate) fn codegen_assign_symbol(
    symbol: &str,
    value_expr: syn::Expr,
    type_annotation_tokens: Option<proc_macro2::TokenStream>,
    is_final: bool,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let target_ident = safe_ident(symbol);

    // Inside generators, check if variable is a state variable
    if ctx.in_generator && ctx.generator_state_vars.contains(symbol) {
        // State variable assignment: self.field = value
        Ok(quote! { self.#target_ident = #value_expr; })
    } else if is_final {
        // Final type annotation - generate const instead of let
        if let Some(type_ann) = type_annotation_tokens {
            Ok(quote! { const #target_ident #type_ann = #value_expr; })
        } else {
            // Final without explicit type annotation - shouldn't happen, but handle gracefully
            Ok(quote! { const #target_ident = #value_expr; })
        }
    } else if ctx.is_declared(symbol) {
        // Variable already exists, just assign
        Ok(quote! { #target_ident = #value_expr; })
    } else {
        // First declaration - check if variable needs mut
        ctx.declare_var(symbol);
        if ctx.mutable_vars.contains(symbol) {
            if let Some(type_ann) = type_annotation_tokens {
                Ok(quote! { let mut #target_ident #type_ann = #value_expr; })
            } else {
                Ok(quote! { let mut #target_ident = #value_expr; })
            }
        } else if let Some(type_ann) = type_annotation_tokens {
            Ok(quote! { let #target_ident #type_ann = #value_expr; })
        } else {
            Ok(quote! { let #target_ident = #value_expr; })
        }
    }
}

/// Generate code for index (dictionary/list subscript) assignment
#[inline]
pub(crate) fn codegen_assign_index(
    base: &HirExpr,
    index: &HirExpr,
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    let mut final_index = index.to_rust_expr(ctx)?;

    // Check base variable type to determine if this is Vec or HashMap
    // Vec.insert() requires usize index, HashMap.insert() takes key of any type
    let is_numeric_index = if let HirExpr::Var(base_name) = base {
        // Check if we have type information for this variable
        if let Some(base_type) = ctx.var_types.get(base_name) {
            // Type-based detection (most reliable)
            match base_type {
                Type::List(_) => true,     // List/Vec → numeric index
                Type::Dict(_, _) => false, // Dict/HashMap → key (not numeric)
                _ => {
                    // Fall back to index heuristic for other types
                    match index {
                        HirExpr::Var(name) => {
                            let name_str = name.as_str();
                            // String-like variable names → NOT numeric
                            if name_str == "key"
                                || name_str == "k"
                                || name_str == "name"
                                || name_str == "id"
                                || name_str == "word"
                                || name_str == "text"
                                || name_str == "char"
                                || name_str == "character"
                                || name_str == "c"
                                || name_str.ends_with("_key")
                                || name_str.ends_with("_name")
                            {
                                false
                            } else {
                                // Default: assume numeric for other variables
                                true
                            }
                        }
                        HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => {
                            true
                        }
                        _ => false,
                    }
                }
            }
        } else {
            // No type info - use heuristic
            match index {
                HirExpr::Var(name) => {
                    let name_str = name.as_str();
                    // String-like variable names → NOT numeric
                    if name_str == "key"
                        || name_str == "k"
                        || name_str == "name"
                        || name_str == "id"
                        || name_str == "word"
                        || name_str == "text"
                        || name_str == "char"
                        || name_str == "character"
                        || name_str == "c"
                        || name_str.ends_with("_key")
                        || name_str.ends_with("_name")
                    {
                        false
                    } else {
                        // Default: assume numeric for other variables
                        true
                    }
                }
                HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => true,
                _ => false,
            }
        }
    } else {
        // Base is not a simple variable - use heuristic
        match index {
            HirExpr::Var(name) => {
                let name_str = name.as_str();
                // String-like variable names → NOT numeric
                if name_str == "key"
                    || name_str == "k"
                    || name_str == "name"
                    || name_str == "id"
                    || name_str == "word"
                    || name_str == "text"
                    || name_str == "char"
                    || name_str == "character"
                    || name_str == "c"
                    || name_str.ends_with("_key")
                    || name_str.ends_with("_name")
                {
                    false
                } else {
                    // Default: assume numeric for other variables
                    true
                }
            }
            HirExpr::Binary { .. } | HirExpr::Literal(crate::hir::Literal::Int(_)) => true,
            _ => false,
        }
    };

    // Convert string literal keys to String for HashMap operations
    // String literals (&str) need to be converted to String for HashMap<String, V>
    if !is_numeric_index && matches!(index, HirExpr::Literal(Literal::String(_))) {
        final_index = parse_quote! { #final_index.to_string() };
    }

    // Extract the base and all intermediate indices
    // For mutation operations on field accesses (both dict and list), we need to avoid cloning
    // Check if the base (or its root for nested Index) is a field access
    let base_is_field_access = match base {
        HirExpr::Attribute { .. } => true,
        HirExpr::Index {
            base: inner_base, ..
        } => {
            // For nested subscripts like matrix[0][1], check if the root is an attribute
            fn has_attribute_root(expr: &HirExpr) -> bool {
                match expr {
                    HirExpr::Attribute { .. } => true,
                    HirExpr::Index { base, .. } => has_attribute_root(base),
                    _ => false,
                }
            }
            has_attribute_root(inner_base)
        }
        _ => false,
    };

    let (base_expr, indices) = if base_is_field_access {
        // Generate field access without clone for mutation operations
        extract_nested_indices_tokens_no_clone(base, ctx)?
    } else {
        extract_nested_indices_tokens(base, ctx)?
    };

    // Check if value_expr is a string literal and the dict value type is String
    let value_expr = if !is_numeric_index {
        // Get the base variable name to look up its type
        let base_name = match base {
            HirExpr::Var(name) => Some(name.as_str()),
            HirExpr::Index {
                base: inner_base, ..
            } => {
                // For nested subscripts, get the root variable
                fn get_root_var(expr: &HirExpr) -> Option<&str> {
                    match expr {
                        HirExpr::Var(name) => Some(name.as_str()),
                        HirExpr::Index { base, .. } => get_root_var(base),
                        _ => None,
                    }
                }
                get_root_var(inner_base)
            }
            _ => None,
        };

        // Check if we need to convert string literal to String
        let needs_string_conversion = if let Some(name) = base_name {
            if let Some(base_type) = ctx.var_types.get(name) {
                // Navigate through nested Dict types to find the innermost value type
                let depth = indices.len() + 1; // +1 for the final index
                let mut current_type = base_type.clone();
                for _ in 0..depth {
                    if let Type::Dict(_, val_type) = current_type {
                        current_type = (*val_type).clone();
                    } else {
                        break;
                    }
                }
                // Check if innermost value type is String
                matches!(current_type, Type::String)
            } else {
                false
            }
        } else {
            false
        };

        // Check if value_expr is a string literal
        let is_string_literal =
            matches!(&value_expr, syn::Expr::Lit(lit) if matches!(&lit.lit, syn::Lit::Str(_)));

        if needs_string_conversion && is_string_literal {
            parse_quote! { #value_expr.to_string() }
        } else {
            value_expr
        }
    } else {
        value_expr
    };

    // Check variable type from context first, then fall back to name heuristic
    let needs_as_object_mut = if let HirExpr::Var(base_name) = base {
        if !is_numeric_index {
            // First check actual type from context
            if let Some(var_type) = ctx.var_types.get(base_name) {
                // If we know the type is Dict/HashMap, don't use as_object_mut
                if matches!(var_type, Type::Dict(_, _)) {
                    false
                } else {
                    // For Unknown types, use name heuristic
                    let name_str = base_name.as_str();
                    // Variables commonly used with serde_json::Value
                    name_str == "config"
                        || name_str == "value"
                        || name_str == "current"
                        || name_str == "obj"
                        || name_str == "json"
                }
            } else {
                // No type info available, use name heuristic
                // But exclude "data" as it's commonly used for HashMap
                let name_str = base_name.as_str();
                name_str == "config"
                    || name_str == "value"
                    || name_str == "current"
                    || name_str == "obj"
                    || name_str == "json"
            }
        } else {
            false
        }
    } else {
        false
    };

    if indices.is_empty() {
        // Simple assignment: d[k] = v OR list[i] = x
        if is_numeric_index {
            // For Vec/List: use direct indexing to replace the element
            // Note: Vec::insert() INSERTS a new element, we want to REPLACE
            Ok(quote! { #base_expr[#final_index as usize] = #value_expr; })
        } else if needs_as_object_mut {
            Ok(quote! { #base_expr.as_object_mut().unwrap().insert(#final_index, #value_expr); })
        } else {
            // HashMap.insert(key, value)
            Ok(quote! { #base_expr.insert(#final_index, #value_expr); })
        }
    } else {
        // Nested assignment: build chain of get_mut calls
        let mut chain = quote! { #base_expr };
        for idx in &indices {
            // Check if the intermediate index is numeric or dict key
            // For list types (Vec), indices should be cast to usize without & reference
            // For now, use the outer is_numeric_index as a heuristic - if the final
            // index is numeric, intermediate indices are likely also numeric (nested lists)
            if is_numeric_index {
                chain = quote! {
                    #chain.get_mut(#idx as usize).unwrap()
                };
            } else {
                chain = quote! {
                    #chain.get_mut(&#idx).unwrap()
                };
            }
        }

        if is_numeric_index {
            // For Vec/List: use direct indexing to replace the element
            // Note: Vec::insert() INSERTS a new element, we want to REPLACE
            Ok(quote! { #chain[#final_index as usize] = #value_expr; })
        } else if needs_as_object_mut {
            Ok(quote! { #chain.as_object_mut().unwrap().insert(#final_index, #value_expr); })
        } else {
            // HashMap.insert(key, value)
            Ok(quote! { #chain.insert(#final_index, #value_expr); })
        }
    }
}

/// Generate code for slice assignment: x[:] = value
#[inline]
pub(crate) fn codegen_assign_slice(
    base: &HirExpr,
    value_expr: syn::Expr,
    _ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Use build_expr_no_clone to avoid adding .clone() to the base
    // This is a mutation operation, so we need direct access to the field
    let base_expr = build_expr_no_clone(base);
    // For full slice assignment x[:] = value, clear and extend
    Ok(quote! {
        #base_expr.clear();
        #base_expr.extend(#value_expr);
    })
}

/// Generate code for attribute (struct field) assignment
#[inline]
pub(crate) fn codegen_assign_attribute(
    base: &HirExpr,
    attr: &str,
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // For assignment targets, we need mutable access - don't use .get().cloned() patterns
    // Instead, use direct indexing which gives us a mutable reference

    // Set flag to indicate we're generating an assignment target (LHS)
    // This prevents adding .clone() to the base expression
    let was_assignment_target = ctx.is_assignment_target;
    ctx.is_assignment_target = true;

    let mut base_expr = if let HirExpr::Index {
        base: inner_base,
        index,
    } = base
    {
        // Generate direct index access for mutation: items[0] instead of items.get(0).cloned().unwrap()
        let inner_base_expr = inner_base.to_rust_expr(ctx)?;
        let index_expr = index.to_rust_expr(ctx)?;
        // Check if index is a literal integer
        if let HirExpr::Literal(crate::hir::Literal::Int(n)) = &**index {
            let idx = *n as usize;
            parse_quote! { #inner_base_expr[#idx] }
        } else {
            parse_quote! { #inner_base_expr[#index_expr as usize] }
        }
    } else {
        base.to_rust_expr(ctx)?
    };

    // Restore flag
    ctx.is_assignment_target = was_assignment_target;

    // Handle Optional variable unwrapping for assignment targets.
    // When the base is an Optional<T> variable (e.g., player: Option<Player>),
    // we need to unwrap it to access the inner type's fields.
    // Example: player.sin_bin_status = x → player.as_mut().unwrap().sin_bin_status = x
    if let HirExpr::Var(var_name) = base {
        if let Some(Type::Optional(_)) = ctx.var_types.get(var_name) {
            base_expr = parse_quote! { #base_expr.as_mut().unwrap() };
        }
    }

    let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
    Ok(quote! { #base_expr.#attr_ident = #value_expr; })
}

/// Generate code for tuple unpacking assignment
#[inline]
pub(crate) fn codegen_assign_tuple(
    targets: &[AssignTarget],
    value_expr: syn::Expr,
    _type_annotation_tokens: Option<proc_macro2::TokenStream>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Check if all targets are simple symbols
    let all_symbols: Option<Vec<&str>> = targets
        .iter()
        .map(|t| match t {
            AssignTarget::Symbol(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();

    match all_symbols {
        Some(symbols) => {
            let all_declared = symbols.iter().all(|s| ctx.is_declared(s));

            if all_declared {
                // All variables exist, do reassignment
                let idents: Vec<_> = symbols.iter().map(|s| safe_ident(s)).collect();
                Ok(quote! { (#(#idents),*) = #value_expr; })
            } else {
                // First declaration - mark each variable individually
                symbols.iter().for_each(|s| ctx.declare_var(s));
                let idents_with_mut: Vec<_> = symbols
                    .iter()
                    .map(|s| {
                        let ident = safe_ident(s);
                        if ctx.mutable_vars.contains(*s) {
                            quote! { mut #ident }
                        } else {
                            quote! { #ident }
                        }
                    })
                    .collect();
                Ok(quote! { let (#(#idents_with_mut),*) = #value_expr; })
            }
        }
        None => {
            // Handle complex tuple unpacking with index targets
            // Pattern: a[0], a[2] = a[2], a[0] (swap pattern)
            codegen_complex_tuple_unpack(targets, value_expr, ctx)
        }
    }
}

/// Generate code for complex tuple unpacking (with index targets)
fn codegen_complex_tuple_unpack(
    targets: &[AssignTarget],
    value_expr: syn::Expr,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Generate temporary variables to capture RHS values first
    let temp_names: Vec<syn::Ident> = (0..targets.len())
        .map(|i| syn::Ident::new(&format!("_swap_tmp{}", i), proc_macro2::Span::call_site()))
        .collect();

    // Create tuple pattern for temporaries: let (_swap_tmp0, _swap_tmp1, ...) = value_expr;
    let temp_pattern: Vec<_> = temp_names.iter().map(|name| quote! { #name }).collect();
    let capture_stmt = quote! { let (#(#temp_pattern),*) = #value_expr; };

    // Generate individual assignments from temporaries to targets
    let mut assignments = Vec::new();
    for (i, target) in targets.iter().enumerate() {
        let temp_name = &temp_names[i];
        let assign = match target {
            AssignTarget::Symbol(symbol) => {
                let ident = safe_ident(symbol);
                quote! { #ident = #temp_name; }
            }
            AssignTarget::Index { base, index } => {
                let base_expr = base.to_rust_expr(ctx)?;
                let index_expr = index.to_rust_expr(ctx)?;
                quote! { #base_expr[#index_expr as usize] = #temp_name; }
            }
            AssignTarget::Attribute { value: base, attr } => {
                let base_expr = base.to_rust_expr(ctx)?;
                let attr_ident = syn::Ident::new(attr, proc_macro2::Span::call_site());
                quote! { #base_expr.#attr_ident = #temp_name; }
            }
            AssignTarget::Tuple(_) => {
                bail!("Nested tuple unpacking not supported")
            }
            AssignTarget::Slice { .. } => {
                bail!("Slice target in tuple unpacking not supported")
            }
        };
        assignments.push(assign);
    }

    Ok(quote! {
        {
            #capture_stmt
            #(#assignments)*
        }
    })
}

/// Generate code for Try/except/finally statement
#[inline]
pub(crate) fn codegen_try_stmt(
    body: &[HirStmt],
    handlers: &[ExceptHandler],
    finalbody: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Pattern: try { return int(str_var) } except ValueError { return literal }
    // We can optimize this to: s.parse::<i32>().unwrap_or(literal)
    // Those need proper match with Err(e) binding
    let simple_pattern_info = if body.len() == 1
        && handlers.len() == 1
        && handlers[0].body.len() == 1
        && handlers[0].name.is_none()
    // No exception variable binding
    {
        // Check if handler body is a Return statement with a simple value
        match &handlers[0].body[0] {
            // Direct literal: return 42, return "error", etc.
            HirStmt::Return(Some(HirExpr::Literal(lit))) => Some((
                (match lit {
                    Literal::Int(n) => n.to_string(),
                    Literal::Float(f) => f.to_string(),
                    Literal::String(s) => format!("\"{}\"", s),
                    Literal::Bool(b) => b.to_string(),
                    _ => "Default::default()".to_string(),
                })
                .to_string(),
                handlers[0].exception_type.clone(),
            )),
            // Unary negation: return -1, return -42, etc.
            HirStmt::Return(Some(HirExpr::Unary { op, operand })) => {
                if let HirExpr::Literal(lit) = &**operand {
                    match (op, lit) {
                        (crate::hir::UnaryOp::Neg, Literal::Int(n)) => {
                            Some((format!("-{}", n), handlers[0].exception_type.clone()))
                        }
                        (crate::hir::UnaryOp::Neg, Literal::Float(f)) => {
                            Some((format!("-{}", f), handlers[0].exception_type.clone()))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };

    let handled_types: Vec<String> = handlers
        .iter()
        .filter_map(|h| h.exception_type.clone())
        .collect();

    // Empty list means bare except (catches all exceptions)
    ctx.enter_try_scope(handled_types.clone());

    let has_zero_div_handler = handlers
        .iter()
        .any(|h| h.exception_type.as_deref() == Some("ZeroDivisionError"));

    if has_zero_div_handler && body.len() == 1 {
        if let HirStmt::Return(Some(expr)) = &body[0] {
            if contains_floor_div(expr) {
                // Extract divisor from floor division
                let divisor_expr = extract_divisor_from_floor_div(expr)?;
                let divisor_tokens = divisor_expr.to_rust_expr(ctx)?;

                // Find ZeroDivisionError handler
                let zero_div_handler_idx = handlers
                    .iter()
                    .position(|h| h.exception_type.as_deref() == Some("ZeroDivisionError"))
                    .unwrap();

                // Generate handler body
                ctx.enter_scope();
                let old_is_final = ctx.is_final_statement;
                ctx.is_final_statement = false;
                let handler_stmts: Vec<_> = handlers[zero_div_handler_idx]
                    .body
                    .iter()
                    .map(|s| s.to_rust_tokens(ctx))
                    .collect::<Result<Vec<_>>>()?;
                ctx.is_final_statement = old_is_final;
                ctx.exit_scope();

                // Generate try block expression (with params shadowing)
                let floor_div_result = expr.to_rust_expr(ctx)?;

                ctx.exit_exception_scope();

                // Generate: if divisor == 0 { handler } else { floor_div_result }
                if let Some(finalbody) = finalbody {
                    ctx.enter_scope();
                    let finally_stmts: Vec<_> = finalbody
                        .iter()
                        .map(|s| s.to_rust_tokens(ctx))
                        .collect::<Result<Vec<_>>>()?;
                    ctx.exit_scope();

                    return Ok(quote! {
                        {
                            if #divisor_tokens == 0 {
                                #(#handler_stmts)*
                            } else {
                                return #floor_div_result;
                            }
                            #(#finally_stmts)*
                        }
                    });
                } else {
                    return Ok(quote! {
                        if #divisor_tokens == 0 {
                            #(#handler_stmts)*
                        } else {
                            return #floor_div_result;
                        }
                    });
                }
            }
        }
    }

    // Convert try body to statements
    // Save and temporarily disable is_final_statement so return statements
    // in try blocks get the explicit 'return' keyword (needed for proper exception handling)
    let saved_is_final = ctx.is_final_statement;
    ctx.is_final_statement = false;

    ctx.enter_scope();
    let try_stmts: Vec<_> = body
        .iter()
        .map(|s| s.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;
    ctx.exit_scope();

    // Restore is_final_statement flag
    ctx.is_final_statement = saved_is_final;

    ctx.exit_exception_scope();

    // Generate except handler code
    let mut handler_tokens = Vec::new();
    for handler in handlers {
        ctx.enter_handler_scope();
        ctx.enter_scope();

        // If there's a name binding, declare it in scope
        if let Some(var_name) = &handler.name {
            ctx.declare_var(var_name);
        }

        // Save and temporarily disable is_final_statement so return statements
        // in handlers get the explicit 'return' keyword (needed for proper exception handling)
        let saved_is_final = ctx.is_final_statement;
        ctx.is_final_statement = false;

        let handler_stmts: Vec<_> = handler
            .body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;

        // Restore is_final_statement flag
        ctx.is_final_statement = saved_is_final;
        ctx.exit_scope();
        ctx.exit_exception_scope();

        handler_tokens.push(quote! { #(#handler_stmts)* });
    }

    // Generate finally clause if present
    let finally_stmts = if let Some(finally_body) = finalbody {
        let stmts: Vec<_> = finally_body
            .iter()
            .map(|s| s.to_rust_tokens(ctx))
            .collect::<Result<Vec<_>>>()?;
        Some(quote! { #(#stmts)* })
    } else {
        None
    };

    // Generate try/except/finally pattern
    if handlers.is_empty() {
        // Try/finally without except
        if let Some(finally_code) = finally_stmts {
            Ok(quote! {
                {
                    #(#try_stmts)*
                    #finally_code
                }
            })
        } else {
            // Just try block
            Ok(quote! { #(#try_stmts)* })
        }
    } else {
        // Check if try_stmts contains a .parse() call that we can convert to match
        if handlers.len() == 1 {
            if let Some((var_name, parse_expr_str, remaining_stmts)) =
                extract_parse_from_tokens(&try_stmts)
            {
                // Parse the expression string back to token stream
                let parse_expr: proc_macro2::TokenStream = parse_expr_str.parse().unwrap();
                let ok_var = safe_ident(&var_name);

                // Generate Ok branch (remaining statements after parse)
                let ok_body = quote! { #(#remaining_stmts)* };

                // Generate Err branch (handler body)
                let err_body = &handler_tokens[0];

                let err_pattern = if let Some(exc_var) = &handlers[0].name {
                    // Bind exception variable: Err(e) => { ... }
                    let exc_ident = safe_ident(exc_var);
                    quote! { Err(#exc_ident) }
                } else {
                    // No exception variable: Err(_) => { ... }
                    quote! { Err(_) }
                };

                // Build match expression
                let match_expr = quote! {
                    match #parse_expr {
                        Ok(#ok_var) => { #ok_body },
                        #err_pattern => { #err_body }
                    }
                };

                // Wrap with finally if present
                if let Some(finally_code) = finally_stmts {
                    return Ok(quote! {
                        {
                            #match_expr
                            #finally_code
                        }
                    });
                } else {
                    return Ok(match_expr);
                }
            }
        }

        // Fall through to existing simple_pattern_info logic
        if let Some((exception_value_str, _exception_type)) = simple_pattern_info {
            // Fall through to existing unwrap_or logic if not a match pattern
            // Convert try_stmts to string to post-process
            let try_code = quote! { #(#try_stmts)* };
            let try_str = try_code.to_string();

            // This handles the case where int(str) generates .parse().unwrap_or_default()
            // but we want .parse().unwrap_or(-1) based on the except clause
            if try_str.contains("unwrap_or_default") {
                // Parse the try code and replace unwrap_or_default with unwrap_or(value)
                // Handle both "unwrap_or_default ()" and "unwrap_or_default()"
                let fixed_code = try_str
                    .replace(
                        "unwrap_or_default ()",
                        &format!("unwrap_or ({})", exception_value_str),
                    )
                    .replace(
                        "unwrap_or_default()",
                        &format!("unwrap_or({})", exception_value_str),
                    );

                // Parse back to token stream
                let fixed_tokens: proc_macro2::TokenStream = fixed_code.parse().unwrap_or(try_code);

                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            #fixed_tokens
                            #finally_code
                        }
                    })
                } else {
                    Ok(fixed_tokens)
                }
            } else {
                // Pattern matched but no unwrap_or_default found
                // This means it's not a parse operation, so fall through to normal concatenation
                // to include the exception handler code
                let handler_code = &handler_tokens[0];
                if let Some(finally_code) = finally_stmts {
                    Ok(quote! {
                        {
                            #(#try_stmts)*
                            #handler_code
                            #finally_code
                        }
                    })
                } else {
                    Ok(quote! {
                        {
                            #(#try_stmts)*
                            #handler_code
                        }
                    })
                }
            }
        } else {
            // Execute try block statements, then if we have a single handler, use it
            if handlers.len() == 1 {
                if handlers[0].name.is_some() && body.len() == 1 {
                    if let HirStmt::Return(Some(HirExpr::Call { func, args, .. })) = &body[0] {
                        if func == "int" && args.len() == 1 {
                            // Single handler with exception binding - use match with Err(e)
                            let arg_expr = args[0].to_rust_expr(ctx)?;
                            let handler_body = &handler_tokens[0];
                            let err_var = handlers[0].name.as_ref().map(|s| safe_ident(s)).unwrap();

                            if let Some(finally_body) = finalbody {
                                let finally_stmts: Vec<_> = finally_body
                                    .iter()
                                    .map(|s| s.to_rust_tokens(ctx))
                                    .collect::<Result<Vec<_>>>()?;
                                return Ok(quote! {
                                    {
                                        match #arg_expr.parse::<i32>() {
                                            Ok(__value) => __value,
                                            Err(#err_var) => {
                                                #handler_body
                                            }
                                        }
                                        #(#finally_stmts)*
                                    }
                                });
                            } else {
                                return Ok(quote! {
                                    match #arg_expr.parse::<i32>() {
                                        Ok(__value) => __value,
                                        Err(#err_var) => {
                                            #handler_body
                                        }
                                    }
                                });
                            }
                        }
                    }
                }

                // If so, don't concatenate handler as it creates invalid syntax or unreachable code
                let try_code_str = quote! { #(#try_stmts)* }.to_string();
                let has_error_handling = try_code_str.contains("unwrap_or_default")
                    || try_code_str.contains("unwrap_or(")
                    || try_code_str.contains(".expect(");

                if has_error_handling {
                    // Try block already handles errors, don't add handler
                    if let Some(finally_code) = finally_stmts {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #finally_code
                            }
                        })
                    } else {
                        Ok(quote! { #(#try_stmts)* })
                    }
                } else {
                    // If so, skip handler code since we can't bind it in unconditional context
                    let has_exception_binding = handlers[0].name.is_some();

                    if has_exception_binding {
                        // Skip handler code - it would reference unbound exception variable
                        // NOTE: This means exception handlers are not fully implemented ()
                        if let Some(finally_code) = finally_stmts {
                            Ok(quote! {
                                {
                                    #(#try_stmts)*
                                    #finally_code
                                }
                            })
                        } else {
                            Ok(quote! { #(#try_stmts)* })
                        }
                    } else {
                        let handler_code = &handler_tokens[0];

                        if let Some(finally_code) = finally_stmts {
                            Ok(quote! {
                                {
                                    #(#try_stmts)*
                                    #handler_code
                                    #finally_code
                                }
                            })
                        } else {
                            // NOTE: This executes both unconditionally - need proper conditional logic ()
                            // based on which operations can panic (ZeroDivisionError, IndexError, etc.)
                            Ok(quote! {
                                {
                                    #(#try_stmts)*
                                    #handler_code
                                }
                            })
                        }
                    }
                }
            } else {
                // For operations like int(data) with multiple exception types, we need proper
                // match-based error handling instead of simple unwrap_or

                // Check if try block is simple return with parse operation
                if body.len() == 1 {
                    if let HirStmt::Return(Some(HirExpr::Call { func, args, .. })) = &body[0] {
                        if func == "int" && args.len() == 1 {
                            let arg_expr = args[0].to_rust_expr(ctx)?;

                            // Check if any handler binds the exception variable
                            let has_exception_binding = handlers.iter().any(|h| h.name.is_some());

                            if has_exception_binding && handlers.len() == 1 {
                                // Single handler with exception binding - use match with Err(e)
                                let handler_body = &handler_tokens[0];
                                let err_var =
                                    handlers[0].name.as_ref().map(|s| safe_ident(s)).unwrap();

                                if let Some(finally_code) = finally_stmts {
                                    return Ok(quote! {
                                        {
                                            match #arg_expr.parse::<i32>() {
                                                Ok(__value) => __value,
                                                Err(#err_var) => {
                                                    #handler_body
                                                }
                                            }
                                            #finally_code
                                        }
                                    });
                                } else {
                                    return Ok(quote! {
                                        match #arg_expr.parse::<i32>() {
                                            Ok(__value) => __value,
                                            Err(#err_var) => {
                                                #handler_body
                                            }
                                        }
                                    });
                                }
                            } else if handlers.len() >= 2 {
                                // Convert: try { return int(data) } except ValueError {...} except TypeError {...}
                                // To: if let Ok(v) = data.parse::<i32>() { v } else { handler1; handler2; }

                                // NOTE: Rust's parse() returns a single error type, so we can't dispatch
                                // to specific handlers. We execute all handlers sequentially.
                                // This is semantically incorrect but compiles. NOTE: Improve error dispatch logic ()

                                if let Some(finally_code) = finally_stmts {
                                    return Ok(quote! {
                                        {
                                            if let Ok(__parse_result) = #arg_expr.parse::<i32>() {
                                                __parse_result
                                            } else {
                                                #(#handler_tokens)*
                                            }
                                            #finally_code
                                        }
                                    });
                                } else {
                                    return Ok(quote! {
                                        {
                                            if let Ok(__parse_result) = #arg_expr.parse::<i32>() {
                                                __parse_result
                                            } else {
                                                #(#handler_tokens)*
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                // In that case, don't concatenate handler tokens as it creates invalid syntax
                let try_code_str = quote! { #(#try_stmts)* }.to_string();
                let has_error_handling = try_code_str.contains("unwrap_or_default")
                    || try_code_str.contains("unwrap_or(");

                if has_error_handling {
                    // Try block has built-in error handling, don't add handlers
                    if let Some(finally_code) = finally_stmts {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #finally_code
                            }
                        })
                    } else {
                        Ok(quote! { #(#try_stmts)* })
                    }
                } else {
                    // Note: Floor division with ZeroDivisionError is handled earlier (line 1366)
                    if let Some(finally_code) = finally_stmts {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #(#handler_tokens)*
                                #finally_code
                            }
                        })
                    } else {
                        Ok(quote! {
                            {
                                #(#try_stmts)*
                                #(#handler_tokens)*
                            }
                        })
                    }
                }
            }
        }
    }
}

///
/// Looks for pattern: `let var = expr.parse::<i32>().unwrap_or_default();`
/// Returns: (variable_name, parse_expression_without_unwrap_or, remaining_statements)
fn extract_parse_from_tokens(
    try_stmts: &[proc_macro2::TokenStream],
) -> Option<(String, String, Vec<proc_macro2::TokenStream>)> {
    if try_stmts.is_empty() {
        return None;
    }

    // Convert first statement to string (note: tokens have spaces between them)
    let first_stmt = try_stmts[0].to_string();

    // Pattern: let var_name = something . parse :: < i32 > () . unwrap_or_default () ;
    // Note: TokenStream.to_string() adds spaces between tokens
    if first_stmt.contains("parse") && first_stmt.contains("unwrap_or_default") {
        // Extract variable name (after "let " and before " =")
        if let Some(let_pos) = first_stmt.find("let ") {
            if let Some(eq_pos) = first_stmt[let_pos..].find(" =") {
                let var_name = first_stmt[let_pos + 4..let_pos + eq_pos].trim().to_string();

                // Extract parse expression (between "= " and "unwrap_or_default")
                // We need to find the start of unwrap_or_default and go back to find the parse call
                if let Some(eq_start) = first_stmt.find(" = ") {
                    if let Some(unwrap_pos) = first_stmt.find("unwrap_or_default") {
                        // Go back from unwrap_pos to skip ". " before it
                        let parse_end =
                            if unwrap_pos >= 2 && &first_stmt[unwrap_pos - 2..unwrap_pos] == ". " {
                                unwrap_pos - 2
                            } else {
                                unwrap_pos
                            };

                        let parse_expr = first_stmt[eq_start + 3..parse_end].trim().to_string();

                        // Collect remaining statements
                        let remaining: Vec<_> = try_stmts[1..].to_vec();

                        return Some((var_name, parse_expr, remaining));
                    }
                }
            }
        }
    }

    None
}

fn contains_floor_div(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary {
            op: BinOp::FloorDiv,
            ..
        } => true,
        HirExpr::Binary { left, right, .. } => {
            contains_floor_div(left) || contains_floor_div(right)
        }
        HirExpr::Unary { operand, .. } => contains_floor_div(operand),
        HirExpr::Call { args, .. } => args.iter().any(contains_floor_div),
        HirExpr::MethodCall { object, args, .. } => {
            contains_floor_div(object) || args.iter().any(contains_floor_div)
        }
        HirExpr::Index { base, index } => contains_floor_div(base) || contains_floor_div(index),
        HirExpr::List(elements) | HirExpr::Tuple(elements) | HirExpr::Set(elements) => {
            elements.iter().any(contains_floor_div)
        }
        _ => false,
    }
}

fn extract_divisor_from_floor_div(expr: &HirExpr) -> Result<&HirExpr> {
    match expr {
        HirExpr::Binary {
            op: BinOp::FloorDiv,
            right,
            ..
        } => Ok(right),
        HirExpr::Binary { left, right, .. } => {
            // Recursively search for floor division
            if contains_floor_div(left) {
                extract_divisor_from_floor_div(left)
            } else if contains_floor_div(right) {
                extract_divisor_from_floor_div(right)
            } else {
                bail!("No floor division found in expression")
            }
        }
        HirExpr::Unary { operand, .. } => extract_divisor_from_floor_div(operand),
        _ => bail!("No floor division found in expression"),
    }
}

///
/// # Complexity
/// 2 (pattern match + string clone)
fn extract_string_literal(expr: &HirExpr) -> String {
    match expr {
        HirExpr::Literal(Literal::String(s)) => s.clone(),
        _ => String::new(),
    }
}

///
/// # Complexity
/// 4 (iterator + filter + match)
fn extract_kwarg_string(kwargs: &[(String, HirExpr)], key: &str) -> Option<String> {
    kwargs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            HirExpr::Literal(Literal::String(s)) => Some(s.clone()),
            _ => None,
        })
}

///
/// # Complexity
/// 4 (iterator + filter + match)
fn extract_kwarg_bool(kwargs: &[(String, HirExpr)], key: &str) -> Option<bool> {
    kwargs
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| match v {
            HirExpr::Var(s) if s == "True" => Some(true),
            HirExpr::Var(s) if s == "False" => Some(false),
            _ => None,
        })
}

///
/// Detects patterns like:
/// ```python
/// if args.command == "clone":
///     handle_clone(args)
/// elif args.command == "push":
///     handle_push(args)
/// ```
///
/// And converts to:
/// ```rust,ignore
/// match args.command {
///     Commands::Clone { url } => {
///         handle_clone(args);
///     }
///     Commands::Push { remote } => {
///         handle_push(args);
///     }
/// }
/// ```
fn try_generate_subcommand_match(
    condition: &HirExpr,
    then_body: &[HirStmt],
    else_body: &Option<Vec<HirStmt>>,
    ctx: &mut CodeGenContext,
) -> Result<Option<proc_macro2::TokenStream>> {
    use quote::{format_ident, quote};

    // Check if condition matches: args.command == "string"
    let command_name = match is_subcommand_check(condition) {
        Some(name) => name,
        None => return Ok(None),
    };

    // Collect all branches (if + elif chain)
    let mut branches = vec![(command_name, then_body)];

    // Check if else is another if statement (elif pattern)
    let mut current_else = else_body;
    while let Some(else_stmts) = current_else {
        // Check if else body is a single If statement
        if else_stmts.len() == 1 {
            if let HirStmt::If {
                condition: elif_cond,
                then_body: elif_then,
                else_body: elif_else,
            } = &else_stmts[0]
            {
                if let Some(elif_name) = is_subcommand_check(elif_cond) {
                    branches.push((elif_name, elif_then.as_slice()));
                    current_else = elif_else;
                    continue;
                }
            }
        }
        // Not an elif pattern, stop collecting
        break;
    }

    // Generate match arms
    let arms: Vec<proc_macro2::TokenStream> = branches
        .iter()
        .map(|(cmd_name, body)| {
            // Convert command name to PascalCase variant
            let variant_name = format_ident!("{}", to_pascal_case_subcommand(cmd_name));

            // Get subcommand info to extract fields
            let subcommand_info = ctx
                .argparser_tracker
                .subcommands
                .values()
                .find(|sc| sc.name == *cmd_name);

            // Generate field bindings
            let field_bindings: Vec<_> = if let Some(sc) = subcommand_info {
                sc.arguments
                    .iter()
                    .map(|arg| format_ident!("{}", arg.rust_field_name()))
                    .collect()
            } else {
                vec![]
            };

            // Generate body statements
            ctx.enter_scope();
            let body_stmts: Vec<_> = body
                .iter()
                .map(|s| s.to_rust_tokens(ctx))
                .collect::<Result<Vec<_>>>()
                .unwrap_or_default();
            ctx.exit_scope();

            if field_bindings.is_empty() {
                quote! {
                    Commands::#variant_name => {
                        #(#body_stmts)*
                    }
                }
            } else {
                quote! {
                    Commands::#variant_name { #(#field_bindings),* } => {
                        #(#body_stmts)*
                    }
                }
            }
        })
        .collect();

    Ok(Some(quote! {
        match args.command {
            #(#arms)*
        }
    }))
}

///
/// Returns the command name if pattern matches: args.command == "string"
fn is_subcommand_check(expr: &HirExpr) -> Option<String> {
    match expr {
        HirExpr::Binary {
            op: BinOp::Eq,
            left,
            right,
        } => {
            // Check if left side is args.command
            let is_command_attr = matches!(
                left.as_ref(),
                HirExpr::Attribute { attr, .. } if attr == "command"
            );

            // Check if right side is a string literal
            if is_command_attr {
                if let HirExpr::Literal(Literal::String(cmd_name)) = right.as_ref() {
                    return Some(cmd_name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

fn to_pascal_case_subcommand(s: &str) -> String {
    s.split(&['-', '_'][..])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

impl RustCodeGen for HirStmt {
    fn to_rust_tokens(&self, ctx: &mut CodeGenContext) -> Result<proc_macro2::TokenStream> {
        // Reset clone_already_applied at the start of each statement to ensure
        // proper clone detection for variables used multiple times across statements
        ctx.clone_already_applied = false;
        match self {
            HirStmt::Assign {
                target,
                value,
                type_annotation,
            } => codegen_assign_stmt(target, value, type_annotation, ctx),
            HirStmt::Return(expr) => codegen_return_stmt(expr, ctx),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => codegen_if_stmt(condition, then_body, else_body, ctx),
            HirStmt::While { condition, body } => codegen_while_stmt(condition, body, ctx),
            HirStmt::For { target, iter, body } => codegen_for_stmt(target, iter, body, ctx),
            HirStmt::Expr(expr) => codegen_expr_stmt(expr, ctx),
            HirStmt::Raise {
                exception,
                cause: _,
            } => codegen_raise_stmt(exception, ctx),
            HirStmt::Break { label } => codegen_break_stmt(label),
            HirStmt::Continue { label } => codegen_continue_stmt(label),
            HirStmt::With {
                context,
                target,
                body,
            } => codegen_with_stmt(context, target, body, ctx),
            HirStmt::Try {
                body,
                handlers,
                orelse: _,
                finalbody,
            } => codegen_try_stmt(body, handlers, finalbody, ctx),
            HirStmt::Assert { test, msg } => codegen_assert_stmt(test, msg, ctx),
            HirStmt::Pass => codegen_pass_stmt(),
            HirStmt::FunctionDef {
                name,
                params,
                ret_type,
                body,
                docstring: _,
            } => codegen_nested_function_def(name, params, ret_type, body, ctx),
            HirStmt::Global { names } => codegen_global_stmt(names),
            HirStmt::Nonlocal { names } => codegen_nonlocal_stmt(names),
            HirStmt::AsyncFor { target, iter, body } => {
                codegen_async_for_stmt(target, iter, body, ctx)
            }
            HirStmt::AsyncWith {
                context,
                target,
                body,
            } => codegen_async_with_stmt(context, target, body, ctx),
            HirStmt::Delete { targets } => codegen_delete_stmt(targets, ctx),
            HirStmt::Import { .. } => codegen_import_stmt(),
            HirStmt::ImportFrom { .. } => codegen_import_from_stmt(),
            HirStmt::AsyncFunctionDef {
                name,
                params,
                ret_type,
                body,
                docstring: _,
            } => codegen_async_nested_function_def(name, params, ret_type, body, ctx),
            HirStmt::Match { subject, cases } => codegen_match_stmt(subject, cases, ctx),
        }
    }
}

// ============================================================================
// ============================================================================

/// Convert HIR Type to proc_macro2::TokenStream for code generation
fn hir_type_to_tokens(ty: &Type, _ctx: &CodeGenContext) -> proc_macro2::TokenStream {
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

/// Generate Rust code for nested function definitions (inner functions)
///
/// Python nested functions are converted to Rust inner functions.
/// This enables code like csv_filter.py and log_analyzer.py to transpile.
///
/// # Examples
///
/// Python:
/// ```python
/// def outer():
///     def inner(x):
///         return x * 2
///     return inner(5)
/// ```
///
/// Rust:
/// ```rust
/// fn outer() -> i64 {
///     fn inner(x: i64) -> i64 {
///         x * 2
///     }
///     inner(5)
/// }
/// ```
fn codegen_nested_function_def(
    name: &str,
    params: &[HirParam],
    ret_type: &Type,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use quote::quote;

    // Generate function name
    let fn_name = syn::Ident::new(name, proc_macro2::Span::call_site());

    // Generate parameters
    let param_tokens: Vec<proc_macro2::TokenStream> = params
        .iter()
        .map(|p| {
            let param_name = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let param_type = hir_type_to_tokens(&p.ty, ctx);

            quote! { #param_name: #param_type }
        })
        .collect();

    // Generate return type
    let return_type = hir_type_to_tokens(ret_type, ctx);

    // Generate body
    let body_tokens: Vec<proc_macro2::TokenStream> = body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    // Generate inner function
    Ok(quote! {
        fn #fn_name(#(#param_tokens),*) -> #return_type {
            #(#body_tokens)*
        }
    })
}

/// Generate Rust code for async nested function definition.
fn codegen_async_nested_function_def(
    name: &str,
    params: &[HirParam],
    ret_type: &Type,
    body: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use quote::quote;

    let fn_name = syn::Ident::new(name, proc_macro2::Span::call_site());

    let param_tokens: Vec<proc_macro2::TokenStream> = params
        .iter()
        .map(|p| {
            let param_name = syn::Ident::new(&p.name, proc_macro2::Span::call_site());
            let param_type = hir_type_to_tokens(&p.ty, ctx);
            quote! { #param_name: #param_type }
        })
        .collect();

    let return_type = hir_type_to_tokens(ret_type, ctx);

    let body_tokens: Vec<proc_macro2::TokenStream> = body
        .iter()
        .map(|stmt| stmt.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        async fn #fn_name(#(#param_tokens),*) -> #return_type {
            #(#body_tokens)*
        }
    })
}

/// Generate Rust code for global statement.
/// In Rust, global is typically a no-op as variable scoping is different.
/// The actual variable needs to be declared as static at module level.
fn codegen_global_stmt(_names: &[String]) -> Result<proc_macro2::TokenStream> {
    // Global statement is a declaration marker in Python, not executable code.
    // The actual semantics depend on how the variable is used later.
    // For now, emit a comment noting the global declaration.
    Ok(quote! {})
}

/// Generate Rust code for nonlocal statement.
/// In Rust, nonlocal is typically handled via closure captures with Rc<RefCell<T>>.
fn codegen_nonlocal_stmt(_names: &[String]) -> Result<proc_macro2::TokenStream> {
    // Nonlocal statement is a declaration marker in Python, not executable code.
    // The actual semantics require tracking captured variables in closures.
    // For now, emit nothing as the closure capture handles this.
    Ok(quote! {})
}

/// Generate Rust code for import statement inside function.
/// In Rust, use statements are typically at module level. Local imports are no-ops.
fn codegen_import_stmt() -> Result<proc_macro2::TokenStream> {
    // Import statement inside a function is a no-op in Rust.
    // The module imports are handled at the module level.
    Ok(quote! {})
}

/// Generate Rust code for import-from statement inside function.
/// In Rust, use statements are typically at module level. Local imports are no-ops.
fn codegen_import_from_stmt() -> Result<proc_macro2::TokenStream> {
    // Import-from statement inside a function is a no-op in Rust.
    // The module imports are handled at the module level.
    Ok(quote! {})
}

/// Generate Rust code for async for statement.
/// Python's `async for x in stream:` becomes `while let Some(x) = stream.next().await`
fn codegen_async_for_stmt(
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

/// Generate Rust code for async with statement.
/// Python's `async with ctx as var:` becomes an async block with the resource
fn codegen_async_with_stmt(
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

/// Convert an AssignTarget to a syn::Pat for pattern matching
fn convert_assign_target_to_pattern(target: &AssignTarget) -> Result<syn::Pat> {
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

/// Generate Rust code for delete statement.
/// In Rust, `del x` becomes `drop(x)`, `del d[k]` becomes `d.remove(&k)`.
fn codegen_delete_stmt(
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

/// Generate Rust code for match statements (Python 3.10+)
fn codegen_match_stmt(
    subject: &HirExpr,
    cases: &[MatchCase],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
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

fn codegen_match_arm(
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

fn codegen_pattern(pattern: &HirPattern) -> Result<proc_macro2::TokenStream> {
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

/// Convert a HIR expression to a pattern literal for match arms
fn expr_to_pattern_literal(expr: &HirExpr) -> Result<proc_macro2::TokenStream> {
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
