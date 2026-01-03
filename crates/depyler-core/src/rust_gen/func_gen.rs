//! Function code generation
//!
//! This module handles converting HIR functions to Rust token streams.
//! It includes all function conversion helpers and the HirFunction RustCodeGen trait implementation.

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

// ============================================================================
// Borrow Conflict Analysis
// ============================================================================

/// Analyze function body for borrow conflicts and populate vars_needing_clone_at_assign.
///
/// Detects the pattern:
///   var1 = func1(&source)  # returns &T borrowing from source
///   var2 = func2(&mut source)  # needs mutable borrow of source  
///   use(var1)  # var1 still alive -> CONFLICT
///
/// For such vars, we need to clone at assignment to release the borrow early.
pub(crate) fn analyze_borrow_conflicts(func: &HirFunction, ctx: &mut CodeGenContext) {
    // Recursively analyze all statement blocks
    analyze_borrow_conflicts_in_block(&func.body, func, ctx);
}

/// Analyze borrow conflicts in a statement block
fn analyze_borrow_conflicts_in_block(
    body: &[HirStmt],
    func: &HirFunction,
    ctx: &mut CodeGenContext,
) {
    // Track: var_name -> (source_param, statement_index)
    // Variables that hold references borrowing from a parameter
    let mut vars_borrowing_from: HashMap<String, (String, usize)> = HashMap::new();

    // First pass: identify assignments that borrow from function parameters
    for (idx, stmt) in body.iter().enumerate() {
        // Recurse into nested blocks first
        match stmt {
            HirStmt::For { body: for_body, .. } => {
                analyze_borrow_conflicts_in_block(for_body, func, ctx);
            }
            HirStmt::While {
                body: while_body, ..
            } => {
                analyze_borrow_conflicts_in_block(while_body, func, ctx);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                analyze_borrow_conflicts_in_block(then_body, func, ctx);
                if let Some(else_b) = else_body {
                    analyze_borrow_conflicts_in_block(else_b, func, ctx);
                }
            }
            _ => {}
        }

        if let HirStmt::Assign {
            target: AssignTarget::Symbol(var_name),
            value,
            ..
        } = stmt
        {
            if let HirExpr::Call {
                func: func_name,
                args,
                ..
            } = value
            {
                // Check if this function returns a reference (check functions_returning_refs)
                if ctx.functions_returning_refs.contains(func_name) {
                    // Find which argument is the source of the borrow
                    // Usually the first argument for getter-style functions
                    if let Some(HirExpr::Var(source_param)) = args.first() {
                        // Check if source_param is a function parameter
                        if func.params.iter().any(|p| &p.name == source_param) {
                            vars_borrowing_from
                                .insert(var_name.clone(), (source_param.clone(), idx));
                        }
                    }
                }
            }
        }
    }

    // Second pass: find mutable borrows and check for conflicts
    for (idx, stmt) in body.iter().enumerate() {
        // Check for calls that take &mut of a parameter
        let mut_borrow_source = find_mut_borrow_source(stmt, func, ctx);

        if let Some(source_param) = mut_borrow_source {
            // Check if any variable borrowing from this source is used after this point
            for (var_name, (borrow_source, assign_idx)) in &vars_borrowing_from {
                if borrow_source == &source_param && *assign_idx < idx {
                    // Check if var_name is used after this statement
                    if is_var_used_after(body, idx, var_name) {
                        ctx.vars_needing_clone_at_assign.insert(var_name.clone());
                    }
                }
            }
        }
    }
}

/// Find if a statement causes a mutable borrow of a function parameter
fn find_mut_borrow_source(
    stmt: &HirStmt,
    func: &HirFunction,
    ctx: &CodeGenContext,
) -> Option<String> {
    match stmt {
        HirStmt::Assign { value, .. } => find_mut_borrow_in_expr(value, func, ctx),
        HirStmt::Expr(expr) => find_mut_borrow_in_expr(expr, func, ctx),
        _ => None,
    }
}

/// Find if an expression causes a mutable borrow of a function parameter
fn find_mut_borrow_in_expr(
    expr: &HirExpr,
    func: &HirFunction,
    ctx: &CodeGenContext,
) -> Option<String> {
    match expr {
        HirExpr::Call {
            func: func_name,
            args,
            ..
        } => {
            // Check if this function takes &mut for any parameter
            if let Some(muts) = ctx.function_param_muts.get(func_name) {
                for (i, needs_mut) in muts.iter().enumerate() {
                    if *needs_mut {
                        if let Some(HirExpr::Var(arg_name)) = args.get(i) {
                            // Check if this is a function parameter
                            if func.params.iter().any(|p| &p.name == arg_name) {
                                return Some(arg_name.clone());
                            }
                        }
                    }
                }
            }
            // Also check if the function itself is known to need &mut return
            // (which means its source param becomes &mut)
            if ctx.functions_with_mutated_return.contains(func_name) {
                if let Some(HirExpr::Var(arg_name)) = args.first() {
                    if func.params.iter().any(|p| &p.name == arg_name) {
                        return Some(arg_name.clone());
                    }
                }
            }
            None
        }
        HirExpr::MethodCall { object, args, .. } => {
            // Check method call arguments
            for arg in args {
                if let Some(source) = find_mut_borrow_in_expr(arg, func, ctx) {
                    return Some(source);
                }
            }
            find_mut_borrow_in_expr(object, func, ctx)
        }
        _ => None,
    }
}

