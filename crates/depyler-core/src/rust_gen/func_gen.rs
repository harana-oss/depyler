//! Function code generation
//!
//! This module handles converting HIR functions to Rust token streams.
//! It includes all function conversion helpers and the HirFunction RustCodeGen trait implementation.

use crate::borrowing_context::BorrowingStrategy;
use crate::hir::*;
use crate::lifetime_analysis::LifetimeInference;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen};
use crate::rust_gen::generator_gen::codegen_generator_function;
use crate::rust_gen::type_gen::{rust_type_to_syn, update_import_needs};
use anyhow::Result;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{self, parse_quote};

// Import analyze_mutable_vars from parent module
use super::analyze_mutable_vars;

/// Check if a name is a Rust keyword that requires raw identifier syntax
/// DEPYLER-0306: Copied from expr_gen.rs to support method name keyword handling
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

/// Generate combined generic parameters (<'a, 'b, T, U: Bound>)
#[inline]
pub(crate) fn codegen_generic_params(
    type_params: &[crate::generic_inference::TypeParameter],
    lifetime_params: &[String],
) -> proc_macro2::TokenStream {
    if type_params.is_empty() && lifetime_params.is_empty() {
        return quote! {};
    }

    let mut all_params = Vec::new();

    // Add lifetime parameters first
    // Note: Filter out 'static as it's a reserved keyword in Rust and doesn't need to be declared
    for lt in lifetime_params {
        if lt != "'static" {
            let lt_ident = syn::Lifetime::new(lt, proc_macro2::Span::call_site());
            all_params.push(quote! { #lt_ident });
        }
    }

    // Add type parameters with their bounds
    for type_param in type_params {
        let param_name = syn::Ident::new(&type_param.name, proc_macro2::Span::call_site());
        if type_param.bounds.is_empty() {
            all_params.push(quote! { #param_name });
        } else {
            let bounds: Vec<_> = type_param
                .bounds
                .iter()
                .map(|b| {
                    let bound: syn::Path =
                        syn::parse_str(b).unwrap_or_else(|_| parse_quote! { Clone });
                    quote! { #bound }
                })
                .collect();
            all_params.push(quote! { #param_name: #(#bounds)+* });
        }
    }

    quote! { <#(#all_params),*> }
}

/// Generate where clause for lifetime bounds (where 'a: 'b, 'c: 'd)
#[inline]
pub(crate) fn codegen_where_clause(
    lifetime_bounds: &[(String, String)],
) -> proc_macro2::TokenStream {
    if lifetime_bounds.is_empty() {
        return quote! {};
    }

    let bounds: Vec<_> = lifetime_bounds
        .iter()
        .map(|(from, to)| {
            let from_lt = syn::Lifetime::new(from, proc_macro2::Span::call_site());
            let to_lt = syn::Lifetime::new(to, proc_macro2::Span::call_site());
            quote! { #from_lt: #to_lt }
        })
        .collect();

    quote! { where #(#bounds),* }
}

/// Generate function attributes (doc comments, panic-free, termination proofs, custom attributes)
#[inline]
pub(crate) fn codegen_function_attrs(
    docstring: &Option<String>,
    properties: &crate::hir::FunctionProperties,
    custom_attributes: &[String],
) -> Vec<proc_macro2::TokenStream> {
    let mut attrs = vec![];

    // Add docstring as documentation if present
    if let Some(docstring) = docstring {
        attrs.push(quote! {
            #[doc = #docstring]
        });
    }

    // Add custom Rust attributes
    for attr in custom_attributes {
        // Parse the attribute string as a TokenStream
        // This allows complex attributes like inline(always), repr(C), etc.
        if let Ok(tokens) = attr.parse::<proc_macro2::TokenStream>() {
            attrs.push(quote! {
                #[#tokens]
            });
        }
    }

    attrs
}

// ============================================================================
// DEPYLER-0141 Phase 2: Medium Complexity Helpers
// ============================================================================

/// Process function body statements with proper scoping
#[inline]
pub(crate) fn codegen_function_body(
    func: &HirFunction,
    can_fail: bool,
    error_type: Option<crate::rust_gen::context::ErrorType>,
    ctx: &mut CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    // Enter function scope and declare parameters
    ctx.enter_scope();
    ctx.current_function_can_fail = can_fail;
    ctx.current_return_type = Some(func.ret_type.clone());
    // DEPYLER-0310: Set error type for raise statement wrapping
    ctx.current_error_type = error_type;

    for param in &func.params {
        ctx.declare_var(&param.name);
        // Store parameter type information for set/dict disambiguation
        ctx.var_types.insert(param.name.clone(), param.ty.clone());
    }

    // DEPYLER-0312 NOTE: analyze_mutable_vars is now called in impl RustCodeGen BEFORE
    // codegen_function_params, so ctx.mutable_vars is already populated here

    // DEPYLER-0271: Convert body, marking final statement for expression-based returns
    let body_len = func.body.len();
    let body_stmts: Vec<_> = func
        .body
        .iter()
        .enumerate()
        .map(|(i, stmt)| {
            // Mark final statement for idiomatic expression-based return
            ctx.is_final_statement = i == body_len - 1;
            stmt.to_rust_tokens(ctx)
        })
        .collect::<Result<Vec<_>>>()?;

    ctx.exit_scope();
    ctx.current_function_can_fail = false;
    ctx.current_return_type = None;

    Ok(body_stmts)
}

// ============================================================================
// DEPYLER-0141 Phase 3: Complex Sections
// ============================================================================

// ========== Phase 3a: Parameter Conversion ==========

/// Convert function parameters with lifetime and borrowing analysis
#[inline]
pub(crate) fn codegen_function_params(
    func: &HirFunction,
    lifetime_result: &crate::lifetime_analysis::LifetimeResult,
    ctx: &mut CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    func.params
        .iter()
        .map(|param| codegen_single_param(param, func, lifetime_result, ctx))
        .collect()
}

/// Convert a single parameter with all borrowing strategies
fn codegen_single_param(
    param: &HirParam,
    func: &HirFunction,
    lifetime_result: &crate::lifetime_analysis::LifetimeResult,
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    // Use parameter name directly to ensure signature matches body references
    // DEPYLER-0357: Removed underscore prefixing logic that was causing compilation errors
    // Parameter names in signature must match exactly how they're referenced in function body
    let param_name = param.name.clone();
    let param_ident = syn::Ident::new(&param_name, proc_macro2::Span::call_site());

    // DEPYLER-0424: Check if this parameter is the argparse args variable
    // If so, type it as &Args instead of default type mapping
    let is_argparse_args = ctx.argparser_tracker.parsers.values().any(|parser_info| {
        parser_info
            .args_var
            .as_ref()
            .is_some_and(|args_var| args_var == &param.name)
    });

    if is_argparse_args {
        // Use &Args for argparse result parameters
        return Ok(quote! { #param_ident: &Args });
    }

    // DEPYLER-0312: Use mutable_vars populated by analyze_mutable_vars
    // This handles ALL mutation patterns: direct assignment, method calls, and parameter reassignments
    // The analyze_mutable_vars function already checked all mutation patterns in codegen_function_body
    let is_mutated_in_body = ctx.mutable_vars.contains(&param.name);

    // Only apply `mut` if ownership is taken (not borrowed)
    // Borrowed parameters (&T, &mut T) handle mutability in the type itself
    let takes_ownership = matches!(
        lifetime_result.borrowing_strategies.get(&param.name),
        Some(crate::borrowing_context::BorrowingStrategy::TakeOwnership) | None
    );

    let is_param_mutated = is_mutated_in_body && takes_ownership;

    // DEPYLER-0447: Detect argparse validator functions (tracked at add_argument() call sites)
    // These should ALWAYS have &str parameter type regardless of type inference
    // Validators are detected when processing add_argument(type=validator_func)
    let is_argparse_validator = ctx.validator_functions.contains(&func.name);

    if is_argparse_validator {
        // Argparse validators always receive string arguments from clap
        let ty = if is_param_mutated {
            quote! { mut #param_ident: &str }
        } else {
            quote! { #param_ident: &str }
        };
        return Ok(ty);
    }

    // Get the inferred parameter info
    if let Some(inferred) = lifetime_result.param_lifetimes.get(&param.name) {
        let rust_type = &inferred.rust_type;

        // Handle Union type placeholders
        let actual_rust_type =
            if let crate::type_mapper::RustType::Enum { name, variants: _ } = rust_type {
                if name == "UnionType" {
                    if let Type::Union(types) = &param.ty {
                        let enum_name = ctx.process_union_type(types);
                        crate::type_mapper::RustType::Custom(enum_name)
                    } else {
                        rust_type.clone()
                    }
                } else {
                    rust_type.clone()
                }
            } else {
                rust_type.clone()
            };

        update_import_needs(ctx, &actual_rust_type);

        // DEPYLER-0330: Override needs_mut for borrowed parameters that are mutated
        // If analyze_mutable_vars detected mutation (via .remove(), .clear(), etc.)
        // and this parameter will be borrowed (&T), upgrade to &mut T
        let mut inferred_with_mut = inferred.clone();
        if is_mutated_in_body && inferred.should_borrow {
            inferred_with_mut.needs_mut = true;
        }

        let ty = apply_param_borrowing_strategy(
            &param.name,
            &actual_rust_type,
            &inferred_with_mut,
            lifetime_result,
            ctx,
        )?;

        // Track which params are borrowed so call-site generation avoids double-referencing
        if inferred_with_mut.should_borrow {
            if inferred_with_mut.needs_mut {
                ctx.current_func_mut_ref_params.insert(param.name.clone());
            } else {
                ctx.current_func_ref_params.insert(param.name.clone());
            }
        }

        Ok(if is_param_mutated {
            quote! { mut #param_ident: #ty }
        } else {
            quote! { #param_ident: #ty }
        })
    } else {
        // Fallback to original mapping
        let rust_type = ctx
            .annotation_aware_mapper
            .map_type_with_annotations(&param.ty, &func.annotations);
        update_import_needs(ctx, &rust_type);
        let ty = rust_type_to_syn(&rust_type)?;

        Ok(if is_param_mutated {
            quote! { mut #param_ident: #ty }
        } else {
            quote! { #param_ident: #ty }
        })
    }
}

/// Apply borrowing strategy to parameter type
fn apply_param_borrowing_strategy(
    param_name: &str,
    rust_type: &crate::type_mapper::RustType,
    inferred: &crate::lifetime_analysis::InferredParam,
    lifetime_result: &crate::lifetime_analysis::LifetimeResult,
    ctx: &mut CodeGenContext,
) -> Result<syn::Type> {
    let mut ty = rust_type_to_syn(rust_type)?;

    // DEPYLER-0275: Check if lifetimes should be elided
    // If lifetime_params is empty, Rust's elision rules apply - don't add explicit lifetimes
    let should_elide_lifetimes = lifetime_result.lifetime_params.is_empty();

    // Check if we have a borrowing strategy
    if let Some(strategy) = lifetime_result.borrowing_strategies.get(param_name) {
        match strategy {
            crate::borrowing_context::BorrowingStrategy::UseCow { lifetime } => {
                ctx.needs_cow = true;

                // DEPYLER-0282 FIX: Parameters should NEVER use 'static lifetime
                // For parameters, we need borrowed data that can be passed from local scope
                // Use generic lifetime or elide it - never 'static for parameters
                if should_elide_lifetimes {
                    // Elide lifetime - let Rust infer it
                    ty = parse_quote! { Cow<'_, str> };
                } else if lifetime == "'static" {
                    // CRITICAL FIX: Don't use 'static for parameters!
                    // If inference suggested 'static, use generic lifetime instead
                    // This allows passing local Strings/&str to the function
                    if let Some(first_lifetime) = lifetime_result.lifetime_params.first() {
                        let lt = syn::Lifetime::new(first_lifetime, proc_macro2::Span::call_site());
                        ty = parse_quote! { Cow<#lt, str> };
                    } else {
                        // No explicit lifetimes - use elision
                        ty = parse_quote! { Cow<'_, str> };
                    }
                } else {
                    // Use the provided non-static lifetime
                    let lt = syn::Lifetime::new(lifetime, proc_macro2::Span::call_site());
                    ty = parse_quote! { Cow<#lt, str> };
                }
            }
            _ => {
                // Apply normal borrowing if needed
                if inferred.should_borrow {
                    ty = apply_borrowing_to_type(ty, rust_type, inferred, should_elide_lifetimes)?;
                }
            }
        }
    } else {
        // Fallback to normal borrowing
        if inferred.should_borrow {
            ty = apply_borrowing_to_type(ty, rust_type, inferred, should_elide_lifetimes)?;
        }
    }

    Ok(ty)
}

/// Apply borrowing (&, &mut, with lifetime) to a type
/// DEPYLER-0275: Added should_elide_lifetimes parameter to respect Rust elision rules
fn apply_borrowing_to_type(
    mut ty: syn::Type,
    rust_type: &crate::type_mapper::RustType,
    inferred: &crate::lifetime_analysis::InferredParam,
    should_elide_lifetimes: bool,
) -> Result<syn::Type> {
    // Special case for strings: use &str instead of &String
    if matches!(rust_type, crate::type_mapper::RustType::String) {
        // DEPYLER-0275: Elide lifetime if elision rules apply
        if should_elide_lifetimes || inferred.lifetime.is_none() {
            ty = if inferred.needs_mut {
                parse_quote! { &mut str }
            } else {
                parse_quote! { &str }
            };
        } else if let Some(ref lifetime) = inferred.lifetime {
            let lt = syn::Lifetime::new(lifetime.as_str(), proc_macro2::Span::call_site());
            ty = if inferred.needs_mut {
                parse_quote! { &#lt mut str }
            } else {
                parse_quote! { &#lt str }
            };
        } else {
            ty = if inferred.needs_mut {
                parse_quote! { &mut str }
            } else {
                parse_quote! { &str }
            };
        }
    } else {
        // Non-string types
        // DEPYLER-0275: Elide lifetime if elision rules apply
        if should_elide_lifetimes || inferred.lifetime.is_none() {
            ty = if inferred.needs_mut {
                parse_quote! { &mut #ty }
            } else {
                parse_quote! { &#ty }
            };
        } else if let Some(ref lifetime) = inferred.lifetime {
            let lt = syn::Lifetime::new(lifetime.as_str(), proc_macro2::Span::call_site());
            ty = if inferred.needs_mut {
                parse_quote! { &#lt mut #ty }
            } else {
                parse_quote! { &#lt #ty }
            };
        } else {
            ty = if inferred.needs_mut {
                parse_quote! { &mut #ty }
            } else {
                parse_quote! { &#ty }
            };
        }
    }

    Ok(ty)
}

// ========== String Method Return Type Analysis (v3.16.0) ==========

/// Classification of string methods by their return type semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StringMethodReturnType {
    /// Returns owned String (e.g., upper, lower, strip, replace)
    Owned,
    /// Returns borrowed &str or bool (e.g., starts_with, is_digit)
    Borrowed,
}

/// Classify a string method by its return type semantics
fn classify_string_method(method_name: &str) -> StringMethodReturnType {
    match method_name {
        // Transformation methods that return owned String
        "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "replace" | "format" | "title"
        | "capitalize" | "swapcase" | "expandtabs" | "center" | "ljust" | "rjust" | "zfill" => {
            StringMethodReturnType::Owned
        }

        // Query/test methods that return bool or &str (borrowed)
        "startswith" | "endswith" | "isalpha" | "isdigit" | "isalnum" | "isspace" | "islower"
        | "isupper" | "istitle" | "isascii" | "isprintable" | "find" | "rfind" | "index"
        | "rindex" | "count" => StringMethodReturnType::Borrowed,

        // Default: assume owned to be safe
        _ => StringMethodReturnType::Owned,
    }
}

/// Check if an expression contains a string method call that returns owned String
fn contains_owned_string_method(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::MethodCall { method, .. } => {
            // Check if this method returns owned String
            classify_string_method(method) == StringMethodReturnType::Owned
        }
        HirExpr::Binary { left, right, .. } => {
            // Check both sides of binary operations
            contains_owned_string_method(left) || contains_owned_string_method(right)
        }
        HirExpr::Unary { operand, .. } => contains_owned_string_method(operand),
        HirExpr::IfExpr { body, orelse, .. } => {
            // Check both branches of conditional
            contains_owned_string_method(body) || contains_owned_string_method(orelse)
        }
        HirExpr::Call { .. }
        | HirExpr::Var(_)
        | HirExpr::Literal(_)
        | HirExpr::List(_)
        | HirExpr::Dict(_)
        | HirExpr::Tuple(_)
        | HirExpr::Set(_)
        | HirExpr::FrozenSet(_)
        | HirExpr::Index { .. }
        | HirExpr::Slice { .. }
        | HirExpr::Attribute { .. }
        | HirExpr::Borrow { .. }
        | HirExpr::ListComp { .. }
        | HirExpr::SetComp { .. }
        | HirExpr::DictComp { .. }
        | HirExpr::Lambda { .. }
        | HirExpr::Await { .. }
        | HirExpr::FString { .. }
        | HirExpr::Yield { .. }
        | HirExpr::SortByKey { .. }
        | HirExpr::GeneratorExp { .. }
        | HirExpr::FlattenedListComp { .. }
        | HirExpr::Uninitialized
        | HirExpr::NamedExpr { .. } => false,
    }
}

/// Check if the function's return expressions contain owned-returning string methods
fn function_returns_owned_string(func: &HirFunction) -> bool {
    // Check all return statements in the function body
    for stmt in &func.body {
        if let HirStmt::Return(Some(expr)) = stmt {
            if contains_owned_string_method(expr) {
                return true;
            }
        }
    }
    false
}

// DEPYLER-0270: String Concatenation Detection

/// Check if an expression contains string concatenation (which returns owned String)
fn contains_string_concatenation(expr: &HirExpr) -> bool {
    match expr {
        // String concatenation: a + b (Add operator generates format!() for strings)
        HirExpr::Binary { op: BinOp::Add, .. } => {
            // Binary Add on strings generates format!() which returns String
            // We detect this by assuming any Add at top level is string concat
            // (numeric Add is handled differently in code generation)
            true
        }
        // F-strings generate format!() which returns String
        HirExpr::FString { .. } => true,
        // Recursive checks for nested expressions
        HirExpr::Binary { left, right, .. } => {
            contains_string_concatenation(left) || contains_string_concatenation(right)
        }
        HirExpr::Unary { operand, .. } => contains_string_concatenation(operand),
        HirExpr::IfExpr { body, orelse, .. } => {
            contains_string_concatenation(body) || contains_string_concatenation(orelse)
        }
        _ => false,
    }
}

/// Check if function returns string concatenation
fn function_returns_string_concatenation(func: &HirFunction) -> bool {
    for stmt in &func.body {
        if let HirStmt::Return(Some(expr)) = stmt {
            if contains_string_concatenation(expr) {
                return true;
            }
        }
    }
    false
}

/// Check if a type expects float values (recursively checks Option, Result, etc.)
pub(crate) fn return_type_expects_float(ty: &Type) -> bool {
    match ty {
        Type::Float => true,
        Type::Optional(inner) => return_type_expects_float(inner),
        Type::List(inner) => return_type_expects_float(inner),
        Type::Tuple(types) => types.iter().any(return_type_expects_float),
        _ => false,
    }
}

// ========== DEPYLER-0410: Return Type Inference from Body ==========

/// Infer return type from function body when no annotation is provided
/// Returns None if type cannot be inferred or there are no return statements
fn infer_return_type_from_body(
    body: &[HirStmt],
    params: &[crate::hir::HirParam],
    class_field_types: &std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
) -> Option<Type> {
    // DEPYLER-0415: Build type environment from variable assignments
    let mut var_types: std::collections::HashMap<String, Type> = std::collections::HashMap::new();

    // Seed with function parameter types
    for param in params {
        if !matches!(param.ty, Type::Unknown) {
            var_types.insert(param.name.clone(), param.ty.clone());
        }
    }

    build_var_type_env(body, &mut var_types, class_field_types);

    let mut return_types = Vec::new();
    collect_return_types_with_env(body, &mut return_types, &var_types, class_field_types);

    // DEPYLER-0412: Also check for trailing expression (implicit return)
    // If the last statement is an expression without return, it's an implicit return
    if let Some(HirStmt::Expr(expr)) = body.last() {
        let trailing_type =
            infer_expr_type_with_class_env(expr, &var_types, class_field_types);
        if !matches!(trailing_type, Type::Unknown) {
            return_types.push(trailing_type);
        }
    }

    if return_types.is_empty() {
        return None;
    }

    // If all return types are the same (ignoring Unknown), use that type
    let first_known = return_types.iter().find(|t| !matches!(t, Type::Unknown));
    if let Some(first) = first_known {
        if return_types
            .iter()
            .all(|t| matches!(t, Type::Unknown) || t == first)
        {
            return Some(first.clone());
        }
    }

    // DEPYLER-0448: Do NOT default Unknown to Int - this causes dict/list/Value returns
    // to be incorrectly typed as i32. Instead, return None and let the type mapper
    // handle the fallback (which will use serde_json::Value for complex types).
    //
    // Previous behavior (DEPYLER-0422): Defaulted Unknown → Int for lambda returns
    // Problem: This also affected dict/list returns, causing E0308 errors
    // New behavior: Return None for Unknown types, allowing proper Value fallback
    if return_types.iter().all(|t| matches!(t, Type::Unknown)) {
        // We have return statements but all returned Unknown types
        // Don't assume Int - let type mapper decide the appropriate fallback
        return None;
    }

    // Try element-level unification for tuple return types
    // When multiple returns yield tuples of the same arity, merge per-element:
    // if one path returns None and another returns a concrete type, wrap in Optional
    if let Some(unified) = try_unify_tuple_return_types(&return_types) {
        return Some(unified);
    }

    // Mixed types - return the first known type
    first_known.cloned()
}

/// Attempt element-level unification of tuple return types.
/// E.g. returns of `(Player, int)` and `(None, int)` unify to `(Optional(Player), int)`.
fn try_unify_tuple_return_types(types: &[Type]) -> Option<Type> {
    // All non-Unknown types must be tuples
    let tuples: Vec<&Vec<Type>> = types
        .iter()
        .filter_map(|t| match t {
            Type::Tuple(elems) => Some(elems),
            Type::Unknown => None,
            _ => return None, // non-tuple, non-Unknown → bail
        })
        .collect();

    if tuples.len() < 2 {
        return None;
    }

    // Ensure any non-Unknown, non-Tuple entry causes a bail
    if types
        .iter()
        .any(|t| !matches!(t, Type::Tuple(_) | Type::Unknown))
    {
        return None;
    }

    let len = tuples[0].len();
    if len == 0 || !tuples.iter().all(|t| t.len() == len) {
        return None;
    }

    let mut result_elems = Vec::with_capacity(len);
    for i in 0..len {
        let elem_types: Vec<&Type> = tuples.iter().map(|t| &t[i]).collect();
        result_elems.push(unify_element_types(&elem_types));
    }

    Some(Type::Tuple(result_elems))
}

/// Unify a single tuple element across return paths.
/// If any path returns `None` and another returns a concrete type, wrap in `Optional`.
fn unify_element_types(types: &[&Type]) -> Type {
    let has_none = types.iter().any(|t| matches!(t, Type::None));
    let concrete: Vec<&Type> = types
        .iter()
        .copied()
        .filter(|t| !matches!(t, Type::Unknown | Type::None))
        .collect();

    if concrete.is_empty() {
        if has_none {
            Type::Optional(Box::new(Type::Unknown))
        } else {
            Type::Unknown
        }
    } else {
        // Use the first concrete type as the base
        let base = concrete[0].clone();
        if has_none {
            // Already Optional → don't double-wrap
            if matches!(base, Type::Optional(_)) {
                base
            } else {
                Type::Optional(Box::new(base))
            }
        } else {
            base
        }
    }
}

// ========== DEPYLER-0415: Variable Type Environment ==========

/// Build a type environment by collecting variable assignments
fn build_var_type_env(
    stmts: &[HirStmt],
    var_types: &mut std::collections::HashMap<String, Type>,
    class_field_types: &std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
) {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: crate::hir::AssignTarget::Symbol(name),
                value,
                ..
            } => {
                let value_type =
                    infer_expr_type_with_class_env(value, var_types, class_field_types);
                if !matches!(value_type, Type::Unknown) {
                    var_types.insert(name.clone(), value_type);
                }
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                build_var_type_env(then_body, var_types, class_field_types);
                if let Some(else_stmts) = else_body {
                    build_var_type_env(else_stmts, var_types, class_field_types);
                }
            }
            HirStmt::While { body, .. } => {
                build_var_type_env(body, var_types, class_field_types);
            }
            HirStmt::For { target, iter, body } => {
                // Infer loop variable type from iterator element type
                let iter_type =
                    infer_expr_type_with_class_env(iter, var_types, class_field_types);
                let elem_type = match &iter_type {
                    Type::List(elem) | Type::Set(elem) => Some(*elem.clone()),
                    Type::Array { element_type, .. } => Some(*element_type.clone()),
                    Type::Dict(key, _) => Some(*key.clone()),
                    Type::String => Some(Type::String),
                    _ => None,
                };
                if let (Some(elem_ty), crate::hir::AssignTarget::Symbol(name)) =
                    (elem_type, target)
                {
                    if !matches!(elem_ty, Type::Unknown) {
                        var_types.insert(name.clone(), elem_ty);
                    }
                }
                build_var_type_env(body, var_types, class_field_types);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                build_var_type_env(body, var_types, class_field_types);
                for handler in handlers {
                    build_var_type_env(&handler.body, var_types, class_field_types);
                }
                if let Some(orelse_stmts) = orelse {
                    build_var_type_env(orelse_stmts, var_types, class_field_types);
                }
                if let Some(finally_stmts) = finalbody {
                    build_var_type_env(finally_stmts, var_types, class_field_types);
                }
            }
            HirStmt::With { body, .. } => {
                build_var_type_env(body, var_types, class_field_types);
            }
            _ => {}
        }
    }
}

/// Collect return types with access to variable type environment
fn collect_return_types_with_env(
    stmts: &[HirStmt],
    types: &mut Vec<Type>,
    var_types: &std::collections::HashMap<String, Type>,
    class_field_types: &std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
) {
    for stmt in stmts {
        match stmt {
            HirStmt::Return(Some(expr)) => {
                types.push(infer_expr_type_with_class_env(
                    expr,
                    var_types,
                    class_field_types,
                ));
            }
            HirStmt::Return(None) => {
                types.push(Type::None);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_return_types_with_env(then_body, types, var_types, class_field_types);
                if let Some(else_stmts) = else_body {
                    collect_return_types_with_env(else_stmts, types, var_types, class_field_types);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                collect_return_types_with_env(body, types, var_types, class_field_types);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                collect_return_types_with_env(body, types, var_types, class_field_types);
                for handler in handlers {
                    collect_return_types_with_env(
                        &handler.body,
                        types,
                        var_types,
                        class_field_types,
                    );
                }
                if let Some(orelse_stmts) = orelse {
                    collect_return_types_with_env(
                        orelse_stmts,
                        types,
                        var_types,
                        class_field_types,
                    );
                }
                if let Some(finally_stmts) = finalbody {
                    collect_return_types_with_env(
                        finally_stmts,
                        types,
                        var_types,
                        class_field_types,
                    );
                }
            }
            HirStmt::With { body, .. } => {
                collect_return_types_with_env(body, types, var_types, class_field_types);
            }
            _ => {}
        }
    }
}

/// Infer expression type with class field type resolution.
/// Used during return type inference where struct field types are needed.
fn infer_expr_type_with_class_env(
    expr: &HirExpr,
    var_types: &std::collections::HashMap<String, Type>,
    class_field_types: &std::collections::HashMap<String, std::collections::HashMap<String, Type>>,
) -> Type {
    match expr {
        // Resolve attribute access via class field types (e.g., state.players → List(Player))
        HirExpr::Attribute { value, attr } => {
            if let HirExpr::Var(var_name) = value.as_ref() {
                if let Some(Type::Custom(class_name)) = var_types.get(var_name) {
                    if let Some(field_types) = class_field_types.get(class_name) {
                        if let Some(ft) = field_types.get(attr.as_str()) {
                            return ft.clone();
                        }
                    }
                }
            }
            infer_expr_type_with_env(expr, var_types)
        }
        // Resolve index access using class-aware base type (e.g., state.players[idx])
        HirExpr::Index { base, .. } => {
            let base_type =
                infer_expr_type_with_class_env(base, var_types, class_field_types);
            match base_type {
                Type::List(elem) => *elem,
                Type::Tuple(elems) => elems.first().cloned().unwrap_or(Type::Unknown),
                Type::Dict(_, val) => *val,
                Type::String => Type::String,
                Type::Array { element_type, .. } => *element_type,
                _ => Type::Unknown,
            }
        }
        // Resolve tuples element-wise with class context
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems
                .iter()
                .map(|e| infer_expr_type_with_class_env(e, var_types, class_field_types))
                .collect();
            Type::Tuple(elem_types)
        }
        // All other expressions: delegate to the standard env-aware inference
        _ => infer_expr_type_with_env(expr, var_types),
    }
}

/// Infer the element type yielded by an iterator expression.
/// For `range(...)` → Int, for a variable with `List(T)` type → T, etc.
pub(crate) fn infer_iter_element_type(
    iter: &HirExpr,
    var_types: &std::collections::HashMap<String, Type>,
) -> Type {
    match iter {
        HirExpr::Call { func, .. } if func == "range" => Type::Int,
        HirExpr::Var(name) => {
            if let Some(var_type) = var_types.get(name) {
                match var_type {
                    Type::List(elem) | Type::Set(elem) => *elem.clone(),
                    _ => Type::Unknown,
                }
            } else {
                Type::Unknown
            }
        }
        _ => Type::Unknown,
    }
}

/// Infer expression type with access to variable type environment
pub(crate) fn infer_expr_type_with_env(
    expr: &HirExpr,
    var_types: &std::collections::HashMap<String, Type>,
) -> Type {
    match expr {
        // DEPYLER-0415: Look up variable types in the environment
        HirExpr::Var(name) => var_types.get(name).cloned().unwrap_or(Type::Unknown),
        // For other expressions, delegate to the simple version
        // but recurse with environment for nested expressions
        HirExpr::Binary { op, left, right } => {
            if matches!(
                op,
                BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::In
                    | BinOp::NotIn
            ) {
                return Type::Bool;
            }

            // DEPYLER-0420: Detect array repeat patterns: [elem] * n or n * [elem]
            if matches!(op, BinOp::Mul) {
                match (left.as_ref(), right.as_ref()) {
                    // Pattern: [elem] * n
                    (HirExpr::List(elems), &HirExpr::Literal(Literal::Int(size)))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_with_env(&elems[0], var_types);
                        return if size <= 32 {
                            Type::Array {
                                element_type: Box::new(elem_type),
                                size: ConstGeneric::Literal(size as usize),
                            }
                        } else {
                            Type::List(Box::new(elem_type))
                        };
                    }
                    // Pattern: n * [elem]
                    (&HirExpr::Literal(Literal::Int(size)), HirExpr::List(elems))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_with_env(&elems[0], var_types);
                        return if size <= 32 {
                            Type::Array {
                                element_type: Box::new(elem_type),
                                size: ConstGeneric::Literal(size as usize),
                            }
                        } else {
                            Type::List(Box::new(elem_type))
                        };
                    }
                    _ => {}
                }
            }

            // Division always produces float in Python
            if matches!(op, BinOp::Div | BinOp::Pow) {
                return Type::Float;
            }

            let left_type = infer_expr_type_with_env(left, var_types);
            let right_type = infer_expr_type_with_env(right, var_types);
            if matches!(left_type, Type::Float) || matches!(right_type, Type::Float) {
                Type::Float
            } else if !matches!(left_type, Type::Unknown) {
                left_type
            } else {
                right_type
            }
        }
        HirExpr::IfExpr { body, orelse, .. } => {
            let body_type = infer_expr_type_with_env(body, var_types);
            if !matches!(body_type, Type::Unknown) {
                body_type
            } else {
                infer_expr_type_with_env(orelse, var_types)
            }
        }
        // DEPYLER-0420: Handle tuples with environment for variable lookups
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems
                .iter()
                .map(|e| infer_expr_type_with_env(e, var_types))
                .collect();
            Type::Tuple(elem_types)
        }
        // Handle Call expressions with environment context for type-preserving builtins
        HirExpr::Call { func, args, .. } => match func.as_str() {
            "min" | "max" | "abs" | "sum" => {
                if args
                    .iter()
                    .any(|arg| matches!(infer_expr_type_with_env(arg, var_types), Type::Float))
                {
                    Type::Float
                } else {
                    Type::Int
                }
            }
            "float" => Type::Float,
            "divmod" => Type::Tuple(vec![Type::Int, Type::Int]),
            _ => infer_expr_type_simple(expr),
        },
        // Handle method calls with environment context for math module methods
        HirExpr::MethodCall { object, method, .. } => {
            if matches!(object.as_ref(), HirExpr::Var(name) if name == "math")
                && matches!(
                    method.as_str(),
                    "exp"
                        | "log"
                        | "log2"
                        | "log10"
                        | "sqrt"
                        | "sin"
                        | "cos"
                        | "tan"
                        | "asin"
                        | "acos"
                        | "atan"
                        | "atan2"
                        | "sinh"
                        | "cosh"
                        | "tanh"
                        | "asinh"
                        | "acosh"
                        | "atanh"
                        | "ceil"
                        | "floor"
                        | "fabs"
                        | "degrees"
                        | "radians"
                        | "hypot"
                        | "pow"
                        | "ldexp"
                        | "fmod"
                        | "copysign"
                        | "remainder"
                        | "erf"
                        | "erfc"
                        | "gamma"
                        | "lgamma"
                )
            {
                Type::Float
            } else {
                infer_expr_type_simple(expr)
            }
        }
        // For other cases, use the simple version
        _ => infer_expr_type_simple(expr),
    }
}

// NOTE: collect_return_types() removed - replaced by collect_return_types_with_env()
// which provides better type inference using variable type environment (DEPYLER-0415)

/// Simple expression type inference without context
/// Handles common cases like literals, comparisons, and arithmetic
fn infer_expr_type_simple(expr: &HirExpr) -> Type {
    match expr {
        HirExpr::Literal(lit) => literal_to_type(lit),
        HirExpr::Binary { op, left, right } => {
            // Comparison operators always return bool
            if matches!(
                op,
                BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::In
                    | BinOp::NotIn
            ) {
                return Type::Bool;
            }

            // DEPYLER-0420: Detect array repeat patterns: [elem] * n or n * [elem]
            if matches!(op, BinOp::Mul) {
                match (left.as_ref(), right.as_ref()) {
                    // Pattern: [elem] * n
                    (HirExpr::List(elems), &HirExpr::Literal(Literal::Int(size)))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_simple(&elems[0]);
                        return if size <= 32 {
                            Type::Array {
                                element_type: Box::new(elem_type),
                                size: ConstGeneric::Literal(size as usize),
                            }
                        } else {
                            Type::List(Box::new(elem_type))
                        };
                    }
                    // Pattern: n * [elem]
                    (&HirExpr::Literal(Literal::Int(size)), HirExpr::List(elems))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_simple(&elems[0]);
                        return if size <= 32 {
                            Type::Array {
                                element_type: Box::new(elem_type),
                                size: ConstGeneric::Literal(size as usize),
                            }
                        } else {
                            Type::List(Box::new(elem_type))
                        };
                    }
                    _ => {}
                }
            }

            // For arithmetic, infer from operands
            let left_type = infer_expr_type_simple(left);
            let right_type = infer_expr_type_simple(right);
            // Float takes precedence
            if matches!(left_type, Type::Float) || matches!(right_type, Type::Float) {
                Type::Float
            } else if !matches!(left_type, Type::Unknown) {
                left_type
            } else {
                right_type
            }
        }
        HirExpr::Unary { op, operand } => {
            if matches!(op, UnaryOp::Not) {
                Type::Bool
            } else {
                infer_expr_type_simple(operand)
            }
        }
        HirExpr::List(elems) => {
            if elems.is_empty() {
                Type::List(Box::new(Type::Unknown))
            } else {
                Type::List(Box::new(infer_expr_type_simple(&elems[0])))
            }
        }
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems.iter().map(infer_expr_type_simple).collect();
            Type::Tuple(elem_types)
        }
        HirExpr::Set(elems) => {
            if elems.is_empty() {
                Type::Set(Box::new(Type::Unknown))
            } else {
                Type::Set(Box::new(infer_expr_type_simple(&elems[0])))
            }
        }
        HirExpr::Dict(pairs) => {
            if pairs.is_empty() {
                Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown))
            } else {
                let key_type = infer_expr_type_simple(&pairs[0].0);
                let val_type = infer_expr_type_simple(&pairs[0].1);
                Type::Dict(Box::new(key_type), Box::new(val_type))
            }
        }
        HirExpr::IfExpr { body, orelse, .. } => {
            // Try to infer from either branch
            let body_type = infer_expr_type_simple(body);
            if !matches!(body_type, Type::Unknown) {
                body_type
            } else {
                infer_expr_type_simple(orelse)
            }
        }
        // DEPYLER-0414: Add Index expression type inference
        HirExpr::Index { base, .. } => {
            // For arr[i], return element type of the container
            match infer_expr_type_simple(base) {
                Type::List(elem) => *elem,
                Type::Tuple(elems) => elems.first().cloned().unwrap_or(Type::Unknown),
                Type::Dict(_, val) => *val,
                Type::String => Type::String,
                Type::Array { element_type, .. } => *element_type,
                _ => Type::Unknown,
            }
        }
        // DEPYLER-0414: Add Slice expression type inference
        HirExpr::Slice { base, .. } => {
            // Slicing returns same container type
            infer_expr_type_simple(base)
        }
        // DEPYLER-0414: Add FString type inference (always String)
        HirExpr::FString { .. } => Type::String,
        // DEPYLER-0414: Add Call expression type inference
        HirExpr::Call { func, .. } => {
            // Common builtin functions with known return types
            match func.as_str() {
                "len" | "int" | "abs" | "ord" | "hash" => Type::Int,
                "float" => Type::Float,
                "str" | "repr" | "chr" | "input" => Type::String,
                "bool" => Type::Bool,
                "list" => Type::List(Box::new(Type::Unknown)),
                "dict" => Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)),
                "set" => Type::Set(Box::new(Type::Unknown)),
                "tuple" => Type::Tuple(vec![]),
                "range" => Type::List(Box::new(Type::Int)),
                "sum" | "min" | "max" => Type::Int, // Common numeric aggregations
                "zeros" | "ones" | "full" => Type::List(Box::new(Type::Int)),
                _ => Type::Unknown,
            }
        }
        // DEPYLER-0414: Add MethodCall expression type inference
        HirExpr::MethodCall { object, method, .. } => {
            match method.as_str() {
                // String methods that return String
                "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "replace" | "title"
                | "capitalize" | "join" | "format" => Type::String,
                // String methods that return bool
                "startswith" | "endswith" | "isdigit" | "isalpha" | "isalnum" | "isupper"
                | "islower" => Type::Bool,
                // String methods that return int
                "find" | "rfind" | "index" | "rindex" | "count" => Type::Int,
                // String methods that return list
                "split" | "splitlines" => Type::List(Box::new(Type::String)),
                // List/Dict methods
                "get" => {
                    // dict.get() returns element type
                    match infer_expr_type_simple(object) {
                        Type::Dict(_, val) => *val,
                        Type::List(elem) => *elem,
                        _ => Type::Unknown,
                    }
                }
                "pop" => match infer_expr_type_simple(object) {
                    Type::List(elem) => *elem,
                    Type::Dict(_, val) => *val,
                    _ => Type::Unknown,
                },
                "keys" => Type::List(Box::new(Type::Unknown)),
                "values" => Type::List(Box::new(Type::Unknown)),
                "items" => Type::List(Box::new(Type::Tuple(vec![Type::Unknown, Type::Unknown]))),
                // Math module functions (math.exp, math.sin, etc.) always return float
                "exp" | "log" | "log2" | "log10" | "sqrt" | "sin" | "cos" | "tan" | "asin"
                | "acos" | "atan" | "atan2" | "sinh" | "cosh" | "tanh" | "asinh" | "acosh"
                | "atanh" | "ceil" | "floor" | "fabs" | "degrees" | "radians" | "hypot" | "pow"
                | "ldexp" | "fmod" | "copysign" | "remainder" | "erf" | "erfc" | "gamma"
                | "lgamma"
                    if matches!(object.as_ref(), HirExpr::Var(name) if name == "math") =>
                {
                    Type::Float
                }
                _ => Type::Unknown,
            }
        }
        // DEPYLER-0414: Add ListComp type inference
        HirExpr::ListComp { element, .. } => Type::List(Box::new(infer_expr_type_simple(element))),
        // DEPYLER-0414: Add SetComp type inference
        HirExpr::SetComp { element, .. } => Type::Set(Box::new(infer_expr_type_simple(element))),
        // DEPYLER-0414: Add DictComp type inference
        HirExpr::DictComp { key, value, .. } => Type::Dict(
            Box::new(infer_expr_type_simple(key)),
            Box::new(infer_expr_type_simple(value)),
        ),
        // DEPYLER-0414: Add Attribute type inference
        HirExpr::Attribute { attr, .. } => {
            // Common attributes with known types
            match attr.as_str() {
                "real" | "imag" => Type::Float,
                _ => Type::Unknown,
            }
        }
        _ => Type::Unknown,
    }
}

/// Convert literal to type
fn literal_to_type(lit: &Literal) -> Type {
    match lit {
        Literal::Int(_) => Type::Int,
        Literal::Float(_) => Type::Float,
        Literal::String(_) => Type::String,
        Literal::Bool(_) => Type::Bool,
        Literal::None => Type::None,
        Literal::Bytes(_) => Type::Unknown, // No direct Bytes type in Type enum
        Literal::Ellipsis => Type::Unknown,
        Literal::Complex(_, _) => Type::Unknown, // Complex numbers not yet supported
    }
}

// ========== Indexed Field Return Reference Detection ==========

/// Detect when return tuple elements hold references from indexed field access
/// on borrowed parameters. E.g., `player = state.players[idx]` then `return (player, idx)`.
/// Returns the reference positions for the return tuple.
fn detect_indexed_field_return_refs(
    func: &HirFunction,
    lifetime_result: &crate::lifetime_analysis::LifetimeResult,
    borrowable_vars: &HashSet<String>,
) -> Vec<bool> {
    let borrowed_params: HashSet<String> = lifetime_result
        .param_lifetimes
        .iter()
        .filter(|(_, inf)| inf.should_borrow)
        .map(|(name, _)| name.clone())
        .collect();

    if borrowed_params.is_empty() {
        return vec![];
    }

    // Find variables assigned from indexed attribute access on borrowed params
    let mut ref_var_sources: HashMap<String, String> = HashMap::new();
    collect_indexed_field_vars(&func.body, &borrowed_params, &mut ref_var_sources);

    // Only keep vars that are actually borrowable (the assignment codegen will add &)
    ref_var_sources.retain(|var_name, _| borrowable_vars.contains(var_name));

    if ref_var_sources.is_empty() {
        return vec![];
    }

    // Find return tuple positions that reference these variables
    let mut ref_positions = Vec::new();
    find_ref_return_positions(&func.body, &ref_var_sources, &mut ref_positions);

    ref_positions
}

/// Collect variables assigned from indexed field access on borrowed parameters.
/// Maps variable name to the source parameter name.
pub fn collect_indexed_field_vars(
    stmts: &[HirStmt],
    borrowed_params: &HashSet<String>,
    ref_var_sources: &mut HashMap<String, String>,
) {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(var_name),
                value,
                ..
            } => {
                if let Some(param_name) =
                    get_indexed_field_source_param(value, borrowed_params)
                {
                    ref_var_sources.insert(var_name.clone(), param_name);
                }
            }
            HirStmt::For { body, .. } | HirStmt::While { body, .. } => {
                collect_indexed_field_vars(body, borrowed_params, ref_var_sources);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_indexed_field_vars(then_body, borrowed_params, ref_var_sources);
                if let Some(else_stmts) = else_body {
                    collect_indexed_field_vars(else_stmts, borrowed_params, ref_var_sources);
                }
            }
            _ => {}
        }
    }
}

/// Check if an expression is an Index into an attribute chain rooted at a borrowed param.
pub fn get_indexed_field_source_param(
    expr: &HirExpr,
    borrowed_params: &HashSet<String>,
) -> Option<String> {
    if let HirExpr::Index { base, .. } = expr {
        get_attribute_chain_root_param(base, borrowed_params)
    } else {
        None
    }
}

/// Walk an attribute/index chain to find a borrowed parameter root.
fn get_attribute_chain_root_param(
    expr: &HirExpr,
    borrowed_params: &HashSet<String>,
) -> Option<String> {
    match expr {
        HirExpr::Attribute { value, .. } => match value.as_ref() {
            HirExpr::Var(name) if borrowed_params.contains(name) => Some(name.clone()),
            _ => get_attribute_chain_root_param(value, borrowed_params),
        },
        HirExpr::Index { base, .. } => get_attribute_chain_root_param(base, borrowed_params),
        _ => None,
    }
}

/// Scan return statements for tuple elements that are reference variables.
fn find_ref_return_positions(
    stmts: &[HirStmt],
    ref_var_sources: &HashMap<String, String>,
    ref_positions: &mut Vec<bool>,
) {
    for stmt in stmts {
        if !ref_positions.is_empty() {
            return;
        }
        match stmt {
            HirStmt::Return(Some(HirExpr::Tuple(elems))) => {
                let pos: Vec<bool> = elems
                    .iter()
                    .map(|e| matches!(e, HirExpr::Var(name) if ref_var_sources.contains_key(name)))
                    .collect();
                if pos.iter().any(|b| *b) {
                    *ref_positions = pos;
                    return;
                }
            }
            HirStmt::For { body, .. } | HirStmt::While { body, .. } => {
                find_ref_return_positions(body, ref_var_sources, ref_positions);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                find_ref_return_positions(then_body, ref_var_sources, ref_positions);
                if let Some(else_stmts) = else_body {
                    find_ref_return_positions(else_stmts, ref_var_sources, ref_positions);
                }
            }
            _ => {}
        }
    }
}

/// Wrap the inner type of an Option or a plain type in a Reference.
fn wrap_rust_type_with_reference(
    rust_type: &crate::type_mapper::RustType,
    lifetime: &Option<String>,
) -> crate::type_mapper::RustType {
    use crate::type_mapper::RustType;
    match rust_type {
        RustType::Option(inner) => RustType::Option(Box::new(RustType::Reference {
            lifetime: lifetime.clone(),
            mutable: false,
            inner: inner.clone(),
        })),
        _ => RustType::Reference {
            lifetime: lifetime.clone(),
            mutable: false,
            inner: Box::new(rust_type.clone()),
        },
    }
}

/// Update lifetime_result when indexed field return references are detected.
fn apply_indexed_field_return_lifetimes(
    func: &HirFunction,
    lifetime_result: &mut crate::lifetime_analysis::LifetimeResult,
    ref_var_sources: &HashMap<String, String>,
) {
    let source_params: HashSet<String> = ref_var_sources.values().cloned().collect();

    for param in &source_params {
        if !lifetime_result.params_with_field_return.contains(param) {
            lifetime_result.params_with_field_return.push(param.clone());
        }
    }

    let ref_param_count = lifetime_result
        .param_lifetimes
        .iter()
        .filter(|(_, inf)| inf.should_borrow)
        .count();

    if ref_param_count > 1 {
        // Multiple borrowed params: need explicit lifetime
        let escaping_lifetime = "'a".to_string();
        for (name, inf) in lifetime_result.param_lifetimes.iter_mut() {
            if source_params.contains(name.as_str()) {
                inf.lifetime = Some(escaping_lifetime.clone());
            } else {
                // Non-source params don't need explicit lifetimes
                inf.lifetime = None;
            }
        }
        lifetime_result.return_lifetime = Some(escaping_lifetime.clone());
        if !lifetime_result.lifetime_params.contains(&escaping_lifetime) {
            lifetime_result.lifetime_params.push(escaping_lifetime);
        }
    } else if ref_param_count == 1 {
        // Single borrowed param: Rust lifetime elision handles it, but
        // we still need to know we have references for the return type
        let _ = func; // Intentional: single-param case uses elision
    }
}

// ========== Phase 3b: Return Type Generation ==========

/// Generate return type with Result wrapper and lifetime handling
///
/// DEPYLER-0310: Now returns ErrorType (4th tuple element) for raise statement wrapping
#[inline]
pub(crate) fn codegen_return_type(
    func: &HirFunction,
    lifetime_result: &crate::lifetime_analysis::LifetimeResult,
    ctx: &mut CodeGenContext,
) -> Result<(
    proc_macro2::TokenStream,
    crate::type_mapper::RustType,
    bool,
    Option<crate::rust_gen::context::ErrorType>,
    Type,
)> {
    // DEPYLER-0410: Infer return type from body when annotation is Unknown
    // DEPYLER-0420: Also infer when tuple/list contains Unknown elements
    let should_infer = matches!(func.ret_type, Type::Unknown)
        || matches!(&func.ret_type, Type::Tuple(elems) if elems.is_empty() || elems.iter().any(|t| matches!(t, Type::Unknown)))
        || matches!(&func.ret_type, Type::List(elem) if matches!(**elem, Type::Unknown));

    let effective_ret_type = if should_infer {
        // Try to infer from return statements in body
        if let Some(inferred) =
            infer_return_type_from_body(&func.body, &func.params, &ctx.class_field_types)
        {
            inferred
        } else {
            func.ret_type.clone()
        }
    } else {
        func.ret_type.clone()
    };

    // Convert return type using annotation-aware mapping
    let mapped_ret_type = ctx
        .annotation_aware_mapper
        .map_return_type_with_annotations(&effective_ret_type, &func.annotations);

    // Check if this is a placeholder Union enum that needs proper generation
    let rust_ret_type = if let crate::type_mapper::RustType::Enum { name, .. } = &mapped_ret_type {
        if name == "UnionType" {
            // Generate a proper enum name and definition from the original Union type
            if let Type::Union(types) = &func.ret_type {
                let enum_name = ctx.process_union_type(types);
                crate::type_mapper::RustType::Custom(enum_name)
            } else {
                mapped_ret_type
            }
        } else {
            mapped_ret_type
        }
    } else {
        mapped_ret_type
    };

    // v3.16.0 Phase 1: Override return type to String if function returns owned via string methods
    // This prevents lifetime analysis from incorrectly converting to borrowed &str
    let rust_ret_type =
        if matches!(func.ret_type, Type::String) && function_returns_owned_string(func) {
            // Force owned String return, don't use lifetime borrowing
            crate::type_mapper::RustType::String
        } else {
            rust_ret_type
        };

    // Update import needs based on return type
    update_import_needs(ctx, &rust_ret_type);

    // Wrap tuple elements in references when they come from indexed field access
    // on borrowed parameters (e.g., `player = state.players[idx]` → `&Player`)
    let rust_ret_type = if !ctx.return_reference_positions.is_empty() {
        if let crate::type_mapper::RustType::Tuple(ref elements) = rust_ret_type {
            if ctx.return_reference_positions.len() == elements.len() {
                let new_elements: Vec<crate::type_mapper::RustType> = elements
                    .iter()
                    .zip(ctx.return_reference_positions.iter())
                    .map(|(elem, is_ref)| {
                        if *is_ref {
                            wrap_rust_type_with_reference(elem, &lifetime_result.return_lifetime)
                        } else {
                            elem.clone()
                        }
                    })
                    .collect();
                crate::type_mapper::RustType::Tuple(new_elements)
            } else {
                rust_ret_type
            }
        } else {
            rust_ret_type
        }
    } else {
        rust_ret_type
    };

    // can_fail is always false — no Result wrapping
    let can_fail = false;
    let error_type: Option<crate::rust_gen::context::ErrorType> = None;

    let return_type = if matches!(rust_ret_type, crate::type_mapper::RustType::Unit) {
        quote! {}
    } else {
        let mut ty = rust_type_to_syn(&rust_ret_type)?;

        // DEPYLER-0270: Check if function returns string concatenation
        // String concatenation (format!(), a + b) always returns owned String
        // Never use Cow for concatenation results
        let returns_concatenation = matches!(func.ret_type, crate::hir::Type::String)
            && function_returns_string_concatenation(func);

        // Check if any parameter escapes through return and uses Cow
        let mut uses_cow_return = false;
        if !returns_concatenation {
            // Only consider Cow if NOT doing string concatenation
            for param in &func.params {
                if let Some(strategy) = lifetime_result.borrowing_strategies.get(&param.name) {
                    if matches!(
                        strategy,
                        crate::borrowing_context::BorrowingStrategy::UseCow { .. }
                    ) {
                        if let Some(_usage) = lifetime_result.param_lifetimes.get(&param.name) {
                            // If a Cow parameter escapes, return type should also be Cow
                            if matches!(func.ret_type, crate::hir::Type::String) {
                                uses_cow_return = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if uses_cow_return && !returns_concatenation {
            // Use the same Cow type for return
            ctx.needs_cow = true;
            if let Some(ref return_lt) = lifetime_result.return_lifetime {
                let lt = syn::Lifetime::new(return_lt.as_str(), proc_macro2::Span::call_site());
                ty = parse_quote! { Cow<#lt, str> };
            } else {
                ty = parse_quote! { Cow<'static, str> };
            }
        } else {
            // v3.16.0 Phase 1: Check if function returns owned String via transformation methods
            // If so, don't convert to borrowed &str even if lifetime analysis suggests it
            let returns_owned_string =
                matches!(func.ret_type, Type::String) && function_returns_owned_string(func);

            // Apply return lifetime if needed (unless returning owned String)
            if let Some(ref return_lt) = lifetime_result.return_lifetime {
                // Check if the return type needs lifetime substitution
                if matches!(
                    rust_ret_type,
                    crate::type_mapper::RustType::Str { .. }
                        | crate::type_mapper::RustType::Reference { .. }
                ) && !returns_owned_string
                {
                    // Only apply lifetime if NOT returning owned String
                    let lt = syn::Lifetime::new(return_lt.as_str(), proc_macro2::Span::call_site());
                    match &rust_ret_type {
                        crate::type_mapper::RustType::Str { .. } => {
                            ty = parse_quote! { &#lt str };
                        }
                        crate::type_mapper::RustType::Reference { mutable, inner, .. } => {
                            let inner_ty = rust_type_to_syn(inner)?;
                            ty = if *mutable {
                                parse_quote! { &#lt mut #inner_ty }
                            } else {
                                parse_quote! { &#lt #inner_ty }
                            };
                        }
                        _ => {}
                    }
                }
            }
            // If returns_owned_string is true, keep ty as String (already set from rust_type_to_syn)
        }

        quote! { -> #ty }
    };

    Ok((
        return_type,
        rust_ret_type,
        can_fail,
        error_type,
        effective_ret_type,
    ))
}

// ========== Phase 3c: Generator Implementation ==========
// (Moved to generator_gen.rs in v3.18.0 Phase 4)

impl RustCodeGen for HirFunction {
    fn to_rust_tokens(&self, ctx: &mut CodeGenContext) -> Result<proc_macro2::TokenStream> {
        // Set current function name for parameter ownership tracking
        ctx.current_function_name = Some(self.name.clone());

        // DEPYLER-0306 FIX: Use raw identifiers for function names that are Rust keywords
        let name = if is_rust_keyword(&self.name) {
            syn::Ident::new_raw(&self.name, proc_macro2::Span::call_site())
        } else {
            syn::Ident::new(&self.name, proc_macro2::Span::call_site())
        };

        // DEPYLER-0269: Track function return type for Display trait selection
        // Store function return type in ctx for later lookup when processing assignments
        // This enables tracking `result = merge(&a, &b)` where merge returns list[int]
        ctx.function_return_types
            .insert(self.name.clone(), self.ret_type.clone());

        // Perform generic type inference
        let mut generic_registry = crate::generic_inference::TypeVarRegistry::new();
        let type_params = generic_registry.infer_function_generics(self)?;

        // Perform lifetime analysis with automatic elision (DEPYLER-0275)
        let mut lifetime_inference = LifetimeInference::new();
        let mut lifetime_result = lifetime_inference
            .apply_elision_rules_with_interprocedural(
                self,
                ctx.type_mapper,
                ctx.interprocedural_analysis,
                &ctx.enum_names,
                &ctx.copy_structs,
            )
            .unwrap_or_else(|| {
                lifetime_inference.analyze_function_with_interprocedural(
                    self,
                    ctx.type_mapper,
                    ctx.interprocedural_analysis,
                    &ctx.enum_names,
                    &ctx.copy_structs,
                )
            });

        // INTERPROCEDURAL FIX: Apply mutability requirements from populate_function_param_borrows
        // The interprocedural analysis may have upgraded parameters to &mut based on callees
        // Skip Copy types (i32, f64, bool, etc.) — they should always be passed by value
        if let Some(param_borrows) = ctx.function_param_borrows.get(&self.name) {
            for (param_idx, borrow_info) in param_borrows.iter().enumerate() {
                if param_idx < self.params.len() {
                    // Skip Copy types — primitives and enums are cheap to pass by value
                    let param_rust_type = ctx.type_mapper.map_type(&self.params[param_idx].ty);
                    if super::is_copy_rust_type(
                        &param_rust_type,
                        &ctx.enum_names,
                        &ctx.copy_structs,
                    ) {
                        continue;
                    }

                    let param_name = &self.params[param_idx].name;
                    if let Some(param_lifetime) =
                        lifetime_result.param_lifetimes.get_mut(param_name)
                    {
                        // Upgrade to borrow if interprocedural analysis says so
                        if borrow_info.should_borrow {
                            param_lifetime.should_borrow = true;
                        }
                        // Upgrade to &mut if interprocedural analysis says so
                        if borrow_info.needs_mut {
                            param_lifetime.needs_mut = true;
                        }
                        // Override take_ownership if interprocedural analysis says to borrow
                        if !borrow_info.takes_ownership && borrow_info.should_borrow {
                            // Update borrowing strategy to match
                            lifetime_result.borrowing_strategies.insert(
                                param_name.clone(),
                                if borrow_info.needs_mut {
                                    BorrowingStrategy::BorrowMutable {
                                        lifetime: param_lifetime.lifetime.clone(),
                                    }
                                } else {
                                    BorrowingStrategy::BorrowImmutable {
                                        lifetime: param_lifetime.lifetime.clone(),
                                    }
                                },
                            );
                        }
                    }
                }
            }
        }

        // Detect indexed field returns (e.g., `player = state.players[idx]` then `return player, idx`)
        // Run field borrowing analysis early to determine which vars will be borrowed
        ctx.analyze_field_borrowing(&self.body);
        let early_borrowable = ctx.borrowable_vars.clone();
        ctx.borrowable_vars.clear();
        ctx.mut_borrowable_vars.clear();

        // Skip reference-return optimization if callers pass &mut for the source parameter,
        // which would create cross-statement borrow conflicts at call sites.
        let return_ref_positions =
            if ctx.functions_suppress_ref_return.contains(&self.name) {
                vec![]
            } else {
                detect_indexed_field_return_refs(self, &lifetime_result, &early_borrowable)
            };
        if !return_ref_positions.is_empty() {
            // Collect the source params from indexed field vars (re-derive for lifetime update)
            let borrowed_params: HashSet<String> = lifetime_result
                .param_lifetimes
                .iter()
                .filter(|(_, inf)| inf.should_borrow)
                .map(|(name, _)| name.clone())
                .collect();
            let mut ref_var_sources: HashMap<String, String> = HashMap::new();
            collect_indexed_field_vars(&self.body, &borrowed_params, &mut ref_var_sources);
            ref_var_sources.retain(|var_name, _| early_borrowable.contains(var_name));
            apply_indexed_field_return_lifetimes(self, &mut lifetime_result, &ref_var_sources);
        }
        ctx.return_reference_positions = return_ref_positions;

        // Generate combined generic parameters (lifetimes + type params)
        let generic_params = codegen_generic_params(&type_params, &lifetime_result.lifetime_params);

        // Generate lifetime bounds
        let where_clause = codegen_where_clause(&lifetime_result.lifetime_bounds);

        // DEPYLER-0312: Analyze mutability BEFORE generating parameters
        // Clear per-function mutable_vars to prevent leaking state from previous functions
        ctx.mutable_vars.clear();
        ctx.mut_ref_index_vars.clear();
        // This populates ctx.mutable_vars which codegen_single_param uses to determine `mut` keyword
        analyze_mutable_vars(&self.body, ctx, &self.params);

        // Clear per-function ref param tracking
        ctx.current_func_mut_ref_params.clear();
        ctx.current_func_ref_params.clear();

        // Populate current function's parameter ownership map for zip/enumerate iterator decisions
        // Maps parameter name -> whether it takes ownership (true) or borrows (false)
        ctx.current_function_param_ownership.clear();
        for param in &self.params {
            let takes_ownership = lifetime_result
                .borrowing_strategies
                .get(&param.name)
                .map(|strategy| matches!(strategy, BorrowingStrategy::TakeOwnership))
                .unwrap_or(false);
            ctx.current_function_param_ownership
                .insert(param.name.clone(), takes_ownership);
        }

        // TODO: Store clone requirements for parameters so expression generation knows when to clone
        // ctx.param_clone_requirements = lifetime_result.param_clone_requirements.clone();

        // Convert parameters using lifetime analysis results
        let params = codegen_function_params(self, &lifetime_result, ctx)?;

        // Analyze field-source variable borrowing AFTER params are generated
        // (needs current_func_mut_ref_params populated by codegen_function_params)
        ctx.analyze_field_borrowing(&self.body);

        // When reference returns are suppressed, remove indexed-field variables from
        // borrowable_vars so assignments generate .get().cloned() instead of &ref.
        if ctx.functions_suppress_ref_return.contains(&self.name) {
            let borrowed_params: HashSet<String> = lifetime_result
                .param_lifetimes
                .iter()
                .filter(|(_, inf)| inf.should_borrow)
                .map(|(name, _)| name.clone())
                .collect();
            let mut ref_var_sources: HashMap<String, String> = HashMap::new();
            collect_indexed_field_vars(&self.body, &borrowed_params, &mut ref_var_sources);
            for var_name in ref_var_sources.keys() {
                ctx.borrowable_vars.remove(var_name);
                ctx.mut_borrowable_vars.remove(var_name);
            }
        }

        // Detect variables consumed in multiple move positions (e.g., assigned to a struct
        // field AND passed to push/append) so the earlier use gets .clone().
        ctx.analyze_move_consuming_uses(&self.body);

        // Variables in mut_borrowable_vars will hold &mut references, so the binding
        // itself doesn't need `mut`. Remove them from mutable_vars to avoid `let mut`.
        for var_name in &ctx.mut_borrowable_vars {
            ctx.mutable_vars.remove(var_name);
        }

        // Variables in mut_ref_index_vars hold &mut references from subscript access,
        // so the binding itself doesn't need `mut`.
        for var_name in &ctx.mut_ref_index_vars.clone() {
            ctx.mutable_vars.remove(var_name);
        }

        // NOTE: function_param_borrows is now pre-populated in rust_gen.rs::populate_function_param_borrows()
        // before any functions are generated. This ensures call sites have complete information
        // about callee parameter signatures. The code below is kept for reference but not executed.
        // let param_borrows: Vec<(bool, bool)> = self
        //     .params
        //     .iter()
        //     .map(|p| {
        //         lifetime_result
        //             .param_lifetimes
        //             .get(&p.name)
        //             .map(|inf| (inf.should_borrow, inf.needs_mut))
        //             .unwrap_or((false, false))
        //     })
        //     .collect();
        // ctx.function_param_borrows
        //     .insert(self.name.clone(), param_borrows);

        // Generate return type with Result wrapper and lifetime handling
        let (return_type, rust_ret_type, can_fail, error_type, effective_ret_type) =
            codegen_return_type(self, &lifetime_result, ctx)?;

        // Store the effective (inferred) return type so codegen_return_stmt can
        // perform element-level Optional wrapping for tuples.
        ctx.effective_return_type = Some(effective_ret_type);

        // DEPYLER-0425: Analyze subcommand field access BEFORE generating body
        // This sets ctx.current_subcommand_fields so expression generation can rewrite args.field → field
        let subcommand_info = if ctx.argparser_tracker.has_subcommands() {
            crate::rust_gen::argparse_transform::analyze_subcommand_field_access(
                self,
                &ctx.argparser_tracker,
            )
        } else {
            None
        };

        // Set context for expression generation
        if let Some((_, ref fields)) = subcommand_info {
            ctx.current_subcommand_fields = Some(fields.iter().cloned().collect());
        }

        // Process function body with proper scoping (expressions will now be rewritten if needed)
        let mut body_stmts = codegen_function_body(self, can_fail, error_type, ctx)?;

        // Clear the subcommand fields context after body generation
        ctx.current_subcommand_fields = None;

        // DEPYLER-0363: Check if ArgumentParser was detected and generate Args struct
        // DEPYLER-0424: Store Args struct and Commands enum in context for module-level emission
        // (hoisted outside function to make Args accessible to handler functions)
        if ctx.argparser_tracker.has_parsers() {
            if let Some(parser_info) = ctx.argparser_tracker.get_first_parser() {
                // DEPYLER-0384: Set flag to include clap dependency in Cargo.toml
                ctx.needs_clap = true;

                // DEPYLER-0399: Generate Commands enum if subcommands exist
                let commands_enum = crate::rust_gen::argparse_transform::generate_commands_enum(
                    &ctx.argparser_tracker,
                );
                if !commands_enum.is_empty() {
                    ctx.generated_commands_enum = Some(commands_enum);
                }

                // Generate the Args struct definition
                let args_struct = crate::rust_gen::argparse_transform::generate_args_struct(
                    parser_info,
                    &ctx.argparser_tracker,
                );
                ctx.generated_args_struct = Some(args_struct);

                // Note: ArgumentParser-related statements are filtered in stmt_gen.rs
                // parse_args() calls are transformed in stmt_gen.rs::codegen_assign_stmt
            }

            // DO NOT clear tracker yet - we need it for parameter type resolution
            // It will be cleared after all functions are generated
        }

        // DEPYLER-0425: Wrap handler functions with subcommand pattern matching
        // If this function accesses subcommand-specific fields, wrap body in pattern matching
        if let Some((variant_name, fields)) = subcommand_info {
            // Get args parameter name (first parameter)
            if let Some(args_param) = self.params.first() {
                let args_param_name = args_param.name.as_ref();
                // Wrap body statements in pattern matching to extract fields from enum variant
                body_stmts = crate::rust_gen::argparse_transform::wrap_body_with_subcommand_pattern(
                    body_stmts,
                    &variant_name,
                    &fields,
                    args_param_name,
                );
            }
        }

        // Add documentation and custom attributes
        let attrs = codegen_function_attrs(
            &self.docstring,
            &self.properties,
            &self.annotations.custom_attributes,
        );

        // Check if function is a generator (contains yield)
        let func_tokens = if self.properties.is_generator {
            codegen_generator_function(
                self,
                &name,
                &generic_params,
                &where_clause,
                &params,
                &attrs,
                &rust_ret_type,
                ctx,
            )?
        } else if self.properties.is_async {
            quote! {
                #(#attrs)*
                pub async fn #name #generic_params(#(#params),*) #return_type #where_clause {
                    #(#body_stmts)*
                }
            }
        } else {
            quote! {
                #(#attrs)*
                pub fn #name #generic_params(#(#params),*) #return_type #where_clause {
                    #(#body_stmts)*
                }
            }
        };

        // Reset clone requirements to avoid leaking into next function
        ctx.param_clone_requirements.clear();

        Ok(func_tokens)
    }
}
