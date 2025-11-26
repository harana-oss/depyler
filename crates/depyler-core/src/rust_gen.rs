use crate::annotation_aware_type_mapper::AnnotationAwareTypeMapper;
use crate::cargo_toml_gen; // Cargo.toml generation
use crate::hir::*;
use crate::string_optimization::StringOptimizer;
use anyhow::Result;
use quote::{ToTokens, quote};
use std::collections::{HashMap, HashSet};
use syn::{self, parse_quote};

// Module declarations for rust_gen refactoring (v3.18.0 Phases 2-7)
mod argparse_transform;
mod builtins;
mod context;
mod error_gen;
mod expr_gen;
mod format;
mod func_gen;
mod generator_gen;
mod import_gen;
pub mod keywords; // Centralized keyword escaping
mod stmt_gen;
mod type_gen;

// Internal imports
use error_gen::generate_error_type_definitions;
use format::format_rust_code;
use import_gen::process_module_imports;
#[cfg(test)]
use stmt_gen::{
    codegen_assign_attribute, codegen_assign_index, codegen_assign_symbol, codegen_assign_tuple, codegen_break_stmt,
    codegen_continue_stmt, codegen_expr_stmt, codegen_pass_stmt, codegen_raise_stmt, codegen_return_stmt,
    codegen_try_stmt, codegen_while_stmt, codegen_with_stmt,
};

// Public re-exports for external modules (union_enum_gen, etc.)
pub use argparse_transform::ArgParserTracker; // Export for testing
pub use context::{CodeGenContext, RustCodeGen, ToRustExpr};
pub use type_gen::rust_type_to_syn;

// Internal re-exports for cross-module access
pub(crate) use func_gen::return_type_expects_float;

///
/// Scans all statements in function bodies and constant expressions to find
/// add_argument(type=validator_func) calls. Populates ctx.validator_functions
/// with function names used as type= parameters.
/// This must run BEFORE function signature generation so parameter types can be corrected.
///
fn analyze_validators(ctx: &mut CodeGenContext, functions: &[HirFunction], constants: &[HirConstant]) {
    // Scan function bodies
    for func in functions {
        scan_stmts_for_validators(&func.body, ctx);
    }

    // Scan constant expressions (module-level code)
    for constant in constants {
        scan_expr_for_validators(&constant.value, ctx);
    }
}

/// Helper: Recursively scan statements for add_argument(type=...) calls
fn scan_stmts_for_validators(stmts: &[HirStmt], ctx: &mut CodeGenContext) {
    for stmt in stmts {
        match stmt {
            HirStmt::Expr(expr) => {
                scan_expr_for_validators(expr, ctx);
            }
            HirStmt::If {
                then_body, else_body, ..
            } => {
                scan_stmts_for_validators(then_body, ctx);
                if let Some(ref else_stmts) = else_body {
                    scan_stmts_for_validators(else_stmts, ctx);
                }
            }
            HirStmt::While { body, .. } => {
                scan_stmts_for_validators(body, ctx);
            }
            HirStmt::For { body, .. } => {
                scan_stmts_for_validators(body, ctx);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                scan_stmts_for_validators(body, ctx);
                for handler in handlers {
                    scan_stmts_for_validators(&handler.body, ctx);
                }
                if let Some(ref else_stmts) = orelse {
                    scan_stmts_for_validators(else_stmts, ctx);
                }
                if let Some(ref final_stmts) = finalbody {
                    scan_stmts_for_validators(final_stmts, ctx);
                }
            }
            _ => {}
        }
    }
}