/// Check if a variable is used after a given statement index
fn is_var_used_after(body: &[HirStmt], after_idx: usize, var_name: &str) -> bool {
    for stmt in body.iter().skip(after_idx + 1) {
        if is_var_used_in_stmt(var_name, stmt) {
            return true;
        }
    }
    false
}

/// Check if a variable is used in a statement
fn is_var_used_in_stmt(var_name: &str, stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { value, .. } => is_var_used_in_expr(var_name, value),
        HirStmt::Return(Some(expr)) => is_var_used_in_expr(var_name, expr),
        HirStmt::Return(None) => false,
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
        HirStmt::Expr(expr) => is_var_used_in_expr(var_name, expr),
        HirStmt::With { context, body, .. } => {
            is_var_used_in_expr(var_name, context)
                || body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::Try {
            body,
            handlers,
            finalbody,
            ..
        } => {
            body.iter().any(|s| is_var_used_in_stmt(var_name, s))
                || handlers
                    .iter()
                    .any(|h| h.body.iter().any(|s| is_var_used_in_stmt(var_name, s)))
                || finalbody
                    .as_ref()
                    .is_some_and(|body| body.iter().any(|s| is_var_used_in_stmt(var_name, s)))
        }
        HirStmt::Raise { exception, cause } => {
            exception
                .as_ref()
                .is_some_and(|e| is_var_used_in_expr(var_name, e))
                || cause
                    .as_ref()
                    .is_some_and(|c| is_var_used_in_expr(var_name, c))
        }
        HirStmt::Assert { test, msg } => {
            is_var_used_in_expr(var_name, test)
                || msg
                    .as_ref()
                    .is_some_and(|m| is_var_used_in_expr(var_name, m))
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } | HirStmt::Pass => false,
        HirStmt::FunctionDef { body, .. } => body.iter().any(|s| is_var_used_in_stmt(var_name, s)),
        HirStmt::Global { names } | HirStmt::Nonlocal { names } => {
            names.iter().any(|n| n == var_name)
        }
        HirStmt::Import { .. } | HirStmt::ImportFrom { .. } => false,
        HirStmt::AsyncFor { iter, body, .. } => {
            is_var_used_in_expr(var_name, iter)
                || body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::AsyncWith { context, body, .. } => {
            is_var_used_in_expr(var_name, context)
                || body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::Delete { targets } => targets.iter().any(|t| match t {
            AssignTarget::Symbol(name) => name == var_name,
            _ => false,
        }),
        HirStmt::AsyncFunctionDef { body, .. } => {
            body.iter().any(|s| is_var_used_in_stmt(var_name, s))
        }
        HirStmt::Match { subject, cases } => {
            is_var_used_in_expr(var_name, subject)
                || cases.iter().any(|case| {
                    case.guard
                        .as_ref()
                        .is_some_and(|g| is_var_used_in_expr(var_name, g))
                        || case.body.iter().any(|s| is_var_used_in_stmt(var_name, s))
                })
        }
    }
}

/// Check if a variable is used in an expression
fn is_var_used_in_expr(var_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(name) => name == var_name,
        HirExpr::Binary { left, right, .. } => {
            is_var_used_in_expr(var_name, left) || is_var_used_in_expr(var_name, right)
        }
        HirExpr::Unary { operand, .. } => is_var_used_in_expr(var_name, operand),
        HirExpr::Call { args, kwargs, .. } => {
            args.iter().any(|a| is_var_used_in_expr(var_name, a))
                || kwargs.iter().any(|(_, v)| is_var_used_in_expr(var_name, v))
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            is_var_used_in_expr(var_name, object)
                || args.iter().any(|a| is_var_used_in_expr(var_name, a))
                || kwargs.iter().any(|(_, v)| is_var_used_in_expr(var_name, v))
        }
        HirExpr::Attribute { value, .. } => is_var_used_in_expr(var_name, value),
        HirExpr::Index { base, index } => {
            is_var_used_in_expr(var_name, base) || is_var_used_in_expr(var_name, index)
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            is_var_used_in_expr(var_name, base)
                || start
                    .as_ref()
                    .is_some_and(|e| is_var_used_in_expr(var_name, e))
                || stop
                    .as_ref()
                    .is_some_and(|e| is_var_used_in_expr(var_name, e))
                || step
                    .as_ref()
                    .is_some_and(|e| is_var_used_in_expr(var_name, e))
        }
        HirExpr::List(elts)
        | HirExpr::Tuple(elts)
        | HirExpr::Set(elts)
        | HirExpr::FrozenSet(elts) => elts.iter().any(|e| is_var_used_in_expr(var_name, e)),
        HirExpr::Dict(pairs) => pairs
            .iter()
            .any(|(k, v)| is_var_used_in_expr(var_name, k) || is_var_used_in_expr(var_name, v)),
        HirExpr::IfExpr { test, body, orelse } => {
            is_var_used_in_expr(var_name, test)
                || is_var_used_in_expr(var_name, body)
                || is_var_used_in_expr(var_name, orelse)
        }
        HirExpr::Lambda { body, .. } => is_var_used_in_expr(var_name, body),
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        }
        | HirExpr::SetComp {
            element,
            iter,
            condition,
            ..
        } => {
            is_var_used_in_expr(var_name, element)
                || is_var_used_in_expr(var_name, iter)
                || condition
                    .as_ref()
                    .is_some_and(|c| is_var_used_in_expr(var_name, c))
        }
        HirExpr::FlattenedListComp {
            element,
            generators,
        } => {
            is_var_used_in_expr(var_name, element)
                || generators.iter().any(|g| {
                    is_var_used_in_expr(var_name, &g.iter)
                        || g.conditions
                            .iter()
                            .any(|c| is_var_used_in_expr(var_name, c))
                })
        }
        HirExpr::DictComp {
            key,
            value,
            iter,
            condition,
            ..
        } => {
            is_var_used_in_expr(var_name, key)
                || is_var_used_in_expr(var_name, value)
                || is_var_used_in_expr(var_name, iter)
                || condition
                    .as_ref()
                    .is_some_and(|c| is_var_used_in_expr(var_name, c))
        }
        HirExpr::GeneratorExp {
            element,
            generators,
        } => {
            is_var_used_in_expr(var_name, element)
                || generators.iter().any(|g| {
                    is_var_used_in_expr(var_name, &g.iter)
                        || g.conditions
                            .iter()
                            .any(|c| is_var_used_in_expr(var_name, c))
                })
        }
        HirExpr::Await { value } => is_var_used_in_expr(var_name, value),
        HirExpr::Yield { value } => value
            .as_ref()
            .is_some_and(|e| is_var_used_in_expr(var_name, e)),
        HirExpr::Borrow { expr, .. } => is_var_used_in_expr(var_name, expr),
        HirExpr::SortByKey {
            iterable, key_body, ..
        } => is_var_used_in_expr(var_name, iterable) || is_var_used_in_expr(var_name, key_body),
        HirExpr::NamedExpr { target, value } => {
            target == var_name || is_var_used_in_expr(var_name, value)
        }
        HirExpr::Literal(_) | HirExpr::FString { .. } | HirExpr::Uninitialized => false,
    }
}

// ============================================================================
// End Borrow Conflict Analysis
// ============================================================================

/// Check if a HIR Type is a Copy type (primitives that can be used in array repeat syntax [x; n])
fn is_copy_type(ty: &Type) -> bool {
    match ty {
        // Primitive types are Copy
        Type::Int | Type::Float | Type::Bool | Type::None => true,
        // Unknown might be primitive, be conservative and assume not Copy
        Type::Unknown => false,
        // Compound types are not Copy
        Type::String | Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::Custom(_) => false,
        // Tuples are Copy only if all elements are Copy
        Type::Tuple(types) => types.iter().all(is_copy_type),
        // Arrays are Copy only if element is Copy
        Type::Array { element_type, .. } => is_copy_type(element_type),
        // Optional/Final are Copy only if inner is Copy
        Type::Optional(inner) | Type::Final(inner) => is_copy_type(inner),
        // Union types are not Copy in general
        Type::Union(_) => false,
        // Functions, type vars, generics are not Copy
        Type::Function { .. } | Type::TypeVar(_) | Type::Generic { .. } => false,
    }
}

/// Check if a name is a Rust keyword that requires raw identifier syntax
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
    _properties: &crate::hir::FunctionProperties,
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
    ctx.current_error_type = error_type;

    // Clear CSE temporary variable types from previous functions to avoid type conflicts
    // (e.g., _cse_temp_0 might be Int in one function, Bool in another)
    // Preserve module-level constant types and other non-temp variables
    ctx.var_types
        .retain(|name, _| !name.starts_with("_cse_temp"));

    for param in &func.params {
        ctx.declare_var(&param.name);
        // Store parameter type information for set/dict disambiguation
        ctx.var_types.insert(param.name.clone(), param.ty.clone());
    }

    // codegen_function_params, so ctx.mutable_vars is already populated here

    // Analyze borrow conflicts before generating statements
    // This detects pattern: var1 = f(&state), var2 = g(&mut state), use(var1)
    // and marks var1 for cloning at assignment to avoid borrow conflict
    analyze_borrow_conflicts(func, ctx);

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
    // Parameter names in signature must match exactly how they're referenced in function body
    // Rename 'self' to 'self_param' since 'self' is a Rust keyword
    let param_name = if param.name == "self" {
        "self_param".to_string()
    } else {
        param.name.clone()
    };
    let param_ident = syn::Ident::new(&param_name, proc_macro2::Span::call_site());

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

    // This handles cases where the param is passed down a call chain from a borrowing caller
    let param_idx = func.params.iter().position(|p| p.name == param.name);
    let interprocedural_needs_mut = param_idx
        .and_then(|idx| {
            ctx.function_param_muts
                .get(&func.name)
                .and_then(|muts| muts.get(idx))
                .copied()
        })
        .unwrap_or(false);

    // This handles ALL mutation patterns: direct assignment, method calls, and parameter reassignments
    // The analyze_mutable_vars function already checked all mutation patterns in codegen_function_body
    let is_mutated_in_body = ctx.mutable_vars.contains(&param.name);

    // Copy types (int, float, bool) should never be borrowed - they're passed by value
    let param_is_copy = is_copy_type(&param.ty);

    // this means the param is passed from a caller that borrows - must also borrow
    // But skip for Copy types which are always passed by value
    let force_borrow_from_call_chain =
        interprocedural_needs_mut && !is_mutated_in_body && !param_is_copy;

    // Only apply `mut` if ownership is taken (not borrowed)
    // Borrowed parameters (&T, &mut T) handle mutability in the type itself
    let takes_ownership = !force_borrow_from_call_chain
        && matches!(
            lifetime_result.borrowing_strategies.get(&param.name),
            Some(crate::borrowing_context::BorrowingStrategy::TakeOwnership) | None
        );

    let is_param_mutated = is_mutated_in_body && takes_ownership;

    // Argparse validators receive String (clap will convert)
    let is_argparse_validator = ctx.validator_functions.contains(&func.name);

    if is_argparse_validator {
        // Argparse validators receive String arguments
        let ty = if is_param_mutated {
            quote! { mut #param_ident: String }
        } else {
            quote! { #param_ident: String }
        };
        return Ok(ty);
    }

    // Get the inferred parameter info
    #[cfg(debug_assertions)]
    log::debug!(
        "codegen_single_param: func={}, param={}, interprocedural_needs_mut={}, is_mutated_in_body={}, force_borrow_from_call_chain={}",
        func.name,
        param.name,
        interprocedural_needs_mut,
        is_mutated_in_body,
        force_borrow_from_call_chain
    );

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

        // If analyze_mutable_vars detected mutation (via .remove(), .clear(), etc.)
        // and this parameter will be borrowed (&T), upgrade to &mut T
        let mut inferred_with_mut = inferred.clone();

        // Check if this param's field is returned and the return value is mutated at call sites
        let return_value_is_mutated = ctx.functions_with_mutated_return.contains(&func.name);
        let param_field_escapes = lifetime_result
            .params_with_field_return
            .contains(&param.name);
        let needs_mut_for_return = return_value_is_mutated && param_field_escapes;

        if force_borrow_from_call_chain {
            // Force borrowing with &mut for call chain requirements
            inferred_with_mut.should_borrow = true;
            inferred_with_mut.needs_mut = true;
            // Track this parameter as already being &mut so we don't add &mut again at call sites
            ctx.current_func_mut_ref_params.insert(param.name.clone());
        } else if needs_mut_for_return && inferred.should_borrow {
            // Return value is mutated at call sites and this param's field escapes through return
            // Must use &mut so the returned reference can be mutated
            inferred_with_mut.needs_mut = true;
            ctx.current_func_mut_ref_params.insert(param.name.clone());
        } else if is_mutated_in_body && inferred.should_borrow {
            inferred_with_mut.needs_mut = true;
            // Track this parameter as already being &mut
            ctx.current_func_mut_ref_params.insert(param.name.clone());
        } else if inferred.should_borrow && !inferred.needs_mut {
            // Track this parameter as an immutable reference
            ctx.current_func_ref_params.insert(param.name.clone());
        }

        let ty = apply_param_borrowing_strategy(
            &param.name,
            &actual_rust_type,
            &inferred_with_mut,
            lifetime_result,
            ctx,
        )?;

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
        // Always use String for string parameters (not &str) to match Python semantics
        if force_borrow_from_call_chain {
            // Track this parameter as already being &mut so we don't add &mut again at call sites
            ctx.current_func_mut_ref_params.insert(param.name.clone());
            Ok(quote! { #param_ident: &mut #ty })
        } else if is_param_mutated {
            Ok(quote! { mut #param_ident: #ty })
        } else {
            Ok(quote! { #param_ident: #ty })
        }
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
    let ty = rust_type_to_syn(rust_type)?;

    // String parameters should always be owned String, never borrowed
    if matches!(rust_type, crate::type_mapper::RustType::String) {
        return Ok(ty);
    }

    let mut ty = ty;

    // If lifetime_params is empty, Rust's elision rules apply - don't add explicit lifetimes
    let should_elide_lifetimes = lifetime_result.lifetime_params.is_empty();

    // Check if we have a borrowing strategy
    if let Some(strategy) = lifetime_result.borrowing_strategies.get(param_name) {
        match strategy {
            crate::borrowing_context::BorrowingStrategy::UseCow { lifetime } => {
                ctx.needs_cow = true;

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
                    ty = apply_borrowing_to_type(ty, inferred, should_elide_lifetimes)?;
                }
            }
        }
    } else {
        // Fallback to normal borrowing
        if inferred.should_borrow {
            ty = apply_borrowing_to_type(ty, inferred, should_elide_lifetimes)?;
        }
    }

    Ok(ty)
}

/// Apply borrowing (&, &mut, with lifetime) to a type
fn apply_borrowing_to_type(
    mut ty: syn::Type,
    inferred: &crate::lifetime_analysis::InferredParam,
    should_elide_lifetimes: bool,
) -> Result<syn::Type> {
    // Use &String for borrowed strings (not &str) to match Python semantics
    // Non-string types also get normal borrowing
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
        | HirExpr::FlattenedListComp { .. }
        | HirExpr::SetComp { .. }
        | HirExpr::DictComp { .. }
        | HirExpr::Lambda { .. }
        | HirExpr::Await { .. }
        | HirExpr::FString { .. }
        | HirExpr::Yield { .. }
        | HirExpr::SortByKey { .. }
        | HirExpr::GeneratorExp { .. }
        | HirExpr::NamedExpr { .. } => false,
        HirExpr::Uninitialized => false,
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

// ========== Return Type Inference from Body ==========

/// Infer return type from function body when no annotation is provided
/// Returns None if type cannot be inferred or there are no return statements
fn infer_return_type_from_body(body: &[HirStmt]) -> Option<Type> {
    let mut var_types: std::collections::HashMap<String, Type> = std::collections::HashMap::new();
    build_var_type_env(body, &mut var_types);

    let mut return_types = Vec::new();
    collect_return_types_with_env(body, &mut return_types, &var_types);

    // If the last statement is an expression without return, it's an implicit return
    if let Some(HirStmt::Expr(expr)) = body.last() {
        let trailing_type = infer_expr_type_with_env(expr, &var_types);
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

    // to be incorrectly typed as i32. Instead, return None and let the type mapper
    // handle the fallback (which will use serde_json::Value for complex types).

    // Previous behavior : Defaulted Unknown → Int for lambda returns
    // Problem: This also affected dict/list returns, causing E0308 errors
    // New behavior: Return None for Unknown types, allowing proper Value fallback
    if return_types.iter().all(|t| matches!(t, Type::Unknown)) {
        // We have return statements but all returned Unknown types
        // Don't assume Int - let type mapper decide the appropriate fallback
        return None;
    }

    // Mixed types - return the first known type
    first_known.cloned()
}

// ========== Variable Type Environment ==========

/// Build a type environment by collecting variable assignments
fn build_var_type_env(stmts: &[HirStmt], var_types: &mut std::collections::HashMap<String, Type>) {
    for stmt in stmts {
        match stmt {
            HirStmt::Assign {
                target: crate::hir::AssignTarget::Symbol(name),
                value,
                ..
            } => {
                let value_type = infer_expr_type_with_env(value, var_types);
                if !matches!(value_type, Type::Unknown) {
                    var_types.insert(name.clone(), value_type);
                }
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                build_var_type_env(then_body, var_types);
                if let Some(else_stmts) = else_body {
                    build_var_type_env(else_stmts, var_types);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                build_var_type_env(body, var_types);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                build_var_type_env(body, var_types);
                for handler in handlers {
                    build_var_type_env(&handler.body, var_types);
                }
                if let Some(orelse_stmts) = orelse {
                    build_var_type_env(orelse_stmts, var_types);
                }
                if let Some(finally_stmts) = finalbody {
                    build_var_type_env(finally_stmts, var_types);
                }
            }
            HirStmt::With { body, .. } => {
                build_var_type_env(body, var_types);
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
) {
    for stmt in stmts {
        match stmt {
            HirStmt::Return(Some(expr)) => {
                types.push(infer_expr_type_with_env(expr, var_types));
            }
            HirStmt::Return(None) => {
                types.push(Type::None);
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_return_types_with_env(then_body, types, var_types);
                if let Some(else_stmts) = else_body {
                    collect_return_types_with_env(else_stmts, types, var_types);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                collect_return_types_with_env(body, types, var_types);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                collect_return_types_with_env(body, types, var_types);
                for handler in handlers {
                    collect_return_types_with_env(&handler.body, types, var_types);
                }
                if let Some(orelse_stmts) = orelse {
                    collect_return_types_with_env(orelse_stmts, types, var_types);
                }
                if let Some(finally_stmts) = finalbody {
                    collect_return_types_with_env(finally_stmts, types, var_types);
                }
            }
            HirStmt::With { body, .. } => {
                collect_return_types_with_env(body, types, var_types);
            }
            _ => {}
        }
    }
}

/// Infer expression type with access to variable type environment
pub(crate) fn infer_expr_type_with_env(
    expr: &HirExpr,
    var_types: &std::collections::HashMap<String, Type>,
) -> Type {
    match expr {
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
                    | BinOp::Is
                    | BinOp::IsNot
            ) {
                return Type::Bool;
            }

            if matches!(op, BinOp::Mul) {
                match (left.as_ref(), right.as_ref()) {
                    // Pattern: [elem] * n
                    (HirExpr::List(elems), &HirExpr::Literal(Literal::Int(size)))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_with_env(&elems[0], var_types);
                        // Non-Copy types always produce Vec (can't use array repeat syntax)
                        return if size <= 32 && is_copy_type(&elem_type) {
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
                        // Non-Copy types always produce Vec (can't use array repeat syntax)
                        return if size <= 32 && is_copy_type(&elem_type) {
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
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems
                .iter()
                .map(|e| infer_expr_type_with_env(e, var_types))
                .collect();
            Type::Tuple(elem_types)
        }
        HirExpr::List(elems) => {
            if elems.is_empty() {
                Type::List(Box::new(Type::Unknown))
            } else {
                // Try to find a non-Unknown element type by scanning all elements
                let elem_type = elems
                    .iter()
                    .map(|e| infer_expr_type_with_env(e, var_types))
                    .find(|t| {
                        !matches!(t, Type::Unknown)
                            && !matches!(t, Type::List(inner) if matches!(inner.as_ref(), Type::Unknown))
                    })
                    .unwrap_or_else(|| infer_expr_type_with_env(&elems[0], var_types));
                Type::List(Box::new(elem_type))
            }
        }
        HirExpr::Set(elems) => {
            if elems.is_empty() {
                Type::Set(Box::new(Type::Unknown))
            } else {
                // Try to find a non-Unknown element type
                let elem_type = elems
                    .iter()
                    .map(|e| infer_expr_type_with_env(e, var_types))
                    .find(|t| !matches!(t, Type::Unknown))
                    .unwrap_or_else(|| infer_expr_type_with_env(&elems[0], var_types));
                Type::Set(Box::new(elem_type))
            }
        }
        HirExpr::Dict(pairs) => {
            if pairs.is_empty() {
                Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown))
            } else {
                let key_type = infer_expr_type_with_env(&pairs[0].0, var_types);
                let val_type = infer_expr_type_with_env(&pairs[0].1, var_types);
                Type::Dict(Box::new(key_type), Box::new(val_type))
            }
        }
        HirExpr::ListComp { element, .. } => {
            Type::List(Box::new(infer_expr_type_with_env(element, var_types)))
        }
        HirExpr::SetComp { element, .. } => {
            Type::Set(Box::new(infer_expr_type_with_env(element, var_types)))
        }
        HirExpr::DictComp { key, value, .. } => Type::Dict(
            Box::new(infer_expr_type_with_env(key, var_types)),
            Box::new(infer_expr_type_with_env(value, var_types)),
        ),
        // Index expression: arr[i] returns element type of container
        HirExpr::Index { base, .. } => {
            let base_type = infer_expr_type_with_env(base, var_types);
            match base_type {
                Type::List(elem) => *elem,
                Type::Tuple(elems) => elems.first().cloned().unwrap_or(Type::Unknown),
                Type::Dict(_, val) => *val,
                Type::String => Type::String,
                _ => Type::Unknown,
            }
        }
        // For other cases, use the simple version
        _ => infer_expr_type_simple(expr),
    }
}

// NOTE: collect_return_types() removed - replaced by collect_return_types_with_env()
// which provides better type inference using variable type environment

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

            if matches!(op, BinOp::Mul) {
                match (left.as_ref(), right.as_ref()) {
                    // Pattern: [elem] * n
                    (HirExpr::List(elems), &HirExpr::Literal(Literal::Int(size)))
                        if elems.len() == 1 && size > 0 =>
                    {
                        let elem_type = infer_expr_type_simple(&elems[0]);
                        // Non-Copy types always produce Vec (can't use array repeat syntax)
                        return if size <= 32 && is_copy_type(&elem_type) {
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
                        // Non-Copy types always produce Vec (can't use array repeat syntax)
                        return if size <= 32 && is_copy_type(&elem_type) {
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
                // Try to find a non-Unknown element type by scanning all elements
                let elem_type = elems
                    .iter()
                    .map(infer_expr_type_simple)
                    .find(|t| {
                        !matches!(t, Type::Unknown)
                            && !matches!(t, Type::List(inner) if matches!(inner.as_ref(), Type::Unknown))
                    })
                    .unwrap_or_else(|| infer_expr_type_simple(&elems[0]));
                Type::List(Box::new(elem_type))
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
                // Try to find a non-Unknown element type
                let elem_type = elems
                    .iter()
                    .map(infer_expr_type_simple)
                    .find(|t| !matches!(t, Type::Unknown))
                    .unwrap_or_else(|| infer_expr_type_simple(&elems[0]));
                Type::Set(Box::new(elem_type))
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
        HirExpr::Index { base, .. } => {
            // For arr[i], return element type of the container
            match infer_expr_type_simple(base) {
                Type::List(elem) => *elem,
                Type::Tuple(elems) => elems.first().cloned().unwrap_or(Type::Unknown),
                Type::Dict(_, val) => *val,
                Type::String => Type::String, // string indexing returns char/string
                _ => Type::Int,               // Default to Int for array-like indexing
            }
        }
        HirExpr::Slice { base, .. } => {
            // Slicing returns same container type
            infer_expr_type_simple(base)
        }
        HirExpr::FString { .. } => Type::String,
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
        HirExpr::MethodCall { object, method, .. } => {
            // Check if this is a math module method call (math.exp, math.sin, etc.)
            // These all return f64
            if let HirExpr::Var(module_name) = object.as_ref() {
                if module_name == "math" {
                    match method.as_str() {
                        // All math module functions return float
                        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "sinh"
                        | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" | "sqrt" | "exp"
                        | "expm1" | "log" | "log2" | "log10" | "log1p" | "pow" | "hypot"
                        | "fabs" | "floor" | "ceil" | "trunc" | "copysign" | "fmod" | "modf"
                        | "frexp" | "ldexp" | "degrees" | "radians" | "erf" | "erfc" | "gamma"
                        | "lgamma" => {
                            return Type::Float;
                        }
                        // factorial returns int
                        "factorial" | "gcd" | "lcm" | "comb" | "perm" => return Type::Int,
                        // isnan, isinf, isfinite return bool
                        "isnan" | "isinf" | "isfinite" | "isclose" => return Type::Bool,
                        _ => {}
                    }
                }
            }

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
                _ => Type::Unknown,
            }
        }
        HirExpr::ListComp { element, .. } => Type::List(Box::new(infer_expr_type_simple(element))),
        HirExpr::SetComp { element, .. } => Type::Set(Box::new(infer_expr_type_simple(element))),
        HirExpr::DictComp { key, value, .. } => Type::Dict(
            Box::new(infer_expr_type_simple(key)),
            Box::new(infer_expr_type_simple(value)),
        ),
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
        Literal::Bytes(_) => Type::Unknown,
        Literal::Ellipsis => Type::None,
        Literal::Complex(_, _) => Type::Custom("num::Complex<f64>".to_string()),
    }
}

/// Recursively checks if a type contains Unknown anywhere in its structure
fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::List(elem) => type_contains_unknown(elem),
        Type::Set(elem) => type_contains_unknown(elem),
        Type::Dict(k, v) => type_contains_unknown(k) || type_contains_unknown(v),
        Type::Optional(inner) => type_contains_unknown(inner),
        Type::Tuple(elems) => elems.iter().any(type_contains_unknown),
        Type::Array { element_type, .. } => type_contains_unknown(element_type),
        Type::Union(types) => types.iter().any(type_contains_unknown),
        _ => false,
    }
}

// ========== Phase 3b: Return Type Generation ==========

/// Generate return type with Result wrapper and lifetime handling
///
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
)> {
    // Reset returns_reference flag - each function should start fresh
    // This flag is set later if this specific function returns a reference
    ctx.returns_reference = false;
    ctx.returns_mutable_reference = false;

    let should_infer = type_contains_unknown(&func.ret_type);

    let effective_ret_type = if should_infer {
        // Try to infer from return statements in body
        if let Some(inferred) = infer_return_type_from_body(&func.body) {
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

    // Check if function can fail and needs Result wrapper
    // When error_strategy is Panic, functions panic on error instead of returning Result
    let can_fail = func.properties.can_fail
        && !matches!(
            func.annotations.error_strategy,
            depyler_annotations::ErrorStrategy::Panic
        );
    let mut error_type_str = if can_fail && !func.properties.error_types.is_empty() {
        // Use first error type or generic for mixed types
        if func.properties.error_types.len() == 1 {
            func.properties.error_types[0].clone()
        } else {
            "Box<dyn std::error::Error>".to_string()
        }
    } else {
        "Box<dyn std::error::Error>".to_string()
    };

    if ctx.validator_functions.contains(&func.name) {
        error_type_str = "Box<dyn std::error::Error>".to_string();
    }

    // If Box<dyn Error>, we need to wrap exceptions with Box::new()
    // If concrete type, no wrapping needed
    let error_type = if can_fail {
        Some(if error_type_str.contains("Box<dyn") {
            crate::rust_gen::context::ErrorType::DynBox
        } else {
            crate::rust_gen::context::ErrorType::Concrete(error_type_str.clone())
        })
    } else {
        None
    };

    // Only generate error struct definitions when the function actually returns a Result
    // with that error type. This avoids generating error structs for functions that
    // merely contain operations that could fail (like indexing) but don't handle errors.
    // Error structs for raise statements and try/except handlers are generated separately
    // in stmt_gen.rs when those statements are encountered.
    if can_fail {
        if error_type_str.contains("ZeroDivisionError") {
            ctx.needs_zerodivisionerror = true;
        }
        if error_type_str.contains("IndexError") {
            ctx.needs_indexerror = true;
        }
        if error_type_str.contains("ValueError") {
            ctx.needs_valueerror = true;
        }
    }

    let return_type = if matches!(rust_ret_type, crate::type_mapper::RustType::Unit) {
        if can_fail {
            let error_type: syn::Type = syn::parse_str(&error_type_str)
                .unwrap_or_else(|_| parse_quote! { Box<dyn std::error::Error> });
            quote! { -> Result<(), #error_type> }
        } else {
            quote! {}
        }
    } else {
        let mut ty = rust_type_to_syn(&rust_ret_type)?;

        // When a borrowed param's field escapes through return, return a reference instead of cloning
        // e.g., `return state.home_players` where state: &State → return &Vec<Player> instead of Vec<Player>
        // BUT: Never return references for Copy types (i32, f64, bool) - they should be returned by value
        let should_return_reference = !is_copy_type(&func.ret_type)
            && !lifetime_result.params_with_field_return.is_empty()
            && lifetime_result
                .params_with_field_return
                .iter()
                .any(|param_name| {
                    // Check if this param is borrowed
                    lifetime_result
                        .param_lifetimes
                        .get(param_name)
                        .is_some_and(|inferred| inferred.should_borrow)
                });

        // Check if this function's return value is mutated at call sites
        let return_value_is_mutated = ctx.functions_with_mutated_return.contains(&func.name);

        if should_return_reference {
            // Count how many parameters are borrowed references
            let borrowed_param_count = lifetime_result
                .param_lifetimes
                .values()
                .filter(|inf| inf.should_borrow)
                .count();

            // Make the return type a reference with appropriate lifetime
            // When there are multiple borrowed params, we need explicit lifetime
            // If return value is mutated at call sites, use &mut instead of &
            if let Some(ref return_lt) = lifetime_result.return_lifetime {
                let lt = syn::Lifetime::new(return_lt.as_str(), proc_macro2::Span::call_site());
                if return_value_is_mutated {
                    ty = parse_quote! { &#lt mut #ty };
                } else {
                    ty = parse_quote! { &#lt #ty };
                }
            } else if borrowed_param_count > 1 {
                // Multiple borrowed params but elision was applied - we need explicit lifetime
                // Find the lifetime of the param whose field escapes through return
                let escaping_param_lt =
                    lifetime_result
                        .params_with_field_return
                        .first()
                        .and_then(|param_name| {
                            lifetime_result
                                .param_lifetimes
                                .get(param_name)
                                .and_then(|inf| inf.lifetime.clone())
                        });
                if let Some(lt_str) = escaping_param_lt {
                    let lt = syn::Lifetime::new(&lt_str, proc_macro2::Span::call_site());
                    if return_value_is_mutated {
                        ty = parse_quote! { &#lt mut #ty };
                    } else {
                        ty = parse_quote! { &#lt #ty };
                    }
                } else {
                    // No explicit lifetime assigned - need to generate one
                    // Use 'a as the default lifetime for the escaping param
                    if return_value_is_mutated {
                        ty = parse_quote! { &'a mut #ty };
                    } else {
                        ty = parse_quote! { &'a #ty };
                    }
                }
            } else {
                // Single or no borrowed params - elision rules apply
                if return_value_is_mutated {
                    ty = parse_quote! { &mut #ty };
                } else {
                    ty = parse_quote! { &#ty };
                }
            }
            // Signal to expression generator not to add .clone()
            ctx.returns_reference = true;
            if return_value_is_mutated {
                ctx.returns_mutable_reference = true;
            }
        }

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

        if can_fail {
            let error_type: syn::Type = syn::parse_str(&error_type_str)
                .unwrap_or_else(|_| parse_quote! { Box<dyn std::error::Error> });
            quote! { -> Result<#ty, #error_type> }
        } else {
            quote! { -> #ty }
        }
    };

    Ok((return_type, rust_ret_type, can_fail, error_type))
}

// ========== Phase 3c: Generator Implementation ==========
// (Moved to generator_gen.rs in v3.18.0 Phase 4)

impl RustCodeGen for HirFunction {
    fn to_rust_tokens(&self, ctx: &mut CodeGenContext) -> Result<proc_macro2::TokenStream> {
        let name = if is_rust_keyword(&self.name) {
            syn::Ident::new_raw(&self.name, proc_macro2::Span::call_site())
        } else {
            syn::Ident::new(&self.name, proc_macro2::Span::call_site())
        };

        // Store function return type in ctx for later lookup when processing assignments
        // This enables tracking `result = merge(&a, &b)` where merge returns list[int]
        ctx.function_return_types
            .insert(self.name.clone(), self.ret_type.clone());

        // Perform generic type inference
        let mut generic_registry = crate::generic_inference::TypeVarRegistry::new();
        let type_params = generic_registry.infer_function_generics(self)?;

        // Perform lifetime analysis with automatic elision
        let mut lifetime_inference = LifetimeInference::new();
        let lifetime_result = lifetime_inference
            .apply_elision_rules_with_interprocedural(self, ctx.type_mapper, None)
            .unwrap_or_else(|| {
                lifetime_inference.analyze_function_with_interprocedural(
                    self,
                    ctx.type_mapper,
                    None,
                )
            });

        // Generate combined generic parameters (lifetimes + type params)
        let generic_params = codegen_generic_params(&type_params, &lifetime_result.lifetime_params);

        // Generate lifetime bounds
        let where_clause = codegen_where_clause(&lifetime_result.lifetime_bounds);

        // This populates ctx.mutable_vars which codegen_single_param uses to determine `mut` keyword
        // IMPORTANT: Clear mutable_vars before analyzing - each function gets its own analysis
        ctx.mutable_vars.clear();
        // Clear mut ref params tracking - each function tracks its own &mut ref params
        ctx.current_func_mut_ref_params.clear();
        // Clear immutable ref params tracking - each function tracks its own & ref params
        ctx.current_func_ref_params.clear();
        analyze_mutable_vars(&self.body, ctx, &self.params);

        // NOTE: We intentionally do NOT add function_param_muts to mutable_vars here.
        // Direct mutations are detected by analyze_mutable_vars.
        // The function_param_muts is used in codegen_single_param to determine
        // if we need &mut due to call chain propagation.

        // Convert parameters using lifetime analysis results
        let params = codegen_function_params(self, &lifetime_result, ctx)?;

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

        // TODO: This feature is not yet fully implemented - the field was removed
        // from CodeGenContext. Kwargs are currently appended as positional args.
        // See expr_gen.rs line 1165 for current kwargs handling.

        // Generate return type with Result wrapper and lifetime handling
        let (return_type, rust_ret_type, can_fail, error_type) =
            codegen_return_type(self, &lifetime_result, ctx)?;

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

        // Analyze variable usage for clone detection before generating code
        ctx.analyze_var_usage(&self.body);

        // Process function body with proper scoping (expressions will now be rewritten if needed)
        let mut body_stmts = codegen_function_body(self, can_fail, error_type, ctx)?;

        // Clear the subcommand fields context after body generation
        ctx.current_subcommand_fields = None;

        // (hoisted outside function to make Args accessible to handler functions)
        if ctx.argparser_tracker.has_parsers() {
            if let Some(parser_info) = ctx.argparser_tracker.get_first_parser() {
                ctx.needs_clap = true;

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

        // When Python function has `-> None` but uses fallible operations (e.g., indexing),
        // the Rust return type becomes `Result<(), IndexError>` and needs Ok(()) at the end
        // Only add Ok(()) if the function doesn't already end with a return statement

        // This fixes functions with side effects that use error handling (raise/try/except)
        // Also handles Type::Unknown (functions without type annotations that don't explicitly return)
        if can_fail {
            let needs_ok = self
                .body
                .last()
                .is_none_or(|stmt| !matches!(stmt, HirStmt::Return(_)));
            if needs_ok {
                // For functions returning unit type (or Unknown which defaults to unit), add Ok(())
                // For functions returning values with explicit returns, they already have Ok() wrapping
                if matches!(self.ret_type, Type::None | Type::Unknown) {
                    body_stmts.push(parse_quote! { Ok(()) });
                }
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

        Ok(func_tokens)
    }
}