/// Helper: Scan expression for add_argument method calls
fn scan_expr_for_validators(expr: &HirExpr, ctx: &mut CodeGenContext) {
    match expr {
        HirExpr::MethodCall { method, kwargs, .. } if method == "add_argument" => {
            // Check for type= parameter
            for (kw_name, kw_value) in kwargs {
                if kw_name == "type" {
                    if let HirExpr::Var(type_name) = kw_value {
                        // Skip built-in types
                        if !matches!(type_name.as_str(), "str" | "int" | "float" | "Path") {
                            ctx.validator_functions.insert(type_name.clone());
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Pre-analyze all functions to determine which parameters need mutable borrows.
/// This must run BEFORE function code generation so call sites know whether to use &mut.
///
/// The analysis is done in two passes:
/// 1. First pass: detect direct mutations for all functions
/// 2. Second pass: propagate mutations through call chains (if param.field is passed
///    to a function that mutates its parameter, the original param also needs &mut)
///
/// Populates ctx.function_param_muts with function_name -> Vec<bool>
/// where each bool indicates if the corresponding parameter needs &mut.
fn pre_analyze_parameter_mutability(ctx: &mut CodeGenContext, functions: &[HirFunction]) {
    // Pass 1: Direct mutation analysis
    for func in functions {
        let param_muts: Vec<bool> = func
            .params
            .iter()
            .map(|param| is_parameter_mutated(&param.name, &func.body))
            .collect();
        ctx.function_param_muts.insert(func.name.clone(), param_muts);
    }

    // Pass 2: Propagate mutations through call chains
    // If a function passes param.field to another function that mutates it,
    // the param itself needs to be mutable
    let mut changed = true;
    while changed {
        changed = false;
        for func in functions {
            for (param_idx, param) in func.params.iter().enumerate() {
                // Skip if already marked as needing mut
                if ctx
                    .function_param_muts
                    .get(&func.name)
                    .and_then(|v| v.get(param_idx))
                    .copied()
                    .unwrap_or(false)
                {
                    continue;
                }

                // Check if any call passes param.field to a function that mutates it
                if param_attr_passed_to_mutating_func(&param.name, &func.body, &ctx.function_param_muts) {
                    if let Some(muts) = ctx.function_param_muts.get_mut(&func.name) {
                        if let Some(m) = muts.get_mut(param_idx) {
                            *m = true;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // Pass 3: Propagate &mut requirement DOWN the call chain
    // If a function has &mut param and passes it to another function,
    // that called function must also accept &mut (for type compatibility)
    let mut changed = true;
    while changed {
        changed = false;
        for func in functions {
            for (param_idx, param) in func.params.iter().enumerate() {
                // Only propagate from params that are already marked as needing mut
                let needs_mut = ctx
                    .function_param_muts
                    .get(&func.name)
                    .and_then(|v| v.get(param_idx))
                    .copied()
                    .unwrap_or(false);

                if needs_mut {
                    // Find all functions called with this param
                    propagate_mut_to_callees(&param.name, &func.body, &mut ctx.function_param_muts, &mut changed);
                }
            }
        }
    }

    // Debug: print final function_param_muts
    #[cfg(debug_assertions)]
    for (func_name, muts) in &ctx.function_param_muts {
        eprintln!("DEBUG function_param_muts: {} = {:?}", func_name, muts);
    }
}

/// Pre-analyze all functions to determine which parameters should be borrowed vs owned.
/// This must run AFTER pre_analyze_parameter_mutability and BEFORE function code generation
/// so that call sites know whether to add & or .to_string() for arguments.
///
/// Populates ctx.function_param_borrows with function_name -> Vec<bool>
/// where each bool indicates if the corresponding parameter is borrowed (true = &str, false = String).
fn pre_analyze_parameter_borrowing(ctx: &mut CodeGenContext, functions: &[HirFunction]) {
    use crate::borrowing_context::BorrowingContext;
    use crate::lifetime_analysis::LifetimeInference;

    for func in functions {
        let mut lifetime_inference = LifetimeInference::new();
        let lifetime_result = lifetime_inference.analyze_function(func, ctx.type_mapper);

        let param_borrows: Vec<bool> = func
            .params
            .iter()
            .map(|param| {
                lifetime_result
                    .param_lifetimes
                    .get(&param.name)
                    .map(|inferred| inferred.should_borrow)
                    .unwrap_or(false) // Default to NOT borrowed (owned) if unknown
            })
            .collect();

        ctx.function_param_borrows.insert(func.name.clone(), param_borrows);
    }
}

/// Propagate &mut requirement to called functions
fn propagate_mut_to_callees(
    param_name: &str,
    body: &[HirStmt],
    function_param_muts: &mut HashMap<String, Vec<bool>>,
    changed: &mut bool,
) {
    for stmt in body {
        propagate_mut_to_callees_stmt(param_name, stmt, function_param_muts, changed);
    }
}

fn propagate_mut_to_callees_stmt(
    param_name: &str,
    stmt: &HirStmt,
    function_param_muts: &mut HashMap<String, Vec<bool>>,
    changed: &mut bool,
) {
    match stmt {
        HirStmt::Expr(expr) | HirStmt::Assign { value: expr, .. } => {
            propagate_mut_to_callees_expr(param_name, expr, function_param_muts, changed);
        }
        HirStmt::If {
            then_body,
            else_body,
            condition,
            ..
        } => {
            propagate_mut_to_callees_expr(param_name, condition, function_param_muts, changed);
            for s in then_body {
                propagate_mut_to_callees_stmt(param_name, s, function_param_muts, changed);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    propagate_mut_to_callees_stmt(param_name, s, function_param_muts, changed);
                }
            }
        }
        HirStmt::While { body, condition, .. } => {
            propagate_mut_to_callees_expr(param_name, condition, function_param_muts, changed);
            for s in body {
                propagate_mut_to_callees_stmt(param_name, s, function_param_muts, changed);
            }
        }
        HirStmt::For { body, .. } => {
            for s in body {
                propagate_mut_to_callees_stmt(param_name, s, function_param_muts, changed);
            }
        }
        _ => {}
    }
}

fn propagate_mut_to_callees_expr(
    param_name: &str,
    expr: &HirExpr,
    function_param_muts: &mut HashMap<String, Vec<bool>>,
    changed: &mut bool,
) {
    match expr {
        HirExpr::Call { func, args, .. } => {
            // Check if param is passed to this function
            for (arg_idx, arg) in args.iter().enumerate() {
                let is_param = matches!(arg, HirExpr::Var(name) if name == param_name);
                let is_param_attr = is_param_attribute_access(param_name, arg);

                if is_param || is_param_attr {
                    // Mark this function's parameter as needing mut
                    if let Some(muts) = function_param_muts.get_mut(func) {
                        if let Some(m) = muts.get_mut(arg_idx) {
                            if !*m {
                                *m = true;
                                *changed = true;
                            }
                        }
                    }
                }
            }
            // Recurse into args
            for arg in args {
                propagate_mut_to_callees_expr(param_name, arg, function_param_muts, changed);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            propagate_mut_to_callees_expr(param_name, left, function_param_muts, changed);
            propagate_mut_to_callees_expr(param_name, right, function_param_muts, changed);
        }
        _ => {}
    }
}

/// Check if param.field is passed to a function that mutates that parameter position
fn param_attr_passed_to_mutating_func(
    param_name: &str,
    body: &[HirStmt],
    function_param_muts: &HashMap<String, Vec<bool>>,
) -> bool {
    for stmt in body {
        if stmt_passes_param_attr_to_mutating_func(param_name, stmt, function_param_muts) {
            return true;
        }
    }
    false
}

fn stmt_passes_param_attr_to_mutating_func(
    param_name: &str,
    stmt: &HirStmt,
    function_param_muts: &HashMap<String, Vec<bool>>,
) -> bool {
    match stmt {
        HirStmt::Expr(expr) | HirStmt::Assign { value: expr, .. } => {
            expr_passes_param_attr_to_mutating_func(param_name, expr, function_param_muts)
        }
        HirStmt::If {
            then_body,
            else_body,
            condition,
            ..
        } => {
            expr_passes_param_attr_to_mutating_func(param_name, condition, function_param_muts)
                || body_passes_param_attr_to_mutating_func(param_name, then_body, function_param_muts)
                || else_body
                    .as_ref()
                    .is_some_and(|eb| body_passes_param_attr_to_mutating_func(param_name, eb, function_param_muts))
        }
        HirStmt::While { body, condition, .. } => {
            expr_passes_param_attr_to_mutating_func(param_name, condition, function_param_muts)
                || body_passes_param_attr_to_mutating_func(param_name, body, function_param_muts)
        }
        HirStmt::For { body, .. } => body_passes_param_attr_to_mutating_func(param_name, body, function_param_muts),
        HirStmt::Return(Some(expr)) => expr_passes_param_attr_to_mutating_func(param_name, expr, function_param_muts),
        _ => false,
    }
}

fn body_passes_param_attr_to_mutating_func(
    param_name: &str,
    body: &[HirStmt],
    function_param_muts: &HashMap<String, Vec<bool>>,
) -> bool {
    body.iter()
        .any(|s| stmt_passes_param_attr_to_mutating_func(param_name, s, function_param_muts))
}

fn expr_passes_param_attr_to_mutating_func(
    param_name: &str,
    expr: &HirExpr,
    function_param_muts: &HashMap<String, Vec<bool>>,
) -> bool {
    match expr {
        HirExpr::Call { func, args, kwargs } => {
            // Check each argument to see if it's the param itself or param.field
            for (arg_idx, arg) in args.iter().enumerate() {
                // Check if argument is param.field
                if is_param_attribute_access(param_name, arg) {
                    // Check if this function's parameter at arg_idx needs mut
                    if function_param_muts
                        .get(func)
                        .and_then(|muts| muts.get(arg_idx))
                        .copied()
                        .unwrap_or(false)
                    {
                        return true;
                    }
                }
                // Also check if argument is param itself (direct pass)
                if let HirExpr::Var(var_name) = arg {
                    if var_name == param_name {
                        // Check if this function's parameter at arg_idx needs mut
                        if function_param_muts
                            .get(func)
                            .and_then(|muts| muts.get(arg_idx))
                            .copied()
                            .unwrap_or(false)
                        {
                            return true;
                        }
                    }
                }
            }
            // Also check kwargs
            for (_, arg) in kwargs {
                if is_param_attribute_access(param_name, arg) {
                    // Conservative: if any kwarg is param.field, might need mut
                    // (We'd need to know param positions for kwargs to be precise)
                }
            }
            // Recursively check args and kwargs
            args.iter()
                .any(|a| expr_passes_param_attr_to_mutating_func(param_name, a, function_param_muts))
                || kwargs
                    .iter()
                    .any(|(_, v)| expr_passes_param_attr_to_mutating_func(param_name, v, function_param_muts))
        }
        HirExpr::MethodCall { args, .. } => args
            .iter()
            .any(|a| expr_passes_param_attr_to_mutating_func(param_name, a, function_param_muts)),
        HirExpr::Binary { left, right, .. } => {
            expr_passes_param_attr_to_mutating_func(param_name, left, function_param_muts)
                || expr_passes_param_attr_to_mutating_func(param_name, right, function_param_muts)
        }
        HirExpr::Unary { operand, .. } => {
            expr_passes_param_attr_to_mutating_func(param_name, operand, function_param_muts)
        }
        _ => false,
    }
}

/// Check if expression is an attribute access on a specific parameter
/// e.g., state.data where param_name == "state"
fn is_param_attribute_access(param_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } => {
            if let HirExpr::Var(var_name) = value.as_ref() {
                var_name == param_name
            } else {
                is_param_attribute_access(param_name, value)
            }
        }
        _ => false,
    }
}

/// Check if a parameter is mutated in the function body
fn is_parameter_mutated(param_name: &str, body: &[HirStmt]) -> bool {
    for stmt in body {
        if stmt_mutates_param(param_name, stmt) {
            return true;
        }
    }
    false
}

/// Check if a statement mutates a specific parameter
fn stmt_mutates_param(param_name: &str, stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            // Check if target is the parameter or an attribute of it
            let target_mutates = match target {
                AssignTarget::Symbol(name) if name == param_name => true,
                AssignTarget::Attribute { value: base, .. } => {
                    if let HirExpr::Var(var_name) = base.as_ref() {
                        var_name == param_name
                    } else {
                        expr_contains_param_mutation(param_name, base)
                    }
                }
                AssignTarget::Index { base, .. } => {
                    if let HirExpr::Var(var_name) = base.as_ref() {
                        var_name == param_name
                    } else {
                        false
                    }
                }
                _ => false,
            };
            target_mutates || expr_mutates_param(param_name, value)
        }
        HirStmt::Expr(expr) => expr_mutates_param(param_name, expr),
        HirStmt::If {
            then_body,
            else_body,
            condition,
            ..
        } => {
            expr_mutates_param(param_name, condition)
                || body_mutates_param(param_name, then_body)
                || else_body.as_ref().is_some_and(|eb| body_mutates_param(param_name, eb))
        }
        HirStmt::While { body, condition, .. } => {
            expr_mutates_param(param_name, condition) || body_mutates_param(param_name, body)
        }
        HirStmt::For { target, iter, body, .. } => {
            // Check if the iterator expression is an attribute access on the parameter
            // and the loop body mutates the loop variable (requires &mut iteration)
            let loop_var = match target {
                AssignTarget::Symbol(name) => Some(name.as_str()),
                _ => None,
            };

            let iter_on_param = matches_param_attribute(param_name, iter);
            let body_mutates_loop_var = loop_var.is_some_and(|lv| body_mutates_param(lv, body));

            // If iterating over param.field and mutating loop var, param needs &mut
            (iter_on_param && body_mutates_loop_var) || body_mutates_param(param_name, body)
        }
        HirStmt::Return(Some(expr)) => expr_mutates_param(param_name, expr),
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            body_mutates_param(param_name, body)
                || handlers.iter().any(|h| body_mutates_param(param_name, &h.body))
                || orelse.as_ref().is_some_and(|o| body_mutates_param(param_name, o))
                || finalbody.as_ref().is_some_and(|f| body_mutates_param(param_name, f))
        }
        _ => false,
    }
}

fn body_mutates_param(param_name: &str, body: &[HirStmt]) -> bool {
    body.iter().any(|stmt| stmt_mutates_param(param_name, stmt))
}

/// Check if an expression mutates a parameter (via method call)
fn expr_mutates_param(param_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::MethodCall {
            object, method, args, ..
        } => {
            // Check if this is a mutating method call on the parameter or its attributes
            // e.g., state.append(...) or state.data.update(...)
            let object_involves_param = expr_involves_param(param_name, object);
            if object_involves_param && is_mutating_method_name(method) {
                return true;
            }
            // Only recurse into args to check for mutations there
            args.iter().any(|a| expr_mutates_param(param_name, a))
        }
        HirExpr::Call { func: _, args, kwargs } => {
            // Check if the parameter is passed to a function that mutates it
            // This requires checking ctx.function_param_muts, but we're pre-populating
            // so we can't check other functions yet. For now, just check args.
            args.iter().any(|a| expr_mutates_param(param_name, a))
                || kwargs.iter().any(|(_, v)| expr_mutates_param(param_name, v))
        }
        HirExpr::Binary { left, right, .. } => {
            expr_mutates_param(param_name, left) || expr_mutates_param(param_name, right)
        }
        HirExpr::Unary { operand, .. } => expr_mutates_param(param_name, operand),
        HirExpr::IfExpr { test, body, orelse } => {
            expr_mutates_param(param_name, test)
                || expr_mutates_param(param_name, body)
                || expr_mutates_param(param_name, orelse)
        }
        HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) => {
            items.iter().any(|i| expr_mutates_param(param_name, i))
        }
        HirExpr::Dict(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_mutates_param(param_name, k) || expr_mutates_param(param_name, v)),
        HirExpr::Index { base, index } => expr_mutates_param(param_name, base) || expr_mutates_param(param_name, index),
        HirExpr::Attribute { value, .. } => expr_mutates_param(param_name, value),
        _ => false,
    }
}

/// Check if expression involves a parameter (directly or via attribute access)
/// This is used to check if a method call might affect the parameter.
/// e.g., expr_involves_param("state", state) => true
/// e.g., expr_involves_param("state", state.data) => true
fn expr_involves_param(param_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(name) => name == param_name,
        HirExpr::Attribute { value, .. } => expr_involves_param(param_name, value),
        _ => false,
    }
}

/// Check if expression contains nested param mutation (for attribute access chains)
fn expr_contains_param_mutation(param_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(name) => name == param_name,
        HirExpr::Attribute { value, .. } => expr_contains_param_mutation(param_name, value),
        _ => false,
    }
}

/// Check if a method name is known to mutate its receiver
fn is_mutating_method_name(method: &str) -> bool {
    matches!(
        method,
        // List methods
        "append" | "extend" | "insert" | "remove" | "pop" | "clear" | "reverse" | "sort" |
        // Dict methods  
        "update" | "setdefault" | "popitem" |
        // Set methods
        "add" | "discard" | "difference_update" | "intersection_update" |
        // Rust Vec methods (in case they're used)
        "push" | "truncate"
    )
}

/// Check if an expression is an attribute access on a specific parameter
/// e.g., matches_param_attribute("state", state.items) => true
fn matches_param_attribute(param_name: &str, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } => {
            if let HirExpr::Var(var_name) = value.as_ref() {
                var_name == param_name
            } else {
                matches_param_attribute(param_name, value)
            }
        }
        _ => false,
    }
}

/// Analyze which variables are reassigned (mutated) in a list of statements
///
/// Populates ctx.mutable_vars with variables that are:
/// 1. Reassigned after declaration (x = 1; x = 2)
/// 2. Mutated via method calls (.push(), .extend(), .insert(), .remove(), .pop(), etc.)
/// 3. Function parameters that are reassigned (requires mut)
///
fn analyze_mutable_vars(stmts: &[HirStmt], ctx: &mut CodeGenContext, params: &[HirParam]) {
    let mut declared = HashSet::new();

    // This allows the reassignment detection logic below to catch parameter mutations
    // Example: def gcd(a, b): a = temp  # Now detected as reassignment → mut a
    for param in params {
        declared.insert(param.name.clone());
    }

    fn analyze_expr_for_mutations(
        expr: &HirExpr,
        mutable: &mut HashSet<String>,
        var_types: &HashMap<String, String>,
        mutating_methods: &HashMap<String, HashSet<String>>,
    ) {
        match expr {
            HirExpr::MethodCall {
                object, method, args, ..
            } => {
                // Check if this is a mutating method call
                let is_mut = if is_mutating_method(method) {
                    // Built-in mutating method
                    true
                } else if let HirExpr::Var(var_name) = &**object {
                    // Check if this is a user-defined mutating method
                    if let Some(class_name) = var_types.get(var_name) {
                        if let Some(mut_methods) = mutating_methods.get(class_name) {
                            mut_methods.contains(method)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_mut {
                    if let HirExpr::Var(var_name) = &**object {
                        mutable.insert(var_name.clone());
                    }
                }
                // Recursively check nested expressions
                analyze_expr_for_mutations(object, mutable, var_types, mutating_methods);
                for arg in args {
                    analyze_expr_for_mutations(arg, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                analyze_expr_for_mutations(left, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(right, mutable, var_types, mutating_methods);
            }
            HirExpr::Unary { operand, .. } => {
                analyze_expr_for_mutations(operand, mutable, var_types, mutating_methods);
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    analyze_expr_for_mutations(arg, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::IfExpr { test, body, orelse } => {
                analyze_expr_for_mutations(test, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(body, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(orelse, mutable, var_types, mutating_methods);
            }
            HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) | HirExpr::FrozenSet(items) => {
                for item in items {
                    analyze_expr_for_mutations(item, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Dict(pairs) => {
                for (key, value) in pairs {
                    analyze_expr_for_mutations(key, mutable, var_types, mutating_methods);
                    analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);
                }
            }
            HirExpr::Index { base, index } => {
                analyze_expr_for_mutations(base, mutable, var_types, mutating_methods);
                analyze_expr_for_mutations(index, mutable, var_types, mutating_methods);
            }
            HirExpr::Attribute { value, .. } => {
                analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);
            }
            _ => {}
        }
    }

    fn is_mutating_method(method: &str) -> bool {
        matches!(
            method,
            // List methods
            "append" | "extend" | "insert" | "remove" | "pop" | "clear" | "reverse" | "sort" |
            // Dict methods
            "update" | "setdefault" | "popitem" |
            // Set methods
            "add" | "discard" | "difference_update" | "intersection_update"
        )
    }

    fn analyze_stmt(
        stmt: &HirStmt,
        declared: &mut HashSet<String>,
        mutable: &mut HashSet<String>,
        var_types: &mut HashMap<String, String>,
        mutating_methods: &HashMap<String, HashSet<String>>,
    ) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // Check if the value expression contains method calls that mutate variables
                analyze_expr_for_mutations(value, mutable, var_types, mutating_methods);

                match target {
                    AssignTarget::Symbol(name) => {
                        // Track variable type if assigned from class constructor
                        if let HirExpr::Call { func, .. } = value {
                            // Store the type (class name) for this variable
                            var_types.insert(name.clone(), func.clone());
                        }

                        if declared.contains(name) {
                            // Variable is being reassigned - mark as mutable
                            mutable.insert(name.clone());
                        } else {
                            // First declaration
                            declared.insert(name.clone());
                        }
                    }
                    AssignTarget::Tuple(targets) => {
                        // Tuple assignment - analyze each element
                        for t in targets {
                            if let AssignTarget::Symbol(name) = t {
                                if declared.contains(name) {
                                    // Variable is being reassigned - mark as mutable
                                    mutable.insert(name.clone());
                                } else {
                                    // First declaration
                                    declared.insert(name.clone());
                                }
                            }
                        }
                    }
                    AssignTarget::Attribute { value, .. } => {
                        // e.g., `b.size = 20` requires `let mut b = ...`
                        if let HirExpr::Var(var_name) = value.as_ref() {
                            mutable.insert(var_name.clone());
                        }
                    }
                    AssignTarget::Index { base, .. } => {
                        // e.g., `arr[i] = value` requires `let mut arr = ...`
                        if let HirExpr::Var(var_name) = base.as_ref() {
                            mutable.insert(var_name.clone());
                        }
                    }
                }
            }
            HirStmt::Expr(expr) => {
                // Check standalone expressions for method calls (e.g., numbers.push(4))
                analyze_expr_for_mutations(expr, mutable, var_types, mutating_methods);
            }
            HirStmt::Return(Some(expr)) => {
                analyze_expr_for_mutations(expr, mutable, var_types, mutating_methods);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                analyze_expr_for_mutations(condition, mutable, var_types, mutating_methods);
                for stmt in then_body {
                    analyze_stmt(stmt, declared, mutable, var_types, mutating_methods);
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        analyze_stmt(stmt, declared, mutable, var_types, mutating_methods);
                    }
                }
            }
            HirStmt::While { condition, body, .. } => {
                analyze_expr_for_mutations(condition, mutable, var_types, mutating_methods);
                for stmt in body {
                    analyze_stmt(stmt, declared, mutable, var_types, mutating_methods);
                }
            }
            HirStmt::For { target, iter, body } => {
                // Check if iterating over a parameter's field and mutating the loop variable
                // This requires the parameter itself to be mutable
                let loop_var = match target {
                    AssignTarget::Symbol(name) => Some(name.as_str()),
                    _ => None,
                };

                // Check if any param in `declared` (function parameters) is being iterated
                // and the loop body mutates the loop variable
                if let Some(lv) = loop_var {
                    // Temporarily add loop var to declared
                    declared.insert(lv.to_string());

                    // Check if any statement in body mutates the loop variable
                    let body_mutates_loop_var = body.iter().any(|stmt| {
                        if let HirStmt::Assign {
                            target: AssignTarget::Attribute { value, .. },
                            ..
                        } = stmt
                        {
                            if let HirExpr::Var(var_name) = value.as_ref() {
                                var_name == lv
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    });

                    // If the iterator is an attribute access and body mutates loop var,
                    // mark the base as mutable
                    if body_mutates_loop_var {
                        if let HirExpr::Attribute { value, .. } = iter {
                            if let HirExpr::Var(var_name) = value.as_ref() {
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                }

                for stmt in body {
                    analyze_stmt(stmt, declared, mutable, var_types, mutating_methods);
                }
            }
            _ => {}
        }
    }

    let mut var_types = HashMap::new();
    let mutating_methods = &ctx.mutating_methods;
    for stmt in stmts {
        analyze_stmt(
            stmt,
            &mut declared,
            &mut ctx.mutable_vars,
            &mut var_types,
            mutating_methods,
        );
    }
}

/// Convert Python classes to Rust structs
///
/// Processes all classes and generates token streams.
fn convert_classes_to_rust(
    classes: &[HirClass],
    type_mapper: &crate::type_mapper::TypeMapper,
) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut class_items = Vec::new();
    for class in classes {
        let items = crate::direct_rules::convert_class_to_struct(class, type_mapper)?;
        for item in items {
            let tokens = item.to_token_stream();
            class_items.push(tokens);
        }
    }
    Ok(class_items)
}

/// Convert HIR functions to Rust token streams
///
/// Processes all functions using the code generation context.
fn convert_functions_to_rust(
    functions: &[HirFunction],
    ctx: &mut CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    functions
        .iter()
        .map(|f| f.to_rust_tokens(ctx))
        .collect::<Result<Vec<_>>>()
}

/// Generate conditional imports based on code generation context
///
/// Adds imports for collections and smart pointers as needed.
/// Deduplicate use statements to avoid E0252 errors
///
/// For example, both generate_import_tokens and generate_conditional_imports
/// might add `use std::collections::HashMap;`.
///
/// # Complexity
/// ~6 (loop + if + string ops)
fn deduplicate_use_statements(items: Vec<proc_macro2::TokenStream>) -> Vec<proc_macro2::TokenStream> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for item in items {
        let item_str = item.to_string();
        // Only deduplicate use statements
        if item_str.starts_with("use ") {
            if seen.insert(item_str) {
                deduped.push(item);
            }
            // else: skip duplicate
        } else {
            // Non-import items: always keep
            deduped.push(item);
        }
    }

    deduped
}

fn generate_conditional_imports(ctx: &CodeGenContext) -> Vec<proc_macro2::TokenStream> {
    let mut imports = Vec::new();

    // Define all possible conditional imports
    let conditional_imports = [
        (ctx.needs_hashmap, quote! { use std::collections::HashMap; }),
        (ctx.needs_hashset, quote! { use std::collections::HashSet; }),
        (ctx.needs_vecdeque, quote! { use std::collections::VecDeque; }),
        (ctx.needs_fnv_hashmap, quote! { use fnv::FnvHashMap; }),
        (ctx.needs_ahash_hashmap, quote! { use ahash::AHashMap; }),
        (ctx.needs_arc, quote! { use std::sync::Arc; }),
        (ctx.needs_rc, quote! { use std::rc::Rc; }),
        (ctx.needs_cow, quote! { use std::borrow::Cow; }),
        (ctx.needs_serde_json, quote! { use serde_json; }),
    ];

    // Add imports where needed
    for (needed, import_tokens) in conditional_imports {
        if needed {
            imports.push(import_tokens);
        }
    }

    imports
}

/// Generate import token streams from Python imports
///
/// Maps Python imports to Rust use statements.
///
fn generate_import_tokens(
    imports: &[Import],
    module_mapper: &crate::module_mapper::ModuleMapper,
) -> Vec<proc_macro2::TokenStream> {
    let mut items = Vec::new();
    let mut external_imports = Vec::new();
    let mut std_imports = Vec::new();

    // Categorize imports
    for import in imports {
        let rust_imports = module_mapper.map_import(import);
        for rust_import in rust_imports {
            if rust_import.path.starts_with("//") {
                // Comment for unmapped imports
                let comment = &rust_import.path;
                items.push(quote! { #[doc = #comment] });
            } else if rust_import.is_external {
                external_imports.push(rust_import);
            } else {
                std_imports.push(rust_import);
            }
        }
    }

    // Multiple Python imports can map to same Rust type (e.g., defaultdict + Counter -> HashMap)
    let mut seen_paths = std::collections::HashSet::new();

    // Add external imports (deduplicated)
    for import in external_imports {
        // Create unique key from path + alias
        let key = format!("{}:{:?}", import.path, import.alias);
        if !seen_paths.insert(key) {
            continue; // Skip duplicate
        }

        let path: syn::Path = syn::parse_str(&import.path).unwrap_or_else(|_| parse_quote! { unknown });
        if let Some(alias) = import.alias {
            let alias_ident = syn::Ident::new(&alias, proc_macro2::Span::call_site());
            items.push(quote! { use #path as #alias_ident; });
        } else {
            items.push(quote! { use #path; });
        }
    }

    // Add standard library imports (deduplicated)
    for import in std_imports {
        // Skip typing imports as they're handled by the type system
        if import.path.starts_with("::") || import.path.is_empty() {
            continue;
        }

        // Create unique key from path + alias
        let key = format!("{}:{:?}", import.path, import.alias);
        if !seen_paths.insert(key) {
            continue; // Skip duplicate
        }

        let path: syn::Path = syn::parse_str(&import.path).unwrap_or_else(|_| parse_quote! { std });
        if let Some(alias) = import.alias {
            let alias_ident = syn::Ident::new(&alias, proc_macro2::Span::call_site());
            items.push(quote! { use #path as #alias_ident; });
        } else {
            items.push(quote! { use #path; });
        }
    }

    items
}

/// Generate module-level constant tokens
///
/// Generates `pub const` declarations for module-level constants.
/// For simple literal values (int, float, string, bool), generates const.
/// For complex expressions, may need to use static or lazy_static.
fn generate_constant_tokens(
    constants: &[HirConstant],
    ctx: &mut CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    use crate::rust_gen::context::ToRustExpr;

    let mut items = Vec::new();

    for constant in constants {
        let name_ident = syn::Ident::new(&constant.name, proc_macro2::Span::call_site());

        // Generate the value expression
        let value_expr = constant.value.to_rust_expr(ctx)?;

        // Generate type annotation - required for Rust const
        let type_annotation = if let Some(ref ty) = constant.type_annotation {
            let rust_type = ctx.type_mapper.map_type(ty);
            let syn_type = type_gen::rust_type_to_syn(&rust_type)?;
            quote! { : #syn_type }
        } else {
            match &constant.value {
                // Literal types
                HirExpr::Literal(Literal::Int(_)) => quote! { : i32 },
                HirExpr::Literal(Literal::Float(_)) => quote! { : f64 },
                HirExpr::Literal(Literal::String(_)) => quote! { : &str },
                HirExpr::Literal(Literal::Bool(_)) => quote! { : bool },

                HirExpr::Dict { .. } => {
                    ctx.needs_serde_json = true;
                    quote! { : serde_json::Value }
                }

                HirExpr::List { .. } => {
                    ctx.needs_serde_json = true;
                    quote! { : serde_json::Value }
                }

                _ => {
                    ctx.needs_serde_json = true;
                    quote! { : serde_json::Value }
                }
            }
        };

        // Generate the constant declaration
        // Use pub const for module-level visibility
        items.push(quote! {
            pub const #name_ident #type_annotation = #value_expr;
        });
    }

    Ok(items)
}

/// Generate a complete Rust file from HIR module
pub fn generate_rust_file(
    module: &HirModule,
    type_mapper: &crate::type_mapper::TypeMapper,
) -> Result<(String, Vec<cargo_toml_gen::Dependency>)> {
    let module_mapper = crate::module_mapper::ModuleMapper::new();

    // Process imports to populate the context
    let (imported_modules, imported_items) = process_module_imports(&module.imports, &module_mapper);

    // Extract class names from module
    let class_names: HashSet<String> = module.classes.iter().map(|class| class.name.clone()).collect();

    let mut mutating_methods: std::collections::HashMap<String, HashSet<String>> = std::collections::HashMap::new();
    for class in &module.classes {
        let mut mut_methods = HashSet::new();
        for method in &class.methods {
            if crate::direct_rules::method_mutates_self(method) {
                mut_methods.insert(method.name.clone());
            }
        }
        mutating_methods.insert(class.name.clone(), mut_methods);
    }

    let mut ctx = CodeGenContext {
        type_mapper,
        annotation_aware_mapper: AnnotationAwareTypeMapper::with_base_mapper(type_mapper.clone()),
        string_optimizer: StringOptimizer::new(),
        union_enum_generator: crate::union_enum_gen::UnionEnumGenerator::new(),
        generated_enums: Vec::new(),
        needs_hashmap: false,
        needs_hashset: false,
        needs_vecdeque: false,
        needs_fnv_hashmap: false,
        needs_ahash_hashmap: false,
        needs_arc: false,
        needs_rc: false,
        needs_cow: false,
        needs_rand: false,
        needs_serde_json: false,
        needs_regex: false,
        needs_chrono: false,
        needs_clap: false,
        needs_csv: false,
        needs_rust_decimal: false,
        needs_num_rational: false,
        needs_base64: false,
        needs_md5: false,
        needs_sha2: false,
        needs_sha3: false,
        needs_blake2: false,
        needs_hex: false,
        needs_uuid: false,
        needs_hmac: false,
        needs_crc32: false,
        needs_url_encoding: false,
        declared_vars: vec![HashSet::new()],
        current_function_can_fail: false,
        current_return_type: None,
        module_mapper,
        imported_modules,
        imported_items,
        mutable_vars: HashSet::new(),
        needs_zerodivisionerror: false,
        needs_indexerror: false,
        needs_valueerror: false,
        needs_argumenttypeerror: false,
        in_generator: false,
        is_classmethod: false,
        generator_state_vars: HashSet::new(),
        var_types: std::collections::HashMap::new(),
        class_names,
        mutating_methods,
        function_return_types: std::collections::HashMap::new(), // Track function return types
        function_param_borrows: std::collections::HashMap::new(), // Track parameter borrowing
        function_param_muts: std::collections::HashMap::new(),   // Track parameters needing &mut
        tuple_iter_vars: HashSet::new(),                         // Track tuple iteration variables
        is_final_statement: false,                               // Track final statement for expression-based returns
        result_bool_functions: HashSet::new(),                   // Track functions returning Result<bool>
        result_returning_functions: HashSet::new(),              // Track ALL Result-returning functions
        current_error_type: None,                                // Track error type for raise statement wrapping
        exception_scopes: Vec::new(),                            // Exception scope tracking stack
        argparser_tracker: argparse_transform::ArgParserTracker::new(), // Track ArgumentParser patterns
        generated_args_struct: None,                             // Args struct (hoisted to module level)
        generated_commands_enum: None,                           // Commands enum (hoisted to module level)
        current_subcommand_fields: None,                         // Subcommand field extraction
        validator_functions: HashSet::new(),                     // Track argparse validator functions
        stdlib_mappings: crate::stdlib_mappings::StdlibMappings::new(), // Stdlib API mappings
        current_func_mut_ref_params: HashSet::new(),             // Track &mut ref params in current function
        function_param_names: std::collections::HashMap::new(),  // Track function parameter names
    };

    // Must run BEFORE function conversion so validator parameter types are correct
    analyze_validators(&mut ctx, &module.functions, &module.constants);

    // All functions that can_fail return Result<T, E> and need unwrapping at call sites
    for func in &module.functions {
        if func.properties.can_fail {
            ctx.result_returning_functions.insert(func.name.clone());
        }
    }

    // Functions that can_fail and return Bool need unwrapping in boolean contexts
    for func in &module.functions {
        if func.properties.can_fail && matches!(func.ret_type, Type::Bool) {
            ctx.result_bool_functions.insert(func.name.clone());
        }
    }

    // This allows convert_call to reorder keyword arguments to match function signatures
    for func in &module.functions {
        let param_names: Vec<String> = func.params.iter().map(|p| p.name.clone()).collect();
        ctx.function_param_names.insert(func.name.clone(), param_names);
    }
    // Also track class method parameter names
    for class in &module.classes {
        for method in &class.methods {
            let method_key = format!("{}.{}", class.name, method.name);
            let param_names: Vec<String> = method.params.iter().map(|p| p.name.clone()).collect();
            ctx.function_param_names.insert(method_key, param_names.clone());
            // Also store just the method name for unqualified calls
            ctx.function_param_names.insert(method.name.clone(), param_names);
        }
    }

    // Pre-analyze all functions for parameter mutability
    // This populates function_param_muts so call sites know whether to use &mut
    pre_analyze_parameter_mutability(&mut ctx, &module.functions);

    // Pre-analyze all functions for parameter borrowing
    // This populates function_param_borrows so call sites know whether to add & or .to_string()
    pre_analyze_parameter_borrowing(&mut ctx, &module.functions);

    // Convert classes first (they might be used by functions)
    let classes = convert_classes_to_rust(&module.classes, ctx.type_mapper)?;

    // Convert all functions to detect what imports we need
    let functions = convert_functions_to_rust(&module.functions, &mut ctx)?;

    // Build items list with all generated code
    let mut items = Vec::new();

    // Add module imports (create new mapper for token generation)
    let import_mapper = crate::module_mapper::ModuleMapper::new();
    items.extend(generate_import_tokens(&module.imports, &import_mapper));

    // Add module-level constants
    items.extend(generate_constant_tokens(&module.constants, &mut ctx)?);

    // Add collection imports if needed
    items.extend(generate_conditional_imports(&ctx));

    // Both generate_import_tokens and generate_conditional_imports can add HashMap
    items = deduplicate_use_statements(items);

    // Add error type definitions if needed
    items.extend(generate_error_type_definitions(&ctx));

    // Add generated union enums
    items.extend(ctx.generated_enums.clone());

    // Add classes
    items.extend(classes);

    // (before functions so handler functions can reference Args type)
    if let Some(ref commands_enum) = ctx.generated_commands_enum {
        items.push(commands_enum.clone());
    }
    if let Some(ref args_struct) = ctx.generated_args_struct {
        items.push(args_struct.clone());
    }

    // Add all functions
    items.extend(functions);

    // Generate tests for all functions in a single test module
    // instead of one per function, which caused "the name `tests` is defined multiple times" errors
    let test_gen = crate::test_generation::TestGenerator::new(Default::default());
    if let Some(test_module) = test_gen.generate_tests_module(&module.functions)? {
        items.push(test_module);
    }

    let file = quote! {
        #(#items)*
    };

    let mut dependencies = cargo_toml_gen::extract_dependencies(&ctx);

    // Format the code first (this is when tokens become readable strings)
    let mut formatted_code = format_rust_code(file.to_string());

    // TokenStreams don't have literal strings - must scan AFTER formatting
    if formatted_code.contains("serde_json::") && !ctx.needs_serde_json {
        // Add missing import at the beginning
        formatted_code = format!("use serde_json;\n{}", formatted_code);
        // Add missing Cargo.toml dependencies
        dependencies.push(cargo_toml_gen::Dependency::new("serde_json", "1.0"));
        dependencies.push(cargo_toml_gen::Dependency::new("serde", "1.0").with_features(vec!["derive".to_string()]));
        // Re-format to ensure imports are properly ordered
        formatted_code = format_rust_code(formatted_code);
    }

    Ok((formatted_code, dependencies))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation_aware_type_mapper::AnnotationAwareTypeMapper;
    use crate::rust_gen::context::RustCodeGen;
    use crate::rust_gen::type_gen::convert_binop;
    use crate::type_mapper::TypeMapper;
    use depyler_annotations::TranspilationAnnotations;
    use std::collections::HashSet;

    fn create_test_context() -> CodeGenContext<'static> {
        // This is a bit of a hack for testing - in real use, the TypeMapper would have a longer lifetime
        let type_mapper: &'static TypeMapper = Box::leak(Box::new(TypeMapper::default()));
        CodeGenContext {
            type_mapper,
            annotation_aware_mapper: AnnotationAwareTypeMapper::with_base_mapper(type_mapper.clone()),
            string_optimizer: StringOptimizer::new(),
            union_enum_generator: crate::union_enum_gen::UnionEnumGenerator::new(),
            generated_enums: Vec::new(),
            needs_hashmap: false,
            needs_hashset: false,
            needs_vecdeque: false,
            needs_fnv_hashmap: false,
            needs_ahash_hashmap: false,
            needs_arc: false,
            needs_rc: false,
            needs_cow: false,
            needs_rand: false,
            needs_serde_json: false,
            needs_regex: false,
            needs_chrono: false,
            needs_clap: false,
            needs_csv: false,
            needs_rust_decimal: false,
            needs_num_rational: false,
            needs_base64: false,
            needs_md5: false,
            needs_sha2: false,
            needs_sha3: false,
            needs_blake2: false,
            needs_hex: false,
            needs_uuid: false,
            needs_hmac: false,
            needs_crc32: false,
            needs_url_encoding: false,
            declared_vars: vec![HashSet::new()],
            current_function_can_fail: false,
            current_return_type: None,
            module_mapper: crate::module_mapper::ModuleMapper::new(),
            imported_modules: std::collections::HashMap::new(),
            imported_items: std::collections::HashMap::new(),
            mutable_vars: HashSet::new(),
            needs_zerodivisionerror: false,
            needs_indexerror: false,
            needs_valueerror: false,
            needs_argumenttypeerror: false,
            is_classmethod: false,
            in_generator: false,
            generator_state_vars: HashSet::new(),
            var_types: std::collections::HashMap::new(),
            class_names: HashSet::new(),
            mutating_methods: std::collections::HashMap::new(),
            function_return_types: std::collections::HashMap::new(), // Track function return types
            function_param_borrows: std::collections::HashMap::new(), // Track parameter borrowing
            function_param_muts: std::collections::HashMap::new(),   // Track parameters needing &mut
            tuple_iter_vars: HashSet::new(),                         // Track tuple iteration variables
            is_final_statement: false, // Track final statement for expression-based returns
            result_bool_functions: HashSet::new(), // Track functions returning Result<bool>
            result_returning_functions: HashSet::new(), // Track ALL Result-returning functions
            current_error_type: None,  // Track error type for raise statement wrapping
            exception_scopes: Vec::new(), // Exception scope tracking stack
            argparser_tracker: argparse_transform::ArgParserTracker::new(), // Track ArgumentParser patterns
            generated_args_struct: None, // Args struct (hoisted to module level)
            generated_commands_enum: None, // Commands enum (hoisted to module level)
            current_subcommand_fields: None, // Subcommand field extraction
            validator_functions: HashSet::new(), // Track argparse validator functions
            stdlib_mappings: crate::stdlib_mappings::StdlibMappings::new(),
            current_func_mut_ref_params: HashSet::new(), // Track &mut ref params in current function
            function_param_names: std::collections::HashMap::new(), // Track function parameter names
        }
    }

    #[test]
    fn test_simple_function_generation() {
        let func = HirFunction {
            name: "add".to_string(),
            params: vec![
                HirParam::new("a".to_string(), Type::Int),
                HirParam::new("b".to_string(), Type::Int),
            ]
            .into(),
            ret_type: Type::Int,
            body: vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let tokens = func.to_rust_tokens(&mut ctx).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("pub fn add"));
        assert!(code.contains("i32"));
        // The function body should contain the expression result without explicit `return`
        assert!(code.contains("a + b"), "Function should contain expression 'a + b'");
    }

    #[test]
    fn test_control_flow_generation() {
        let if_stmt = HirStmt::If {
            condition: HirExpr::Binary {
                op: BinOp::Gt,
                left: Box::new(HirExpr::Var("x".to_string())),
                right: Box::new(HirExpr::Literal(Literal::Int(0))),
            },
            then_body: vec![HirStmt::Return(Some(HirExpr::Literal(Literal::String(
                "positive".to_string(),
            ))))],
            else_body: Some(vec![HirStmt::Return(Some(HirExpr::Literal(Literal::String(
                "negative".to_string(),
            ))))]),
        };

        let mut ctx = create_test_context();
        let tokens = if_stmt.to_rust_tokens(&mut ctx).unwrap();
        let code = tokens.to_string();

        assert!(code.contains("if"));
        assert!(code.contains("else"));
        assert!(code.contains("return"));
    }

    #[test]
    fn test_list_generation() {
        // Test literal array generation
        let list_expr = HirExpr::List(vec![
            HirExpr::Literal(Literal::Int(1)),
            HirExpr::Literal(Literal::Int(2)),
            HirExpr::Literal(Literal::Int(3)),
        ]);

        let mut ctx = create_test_context();
        let expr = list_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #expr }.to_string();

        // Small literal lists should generate arrays
        assert!(code.contains("[") && code.contains("]"));
        assert!(code.contains("1"));
        assert!(code.contains("2"));
        assert!(code.contains("3"));

        // Test non-literal list still uses vec!
        let var_list = HirExpr::List(vec![HirExpr::Var("x".to_string()), HirExpr::Var("y".to_string())]);

        let expr2 = var_list.to_rust_expr(&mut ctx).unwrap();
        let code2 = quote! { #expr2 }.to_string();
        assert!(code2.contains("vec !"));
    }

    #[test]
    fn test_dict_generation_sets_needs_hashmap() {
        let dict_expr = HirExpr::Dict(vec![(
            HirExpr::Literal(Literal::String("key".to_string())),
            HirExpr::Literal(Literal::Int(42)),
        )]);

        let mut ctx = create_test_context();
        assert!(!ctx.needs_hashmap);

        let _ = dict_expr.to_rust_expr(&mut ctx).unwrap();

        assert!(ctx.needs_hashmap);
    }

    #[test]
    fn test_binary_operations() {
        let ops = vec![
            (BinOp::Add, "+"),
            (BinOp::Sub, "-"),
            (BinOp::Mul, "*"),
            (BinOp::Eq, "=="),
            (BinOp::Lt, "<"),
        ];

        for (op, expected) in ops {
            let result = convert_binop(op).unwrap();
            assert_eq!(quote! { #result }.to_string(), expected);
        }
    }

    #[test]
    fn test_unsupported_operators() {
        assert!(convert_binop(BinOp::Pow).is_err());
        assert!(convert_binop(BinOp::In).is_err());
        assert!(convert_binop(BinOp::NotIn).is_err());
    }

    // ========================================================================
    // ========================================================================

    #[test]
    fn test_codegen_pass_stmt() {
        let result = codegen_pass_stmt().unwrap();
        assert!(result.is_empty(), "Pass statement should generate no code");
    }

    #[test]
    fn test_codegen_break_stmt_simple() {
        let result = codegen_break_stmt(&None).unwrap();
        assert_eq!(result.to_string(), "break ;");
    }

    #[test]
    fn test_codegen_break_stmt_with_label() {
        let result = codegen_break_stmt(&Some("outer".to_string())).unwrap();
        assert_eq!(result.to_string(), "break 'outer ;");
    }

    #[test]
    fn test_codegen_continue_stmt_simple() {
        let result = codegen_continue_stmt(&None).unwrap();
        assert_eq!(result.to_string(), "continue ;");
    }

    #[test]
    fn test_codegen_continue_stmt_with_label() {
        let result = codegen_continue_stmt(&Some("outer".to_string())).unwrap();
        assert_eq!(result.to_string(), "continue 'outer ;");
    }

    #[test]
    fn test_codegen_expr_stmt() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let expr = HirExpr::Literal(Literal::Int(42));

        let result = codegen_expr_stmt(&expr, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "42 ;");
    }

    // ========================================================================
    // ========================================================================

    #[test]
    fn test_codegen_return_stmt_simple() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let expr = Some(HirExpr::Literal(Literal::Int(42)));

        let result = codegen_return_stmt(&expr, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "return 42 ;");
    }

    #[test]
    fn test_codegen_return_stmt_none() {
        let mut ctx = create_test_context();

        let result = codegen_return_stmt(&None, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "return ;");
    }

    #[test]
    fn test_codegen_while_stmt() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let condition = HirExpr::Literal(Literal::Bool(true));
        let body = vec![HirStmt::Pass];

        let result = codegen_while_stmt(&condition, &body, &mut ctx).unwrap();
        assert!(result.to_string().contains("while true"));
    }

    #[test]
    fn test_codegen_raise_stmt_with_exception() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        ctx.current_function_can_fail = true; // Function returns Result, so raise becomes return Err
        let exc = Some(HirExpr::Literal(Literal::String("Error".to_string())));

        let result = codegen_raise_stmt(&exc, &mut ctx).unwrap();
        // String literals are now &str, which is valid for error contexts
        // The Error type will handle the conversion
        assert_eq!(result.to_string(), "return Err (\"Error\") ;");
    }

    #[test]
    fn test_codegen_raise_stmt_bare() {
        let mut ctx = create_test_context();
        ctx.current_function_can_fail = true; // Function returns Result, so raise becomes return Err

        let result = codegen_raise_stmt(&None, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "return Err (\"Exception raised\" . into ()) ;");
    }

    // NOTE: With statement with target incomplete - requires full implementation ()
    // This test was written ahead of implementation (aspirational test)
    // Tracked in roadmap: Complete with statement target binding support
    #[test]
    #[ignore = "Incomplete feature: With statement target binding not yet implemented"]
    fn test_codegen_with_stmt_with_target() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let context = HirExpr::Literal(Literal::Int(42));
        let target = Some("file".to_string());
        let body = vec![HirStmt::Pass];

        let result = codegen_with_stmt(&context, &target, &body, &mut ctx).unwrap();
        assert!(result.to_string().contains("let mut file"));
    }

    #[test]
    fn test_codegen_with_stmt_no_target() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let context = HirExpr::Literal(Literal::Int(42));
        let body = vec![HirStmt::Pass];

        let result = codegen_with_stmt(&context, &None, &body, &mut ctx).unwrap();
        assert!(result.to_string().contains("let _context"));
    }

    // Phase 3b tests - Assign handler tests
    #[test]
    fn test_codegen_assign_symbol_new_var() {
        let mut ctx = create_test_context();
        let value_expr = syn::parse_quote! { 42 };

        let result = codegen_assign_symbol("x", value_expr, None, false, &mut ctx).unwrap();
        assert!(result.to_string().contains("let x = 42"));
    }

    #[test]
    fn test_codegen_assign_symbol_with_type() {
        let mut ctx = create_test_context();
        let value_expr = syn::parse_quote! { 42 };
        let type_ann = Some(quote! { : i32 });

        let result = codegen_assign_symbol("x", value_expr, type_ann, false, &mut ctx).unwrap();
        assert!(result.to_string().contains("let x : i32 = 42"));
    }

    #[test]
    fn test_codegen_assign_symbol_existing_var() {
        let mut ctx = create_test_context();
        ctx.declare_var("x");
        let value_expr = syn::parse_quote! { 100 };

        let result = codegen_assign_symbol("x", value_expr, None, false, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "x = 100 ;");
    }

    #[test]
    fn test_codegen_assign_index() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        let base = HirExpr::Var("dict".to_string());
        let index = HirExpr::Literal(Literal::String("key".to_string()));
        let value_expr = syn::parse_quote! { 42 };

        let result = codegen_assign_index(&base, &index, value_expr, &mut ctx).unwrap();
        assert!(result.to_string().contains("dict . insert"));
    }

    #[test]
    fn test_codegen_assign_attribute() {
        let mut ctx = create_test_context();
        let base = HirExpr::Var("obj".to_string());
        let value_expr = syn::parse_quote! { 42 };

        let result = codegen_assign_attribute(&base, "field", value_expr, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "obj . field = 42 ;");
    }

    #[test]
    fn test_codegen_assign_tuple_new_vars() {
        use crate::hir::AssignTarget;

        let mut ctx = create_test_context();
        let targets = vec![
            AssignTarget::Symbol("a".to_string()),
            AssignTarget::Symbol("b".to_string()),
        ];
        let value_expr = syn::parse_quote! { (1, 2) };

        let result = codegen_assign_tuple(&targets, value_expr, None, &mut ctx).unwrap();
        assert!(result.to_string().contains("let (a , b) = (1 , 2)"));
    }

    // Phase 3b tests - Try handler tests
    #[test]
    fn test_codegen_try_stmt_simple() {
        use crate::hir::ExceptHandler;

        let mut ctx = create_test_context();
        let body = vec![HirStmt::Pass];
        let handlers = vec![ExceptHandler {
            exception_type: None,
            name: None,
            body: vec![HirStmt::Pass],
        }];

        let result = codegen_try_stmt(&body, &handlers, &None, &mut ctx).unwrap();
        let result_str = result.to_string();
        // Just executes try block statements directly
        assert!(!result_str.is_empty(), "Should generate code");
        // Code should be simple block execution (no complex patterns for now)
    }

    #[test]
    fn test_codegen_try_stmt_with_finally() {
        let mut ctx = create_test_context();
        let body = vec![HirStmt::Pass];
        let handlers = vec![];
        let finally = Some(vec![HirStmt::Pass]);

        let result = codegen_try_stmt(&body, &handlers, &finally, &mut ctx).unwrap();
        assert!(!result.to_string().is_empty());
    }

    #[test]
    fn test_codegen_try_stmt_except_and_finally() {
        use crate::hir::ExceptHandler;

        let mut ctx = create_test_context();
        let body = vec![HirStmt::Pass];
        let handlers = vec![ExceptHandler {
            exception_type: None,
            name: Some("e".to_string()),
            body: vec![HirStmt::Pass],
        }];
        let finally = Some(vec![HirStmt::Pass]);

        let result = codegen_try_stmt(&body, &handlers, &finally, &mut ctx).unwrap();
        let result_str = result.to_string();
        // Executes try block then finally block
        assert!(!result_str.is_empty(), "Should generate code");
        // Code should execute try block and finally block
    }

    // Phase 1b/1c tests - Type conversion functions
    #[test]
    fn test_int_cast_conversion() {
        // Previous behavior (no cast) caused "cannot add bool to bool" errors
        // when x is a bool variable: int(flag1) + int(flag2) → flag1 + flag2 (ERROR!)
        let call_expr = HirExpr::Call {
            func: "int".to_string(),
            args: vec![HirExpr::Var("x".to_string())],
            kwargs: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        // Should generate cast for variables to prevent bool arithmetic errors
        assert!(code.contains("x"), "Expected 'x', got: {}", code);
        assert!(code.contains("as i32"), "Should contain 'as i32' cast, got: {}", code);
    }

    #[test]
    fn test_float_cast_conversion() {
        // Python: float(x) → Rust: (x) as f64
        let call_expr = HirExpr::Call {
            func: "float".to_string(),
            args: vec![HirExpr::Var("y".to_string())],
            kwargs: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        assert!(code.contains("as f64"), "Expected '(y) as f64', got: {}", code);
    }

    #[test]
    fn test_str_conversion() {
        // Python: str(x) → Rust: x.to_string()
        let call_expr = HirExpr::Call {
            func: "str".to_string(),
            args: vec![HirExpr::Var("value".to_string())],
            kwargs: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        assert!(
            code.contains("to_string"),
            "Expected 'value.to_string()', got: {}",
            code
        );
    }

    // NOTE: Boolean casting incomplete - requires type cast implementation ()
    // This test was written ahead of implementation (aspirational test)
    // Tracked in roadmap: Implement bool() builtin casting
    #[test]
    #[ignore = "Incomplete feature: bool() casting not yet implemented"]
    fn test_bool_cast_conversion() {
        // Python: bool(x) → Rust: (x) as bool
        let call_expr = HirExpr::Call {
            func: "bool".to_string(),
            args: vec![HirExpr::Var("flag".to_string())],
            kwargs: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        assert!(code.contains("as bool"), "Expected '(flag) as bool', got: {}", code);
    }

    #[test]
    fn test_int_cast_with_expression() {
        // Previous behavior (no cast) caused "cannot add bool to bool" errors
        // when expression might be bool: int(x > 0) + int(y > 0) → (x > 0) + (y > 0) (ERROR!)
        let division = HirExpr::Binary {
            op: BinOp::Div,
            left: Box::new(HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Var("low".to_string())),
                right: Box::new(HirExpr::Var("high".to_string())),
            }),
            right: Box::new(HirExpr::Literal(Literal::Int(2))),
        };

        let call_expr = HirExpr::Call {
            func: "int".to_string(),
            args: vec![division],
            kwargs: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        // Should generate cast for expressions to prevent bool arithmetic errors
        assert!(code.contains("low"), "Expected 'low' variable, got: {}", code);
        assert!(code.contains("high"), "Expected 'high' variable, got: {}", code);
        assert!(code.contains("as i32"), "Should contain 'as i32' cast, got: {}", code);
    }

    #[test]
    fn test_float_literal_decimal_point() {
        // Regression test for DEPYLER-TBD: Ensure float literals always have decimal point
        // Bug: f64::to_string() for 0.0 produces "0" (no decimal), parsed as integer
        // Fix: Always ensure ".0" suffix for floats without decimal/exponent
        let mut ctx = create_test_context();

        // Test 0.0 → should generate "0.0" not "0"
        let zero_float = HirExpr::Literal(Literal::Float(0.0));
        let result = zero_float.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();
        assert!(
            code.contains("0.0") || code.contains("0 ."),
            "Expected '0.0' for float zero, got: {}",
            code
        );

        // Test 42.0 → should generate "42.0" not "42"
        let forty_two = HirExpr::Literal(Literal::Float(42.0));
        let result = forty_two.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();
        assert!(
            code.contains("42.0") || code.contains("42 ."),
            "Expected '42.0' for float, got: {}",
            code
        );

        // Test 1.5 → should preserve "1.5" (already has decimal)
        let one_half = HirExpr::Literal(Literal::Float(1.5));
        let result = one_half.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();
        assert!(code.contains("1.5"), "Expected '1.5', got: {}", code);

        // Test scientific notation: 1e10 → should preserve (has 'e')
        let scientific = HirExpr::Literal(Literal::Float(1e10));
        let result = scientific.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();
        assert!(
            code.contains("e") || code.contains("E") || code.contains("."),
            "Expected scientific notation or decimal, got: {}",
            code
        );
    }

    #[test]
    fn test_string_method_return_types() {
        // Regression test for v3.16.0 Phase 1
        // String transformation methods (.upper(), .lower(), .strip()) return owned String
        // Function signatures should reflect this: `fn f(s: &str) -> String` not `-> &str`

        // Test 1: .upper() should generate String return type
        let upper_func = HirFunction {
            name: "to_upper".to_string(),
            params: vec![HirParam::new("text".to_string(), Type::String)].into(),
            ret_type: Type::String,
            body: vec![HirStmt::Return(Some(HirExpr::MethodCall {
                object: Box::new(HirExpr::Var("text".to_string())),
                method: "upper".to_string(),
                args: vec![],
                kwargs: vec![],
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = upper_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        // Should generate: fn to_upper(text: &str) -> String
        // NOT: fn to_upper<'a>(text: &'a str) -> &'a str
        assert!(
            code.contains("-> String"),
            "Expected '-> String' for .upper() method, got: {}",
            code
        );
        assert!(
            !code.contains("-> & ") && !code.contains("-> &'"),
            "Should not generate borrowed return for .upper(), got: {}",
            code
        );

        // Test 2: .lower() should also generate String return type
        let lower_func = HirFunction {
            name: "to_lower".to_string(),
            params: vec![HirParam::new("text".to_string(), Type::String)].into(),
            ret_type: Type::String,
            body: vec![HirStmt::Return(Some(HirExpr::MethodCall {
                object: Box::new(HirExpr::Var("text".to_string())),
                method: "lower".to_string(),
                args: vec![],
                kwargs: vec![],
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = lower_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        assert!(
            code.contains("-> String"),
            "Expected '-> String' for .lower() method, got: {}",
            code
        );

        // Test 3: .strip() should also generate String return type
        let strip_func = HirFunction {
            name: "trim_text".to_string(),
            params: vec![HirParam::new("text".to_string(), Type::String)].into(),
            ret_type: Type::String,
            body: vec![HirStmt::Return(Some(HirExpr::MethodCall {
                object: Box::new(HirExpr::Var("text".to_string())),
                method: "strip".to_string(),
                args: vec![],
                kwargs: vec![],
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = strip_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        assert!(
            code.contains("-> String"),
            "Expected '-> String' for .strip() method, got: {}",
            code
        );
    }

    #[test]
    fn test_int_float_division_semantics() {
        // Regression test for v3.16.0 Phase 2
        // Python's `/` operator always returns float, even with int operands
        // Rust's `/` does integer division with int operands
        // We need to cast to float when the context expects float

        // Test 1: int / int returning float (the main bug)
        let divide_func = HirFunction {
            name: "safe_divide".to_string(),
            params: vec![
                HirParam::new("a".to_string(), Type::Int),
                HirParam::new("b".to_string(), Type::Int),
            ]
            .into(),
            ret_type: Type::Float, // Expects float return!
            body: vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::Div,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = divide_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        // Should generate: (a as f64) / (b as f64)
        // NOT: a / b (which would do integer division)
        assert!(
            code.contains("as f64") || code.contains("as f32"),
            "Expected float cast for int/int division with float return, got: {}",
            code
        );
        assert!(
            code.contains("-> f64") || code.contains("-> f32"),
            "Expected float return type, got: {}",
            code
        );

        // Test 2: int // int returning int (floor division - should NOT cast)
        let floor_div_func = HirFunction {
            name: "floor_divide".to_string(),
            params: vec![
                HirParam::new("a".to_string(), Type::Int),
                HirParam::new("b".to_string(), Type::Int),
            ]
            .into(),
            ret_type: Type::Int, // Expects int return
            body: vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::FloorDiv,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = floor_div_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        // Floor division should NOT add float casts
        assert!(
            code.contains("-> i32") || code.contains("-> i64"),
            "Expected int return type for floor division, got: {}",
            code
        );

        // Test 3: float / float should work without changes
        let float_div_func = HirFunction {
            name: "divide_floats".to_string(),
            params: vec![
                HirParam::new("a".to_string(), Type::Float),
                HirParam::new("b".to_string(), Type::Float),
            ]
            .into(),
            ret_type: Type::Float,
            body: vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::Div,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        };

        let mut ctx = create_test_context();
        let result = float_div_func.to_rust_tokens(&mut ctx).unwrap();
        let code = result.to_string();

        assert!(
            code.contains("-> f64") || code.contains("-> f32"),
            "Expected float return type, got: {}",
            code
        );
    }
}
