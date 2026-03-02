use crate::annotation_aware_type_mapper::AnnotationAwareTypeMapper;
use crate::borrowing_context::BorrowingStrategy;
use crate::cargo_toml_gen; // DEPYLER-0384: Cargo.toml generation
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
pub mod keywords; // DEPYLER-0023: Centralized keyword escaping
mod stmt_gen;
mod type_gen;

// Internal imports
use error_gen::generate_error_type_definitions;
use format::format_rust_code;
use import_gen::process_module_imports;
#[cfg(test)]
use stmt_gen::{
    assign::{
        codegen_assign_attribute, codegen_assign_index, codegen_assign_symbol, codegen_assign_tuple,
    },
    context_mgr::codegen_with_stmt,
    control_flow::{codegen_break_stmt, codegen_continue_stmt, codegen_while_stmt},
    exception::{codegen_raise_stmt, codegen_try_stmt},
    return_stmt::codegen_return_stmt,
    simple::{codegen_expr_stmt, codegen_pass_stmt},
};

// Public re-exports for external modules (union_enum_gen, etc.)
pub use argparse_transform::ArgParserTracker; // DEPYLER-0384: Export for testing
pub use context::{CodeGenContext, RustCodeGen, ToRustExpr};
pub use type_gen::rust_type_to_syn;

// Internal re-exports for cross-module access
pub(crate) use func_gen::return_type_expects_float;

/// Analyze functions for string optimization
/// Check if a field type is Copy (for determining struct Copy derivation).
fn is_field_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::None => true,
        Type::Optional(inner) | Type::Final(inner) => is_field_copy_type(inner),
        Type::Tuple(elements) => elements.iter().all(|t| is_field_copy_type(t)),
        Type::Array { element_type, .. } => is_field_copy_type(element_type),
        _ => false,
    }
}

///
/// Performs string optimization analysis on all functions.
/// Complexity: 2 (well within ≤10 target)
fn analyze_string_optimization(ctx: &mut CodeGenContext, functions: &[HirFunction]) {
    for func in functions {
        ctx.string_optimizer.analyze_function(func);
    }
}

/// Pre-populate function_param_borrows for all functions
///
/// This two-pass approach ensures that when function A calls function B,
/// A knows B's parameter signature (borrowed vs owned, mutable vs immutable)
/// even if B hasn't been fully generated yet.
///
/// Without this, call sites default to immutable borrow, causing incorrect
/// code generation when the callee expects mutable borrow or ownership.
///
/// Performs interprocedural analysis via fixpoint iteration to propagate
/// mutability requirements: if function A passes parameter X to function B
/// which expects &mut, then A's parameter X must also be &mut.
fn populate_function_param_borrows(
    functions: &[HirFunction],
    ctx: &mut CodeGenContext,
) -> Result<()> {
    use crate::lifetime_analysis::LifetimeInference;

    // Phase 1: Initial analysis - determine direct mutations
    for func in functions {
        // Perform lifetime analysis to determine parameter borrowing strategies
        let mut lifetime_inference = LifetimeInference::new();
        let lifetime_result = lifetime_inference
            .apply_elision_rules_with_interprocedural(
                func,
                ctx.type_mapper,
                ctx.interprocedural_analysis,
            )
            .unwrap_or_else(|| {
                lifetime_inference.analyze_function_with_interprocedural(
                    func,
                    ctx.type_mapper,
                    ctx.interprocedural_analysis,
                )
            });

        // Extract parameter borrowing information
        let mut param_borrows = Vec::new();
        let mut param_strategies = Vec::new();

        for param in &func.params {
            let inferred = lifetime_result
                .param_lifetimes
                .get(&param.name)
                .map(|inf| (inf.should_borrow, inf.needs_mut))
                .unwrap_or((false, false));

            let strategy = lifetime_result
                .borrowing_strategies
                .get(&param.name)
                .cloned()
                .unwrap_or(BorrowingStrategy::TakeOwnership);

            param_borrows.push(context::ParamBorrowInfo {
                should_borrow: inferred.0,
                needs_mut: inferred.1,
                takes_ownership: matches!(strategy, BorrowingStrategy::TakeOwnership),
            });
            param_strategies.push(strategy);
        }

        ctx.function_param_borrows
            .insert(func.name.clone(), param_borrows);
        ctx.function_param_strategies
            .insert(func.name.clone(), param_strategies);
    }

    // Phase 2: Interprocedural analysis - propagate mutability requirements
    // Fixpoint iteration: if function A passes parameter P to function B where B expects &mut,
    // then A must also take P as &mut
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10; // Prevent infinite loops

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        for func in functions {
            // Find all function calls in this function's body
            let calls = find_function_calls(&func.body);

            for (called_func_name, arg_exprs) in calls {
                // Clone the callee's parameter requirements to avoid borrowing conflict
                let callee_borrows = ctx.function_param_borrows.get(&called_func_name).cloned();

                if let Some(callee_borrows) = callee_borrows {
                    // Check each argument
                    for (arg_idx, arg_expr) in arg_exprs.iter().enumerate() {
                        // If this argument is a variable that matches a parameter of the current function
                        if let HirExpr::Var(var_name) = arg_expr {
                            // Check if this variable is a parameter of the current function
                            if let Some(param_idx) =
                                func.params.iter().position(|p| &p.name == var_name)
                            {
                                // Get the current parameter's borrow info
                                let current_caller_info = ctx
                                    .function_param_borrows
                                    .get(&func.name)
                                    .and_then(|borrows| borrows.get(param_idx))
                                    .cloned();

                                // Check if the callee expects &mut for this argument position
                                if let Some(callee_info) = callee_borrows.get(arg_idx) {
                                    // Key insight: If a parameter is passed to a function call,
                                    // it CANNOT take ownership (would be moved). Must be borrowed.
                                    // The borrowing must be &mut if:
                                    // - The callee needs &mut, OR
                                    // - The parameter has its fields mutated locally

                                    let currently_takes_ownership = current_caller_info
                                        .as_ref()
                                        .map(|info| info.takes_ownership)
                                        .unwrap_or(false);

                                    // Check if this parameter actually has field mutations
                                    let param_has_mutations =
                                        parameter_has_field_mutations(func, var_name);

                                    // If parameter takes ownership, it's being passed to a function
                                    // so it must be borrowed instead
                                    let should_upgrade_to_borrow = currently_takes_ownership;
                                    let should_be_mut =
                                        param_has_mutations || callee_info.needs_mut;

                                    if should_upgrade_to_borrow {
                                        // Skip borrowing upgrade for Copy types (i32, f64, bool, etc.)
                                        // Copy types are cheap to pass by value — no need for references
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if is_copy_rust_type(&rust_type) {
                                            // Copy type: leave as TakeOwnership (pass by value)
                                        } else if let Some(caller_borrows) =
                                            ctx.function_param_borrows.get_mut(&func.name)
                                        {
                                            if let Some(caller_info) =
                                                caller_borrows.get_mut(param_idx)
                                            {
                                                if caller_info.takes_ownership {
                                                    // Upgrade from ownership to borrowing
                                                    caller_info.should_borrow = true;
                                                    caller_info.takes_ownership = false;
                                                    caller_info.needs_mut = should_be_mut;
                                                    changed = true;
                                                }
                                            }
                                        }
                                    } else if callee_info.needs_mut {
                                        // Skip for Copy types — they don't need &mut
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if !is_copy_rust_type(&rust_type) {
                                            // Already borrowed, but need to upgrade to &mut
                                            if let Some(caller_borrows) =
                                                ctx.function_param_borrows.get_mut(&func.name)
                                            {
                                                if let Some(caller_info) =
                                                    caller_borrows.get_mut(param_idx)
                                                {
                                                    if !caller_info.needs_mut {
                                                        caller_info.needs_mut = true;
                                                        changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    } else if callee_info.should_borrow
                                        && !callee_info.takes_ownership
                                    {
                                        // Skip for Copy types — they don't need borrowing
                                        let param_type = &func.params[param_idx].ty;
                                        let rust_type = ctx.type_mapper.map_type(param_type);
                                        if !is_copy_rust_type(&rust_type) {
                                            // The callee needs at least &, ensure we provide a borrow
                                            if let Some(caller_borrows) =
                                                ctx.function_param_borrows.get_mut(&func.name)
                                            {
                                                if let Some(caller_info) =
                                                    caller_borrows.get_mut(param_idx)
                                                {
                                                    if !caller_info.should_borrow {
                                                        caller_info.should_borrow = true;
                                                        caller_info.takes_ownership = false;
                                                        changed = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a RustType is Copy (primitives should not be borrowed).
fn is_copy_rust_type(rust_type: &crate::type_mapper::RustType) -> bool {
    use crate::type_mapper::RustType;
    match rust_type {
        RustType::Primitive(_) | RustType::Unit => true,
        RustType::Tuple(types) => types.iter().all(is_copy_rust_type),
        _ => false,
    }
}

/// Helper: Check if a parameter is mutated in the function body
/// Detects: field assignments, method calls that mutate, index assignments
fn parameter_has_field_mutations(func: &HirFunction, param_name: &str) -> bool {
    fn check_stmts(stmts: &[HirStmt], param_name: &str) -> bool {
        stmts.iter().any(|stmt| check_stmt(stmt, param_name))
    }

    fn check_stmt(stmt: &HirStmt, param_name: &str) -> bool {
        match stmt {
            HirStmt::Assign { target, .. } => {
                // Check if assigning to a field or index of the parameter
                match target {
                    AssignTarget::Attribute { value, .. } => {
                        matches!(value.as_ref(), HirExpr::Var(name) if name == param_name)
                    }
                    AssignTarget::Index { base, .. } => {
                        matches!(base.as_ref(), HirExpr::Var(name) if name == param_name)
                    }
                    _ => false,
                }
            }
            HirStmt::Expr(expr) => check_expr_for_mutation(expr, param_name),
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                check_stmts(then_body, param_name)
                    || else_body
                        .as_ref()
                        .map(|body| check_stmts(body, param_name))
                        .unwrap_or(false)
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                check_stmts(body, param_name)
            }
            _ => false,
        }
    }

    fn check_expr_for_mutation(expr: &HirExpr, param_name: &str) -> bool {
        match expr {
            // Method calls on the parameter that are known to mutate
            HirExpr::MethodCall { object, method, .. } => {
                if let HirExpr::Var(obj_name) = object.as_ref() {
                    if obj_name == param_name {
                        // List of mutating methods
                        return matches!(
                            method.as_str(),
                            "append"
                                | "extend"
                                | "insert"
                                | "remove"
                                | "pop"
                                | "clear"
                                | "sort"
                                | "reverse"
                                | "update"
                                | "add"
                                | "discard"
                                | "setdefault"
                                | "popitem"
                        );
                    }
                }
                false
            }
            // Recursively check nested expressions
            HirExpr::Binary { left, right, .. } => {
                check_expr_for_mutation(left, param_name)
                    || check_expr_for_mutation(right, param_name)
            }
            HirExpr::Unary { operand, .. } => check_expr_for_mutation(operand, param_name),
            HirExpr::Call { args, .. } => args
                .iter()
                .any(|arg| check_expr_for_mutation(arg, param_name)),
            _ => false,
        }
    }

    check_stmts(&func.body, param_name)
}

/// Helper: Find all function calls in a list of statements
/// Returns a list of (function_name, arguments) tuples
fn find_function_calls(stmts: &[HirStmt]) -> Vec<(String, Vec<HirExpr>)> {
    let mut calls = Vec::new();

    fn scan_expr(expr: &HirExpr, calls: &mut Vec<(String, Vec<HirExpr>)>) {
        match expr {
            HirExpr::Call { func, args, .. } => {
                calls.push((func.clone(), args.clone()));
                // Recursively scan arguments
                for arg in args {
                    scan_expr(arg, calls);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                scan_expr(left, calls);
                scan_expr(right, calls);
            }
            HirExpr::Unary { operand, .. } => {
                scan_expr(operand, calls);
            }
            HirExpr::MethodCall { object, args, .. } => {
                scan_expr(object, calls);
                for arg in args {
                    scan_expr(arg, calls);
                }
            }
            HirExpr::Attribute { value, .. } => {
                scan_expr(value, calls);
            }
            HirExpr::Index { base, index } => {
                scan_expr(base, calls);
                scan_expr(index, calls);
            }
            HirExpr::IfExpr { test, body, orelse } => {
                scan_expr(test, calls);
                scan_expr(body, calls);
                scan_expr(orelse, calls);
            }
            HirExpr::List(exprs) | HirExpr::Tuple(exprs) | HirExpr::Set(exprs) => {
                for e in exprs {
                    scan_expr(e, calls);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    scan_expr(k, calls);
                    scan_expr(v, calls);
                }
            }
            HirExpr::Lambda { body, .. } => {
                scan_expr(body, calls);
            }
            HirExpr::ListComp { element, .. } | HirExpr::SetComp { element, .. } => {
                scan_expr(element, calls);
            }
            HirExpr::DictComp { key, value, .. } => {
                scan_expr(key, calls);
                scan_expr(value, calls);
            }
            _ => {}
        }
    }

    fn scan_stmt(stmt: &HirStmt, calls: &mut Vec<(String, Vec<HirExpr>)>) {
        match stmt {
            HirStmt::Expr(expr) => scan_expr(expr, calls),
            HirStmt::Assign { value, .. } => scan_expr(value, calls),
            HirStmt::Return(Some(expr)) => scan_expr(expr, calls),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                scan_expr(condition, calls);
                for s in then_body {
                    scan_stmt(s, calls);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        scan_stmt(s, calls);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                scan_expr(condition, calls);
                for s in body {
                    scan_stmt(s, calls);
                }
            }
            HirStmt::For { iter, body, .. } => {
                scan_expr(iter, calls);
                for s in body {
                    scan_stmt(s, calls);
                }
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                for s in body {
                    scan_stmt(s, calls);
                }
                for handler in handlers {
                    for s in &handler.body {
                        scan_stmt(s, calls);
                    }
                }
                if let Some(else_stmts) = orelse {
                    for s in else_stmts {
                        scan_stmt(s, calls);
                    }
                }
                if let Some(final_stmts) = finalbody {
                    for s in final_stmts {
                        scan_stmt(s, calls);
                    }
                }
            }
            _ => {}
        }
    }

    for stmt in stmts {
        scan_stmt(stmt, &mut calls);
    }

    calls
}

/// DEPYLER-0447: Analyze function bodies AND constants to find argparse validators
///
/// Scans all statements in function bodies and constant expressions to find
/// add_argument(type=validator_func) calls. Populates ctx.validator_functions
/// with function names used as type= parameters.
/// This must run BEFORE function signature generation so parameter types can be corrected.
///
/// Complexity: 8 (func loop + const loop + stmt loop + match + expr match + kwargs loop + filter)
fn analyze_validators(
    ctx: &mut CodeGenContext,
    functions: &[HirFunction],
    constants: &[HirConstant],
) {
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
                then_body,
                else_body,
                ..
            } => {
                scan_stmts_for_validators(then_body, ctx);
                if let Some(else_stmts) = else_body {
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
                if let Some(else_stmts) = orelse {
                    scan_stmts_for_validators(else_stmts, ctx);
                }
                if let Some(final_stmts) = finalbody {
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

/// Analyze which variables are reassigned (mutated) in a list of statements
///
/// Populates ctx.mutable_vars with variables that are:
/// 1. Reassigned after declaration (x = 1; x = 2)
/// 2. Mutated via method calls (.push(), .extend(), .insert(), .remove(), .pop(), etc.)
/// 3. DEPYLER-0312: Function parameters that are reassigned (requires mut)
///
/// Complexity: 7 (stmt loop + match + if + expr scan + method match)
fn analyze_mutable_vars(stmts: &[HirStmt], ctx: &mut CodeGenContext, params: &[HirParam]) {
    let mut declared = HashSet::new();
    let mut loop_var_origins: HashMap<String, String> = HashMap::new();

    // DEPYLER-0312: Pre-populate declared with function parameters
    // This allows the reassignment detection logic below to catch parameter mutations
    // Example: def gcd(a, b): a = temp  # Now detected as reassignment → mut a
    for param in params {
        declared.insert(param.name.clone());
    }

    fn extract_param_from_expr(expr: &HirExpr, declared: &HashSet<String>) -> Option<String> {
        match expr {
            HirExpr::Var(name) => {
                if declared.contains(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExpr::Attribute { value, .. } => extract_param_from_expr(value, declared),
            HirExpr::Index { base, .. } => extract_param_from_expr(base, declared),
            _ => None,
        }
    }

    fn analyze_expr_for_mutations(
        expr: &HirExpr,
        mutable: &mut HashSet<String>,
        var_types: &HashMap<String, String>,
        mutating_methods: &HashMap<String, HashSet<String>>,
    ) {
        match expr {
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
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
            HirExpr::List(items)
            | HirExpr::Tuple(items)
            | HirExpr::Set(items)
            | HirExpr::FrozenSet(items) => {
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
        loop_var_origins: &mut HashMap<String, String>,
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
                    AssignTarget::Attribute { value: obj, .. } => {
                        // DEPYLER-0235 FIX: Property writes require the base object to be mutable
                        // e.g., `b.size = 20` requires `let mut b = ...`
                        // Also check if this is a loop variable that traces back to a parameter
                        if let HirExpr::Var(var_name) = obj.as_ref() {
                            if let Some(param_name) = loop_var_origins.get(var_name) {
                                // Mutating through a loop variable - mark the source parameter as mutable
                                mutable.insert(param_name.clone());
                            } else {
                                // Direct variable mutation
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                    AssignTarget::Index { base, .. } => {
                        // DEPYLER-0235 FIX: Index assignments also require mutability
                        // e.g., `arr[i] = value` requires `let mut arr = ...`
                        // Also check if this is a loop variable that traces back to a parameter
                        if let HirExpr::Var(var_name) = base.as_ref() {
                            if let Some(param_name) = loop_var_origins.get(var_name) {
                                // Mutating through a loop variable - mark the source parameter as mutable
                                mutable.insert(param_name.clone());
                            } else {
                                // Direct variable mutation
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                    AssignTarget::Slice { base, .. } => {
                        // Slice assignments require mutability
                        // e.g., `arr[1:3] = [10, 20]` requires `let mut arr = ...`
                        if let HirExpr::Var(var_name) = base.as_ref() {
                            if let Some(param_name) = loop_var_origins.get(var_name) {
                                mutable.insert(param_name.clone());
                            } else {
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                    AssignTarget::Starred(_) => {
                        // Starred assignment targets (e.g., *rest in tuple unpacking)
                        // Not commonly used, but should be supported
                        // For now, we don't track specific mutability for starred targets
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
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                    );
                }
                if let Some(else_stmts) = else_body {
                    for stmt in else_stmts {
                        analyze_stmt(
                            stmt,
                            declared,
                            mutable,
                            var_types,
                            mutating_methods,
                            loop_var_origins,
                        );
                    }
                }
            }
            HirStmt::While {
                condition, body, ..
            } => {
                analyze_expr_for_mutations(condition, mutable, var_types, mutating_methods);
                for stmt in body {
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                    );
                }
            }
            HirStmt::For { target, iter, body } => {
                // Track the origin of the loop variable, but only for field access
                // iteration (e.g., `for item in self.items`), which uses &/&mut borrowing.
                // Simple variable iteration (e.g., `for item in items`) generates
                // .iter().cloned(), so mutations to the loop variable don't affect
                // the original collection.
                if let AssignTarget::Symbol(loop_var) = target {
                    if !matches!(iter, HirExpr::Var(_)) {
                        if let Some(param_name) = extract_param_from_expr(iter, declared) {
                            loop_var_origins.insert(loop_var.clone(), param_name);
                        }
                    }
                }

                for stmt in body {
                    analyze_stmt(
                        stmt,
                        declared,
                        mutable,
                        var_types,
                        mutating_methods,
                        loop_var_origins,
                    );
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
            &mut loop_var_origins,
        );
    }

    // Mark variables as mutable when passed to functions expecting &mut parameters
    mark_mut_ref_call_args(stmts, &mut ctx.mutable_vars, &ctx.function_param_borrows);
}

/// Recursively scan statements for function calls that pass variables to &mut parameters,
/// and mark those variables as mutable.
fn mark_mut_ref_call_args(
    stmts: &[HirStmt],
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    for stmt in stmts {
        mark_mut_ref_call_args_in_stmt(stmt, mutable, function_param_borrows);
    }
}

fn mark_mut_ref_call_args_in_expr(
    expr: &HirExpr,
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    match expr {
        HirExpr::Call { func, args, .. } => {
            if let Some(borrows) = function_param_borrows.get(func.as_str()) {
                for (idx, arg) in args.iter().enumerate() {
                    if let HirExpr::Var(var_name) = arg {
                        if let Some(info) = borrows.get(idx) {
                            if info.should_borrow && info.needs_mut {
                                mutable.insert(var_name.clone());
                            }
                        }
                    }
                }
            }
            for arg in args {
                mark_mut_ref_call_args_in_expr(arg, mutable, function_param_borrows);
            }
        }
        HirExpr::MethodCall { object, args, .. } => {
            mark_mut_ref_call_args_in_expr(object, mutable, function_param_borrows);
            for arg in args {
                mark_mut_ref_call_args_in_expr(arg, mutable, function_param_borrows);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            mark_mut_ref_call_args_in_expr(left, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(right, mutable, function_param_borrows);
        }
        HirExpr::Unary { operand, .. } => {
            mark_mut_ref_call_args_in_expr(operand, mutable, function_param_borrows);
        }
        HirExpr::IfExpr { test, body, orelse } => {
            mark_mut_ref_call_args_in_expr(test, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(body, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(orelse, mutable, function_param_borrows);
        }
        HirExpr::List(items)
        | HirExpr::Tuple(items)
        | HirExpr::Set(items)
        | HirExpr::FrozenSet(items) => {
            for item in items {
                mark_mut_ref_call_args_in_expr(item, mutable, function_param_borrows);
            }
        }
        HirExpr::Dict(pairs) => {
            for (key, value) in pairs {
                mark_mut_ref_call_args_in_expr(key, mutable, function_param_borrows);
                mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
            }
        }
        HirExpr::Index { base, index } => {
            mark_mut_ref_call_args_in_expr(base, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(index, mutable, function_param_borrows);
        }
        HirExpr::Attribute { value, .. } => {
            mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
        }
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        } => {
            mark_mut_ref_call_args_in_expr(element, mutable, function_param_borrows);
            mark_mut_ref_call_args_in_expr(iter, mutable, function_param_borrows);
            if let Some(cond) = condition {
                mark_mut_ref_call_args_in_expr(cond, mutable, function_param_borrows);
            }
        }
        HirExpr::FlattenedListComp {
            element,
            generators,
        } => {
            mark_mut_ref_call_args_in_expr(element, mutable, function_param_borrows);
            for generator in generators {
                mark_mut_ref_call_args_in_expr(&generator.iter, mutable, function_param_borrows);
                for cond in &generator.conditions {
                    mark_mut_ref_call_args_in_expr(cond, mutable, function_param_borrows);
                }
            }
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            mark_mut_ref_call_args_in_expr(base, mutable, function_param_borrows);
            if let Some(s) = start {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
            if let Some(s) = stop {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
            if let Some(s) = step {
                mark_mut_ref_call_args_in_expr(s, mutable, function_param_borrows);
            }
        }
        HirExpr::Lambda { body, .. } => {
            mark_mut_ref_call_args_in_expr(body, mutable, function_param_borrows);
        }
        _ => {}
    }
}

fn mark_mut_ref_call_args_in_stmt(
    stmt: &HirStmt,
    mutable: &mut HashSet<String>,
    function_param_borrows: &HashMap<String, Vec<context::ParamBorrowInfo>>,
) {
    match stmt {
        HirStmt::Assign { value, .. } => {
            mark_mut_ref_call_args_in_expr(value, mutable, function_param_borrows);
        }
        HirStmt::Expr(expr) => {
            mark_mut_ref_call_args_in_expr(expr, mutable, function_param_borrows);
        }
        HirStmt::Return(Some(expr)) => {
            mark_mut_ref_call_args_in_expr(expr, mutable, function_param_borrows);
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            mark_mut_ref_call_args_in_expr(condition, mutable, function_param_borrows);
            mark_mut_ref_call_args(then_body, mutable, function_param_borrows);
            if let Some(else_stmts) = else_body {
                mark_mut_ref_call_args(else_stmts, mutable, function_param_borrows);
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            mark_mut_ref_call_args_in_expr(condition, mutable, function_param_borrows);
            mark_mut_ref_call_args(body, mutable, function_param_borrows);
        }
        HirStmt::For { body, .. } => {
            mark_mut_ref_call_args(body, mutable, function_param_borrows);
        }
        _ => {}
    }
}

/// Convert Python classes to Rust structs
///
/// Processes all classes and generates token streams.
/// Complexity: 3 (well within ≤10 target)
fn convert_classes_to_rust(
    classes: &[HirClass],
    type_mapper: &crate::type_mapper::TypeMapper,
) -> Result<Vec<proc_macro2::TokenStream>> {
    // Build abc_classes map for abstract base class lookup
    let mut abc_classes = HashMap::new();
    for class in classes {
        if class.is_abc {
            abc_classes.insert(class.name.clone(), class);
        }
    }

    let mut class_items = Vec::new();
    for class in classes {
        if class.is_abc {
            let trait_item = crate::direct_rules::convert_abc_to_trait(class, type_mapper)?;
            class_items.push(trait_item.to_token_stream());
        } else if class.is_intflag {
            let items = crate::direct_rules::convert_class_to_intflag(class)?;
            for item in items {
                class_items.push(item.to_token_stream());
            }
        } else if class.is_enum {
            let items = crate::direct_rules::convert_class_to_enum(class)?;
            for item in items {
                class_items.push(item.to_token_stream());
            }
        } else {
            let items =
                crate::direct_rules::convert_class_to_struct(class, type_mapper, &abc_classes)?;
            for item in items {
                let tokens = item.to_token_stream();
                class_items.push(tokens);
            }
        }
    }
    Ok(class_items)
}

/// Convert HIR functions to Rust token streams
///
/// Processes all functions using the code generation context.
/// Complexity: 2 (well within ≤10 target)
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
/// Complexity: 1 (data-driven approach, well within ≤10 target)
/// Deduplicate use statements to avoid E0252 errors
///
/// DEPYLER-0335 FIX #1: Multiple sources can generate the same import.
/// For example, both generate_import_tokens and generate_conditional_imports
/// might add `use std::collections::HashMap;`.
///
/// # Complexity
/// ~6 (loop + if + string ops)
fn deduplicate_use_statements(
    items: Vec<proc_macro2::TokenStream>,
) -> Vec<proc_macro2::TokenStream> {
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
        (
            ctx.needs_vecdeque,
            quote! { use std::collections::VecDeque; },
        ),
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

    // SmallRng thread_local for random module
    if ctx.needs_small_rng {
        imports.push(quote! { use rand::Rng; });
        imports.push(quote! { use rand::SeedableRng; });
        imports.push(quote! { use rand::rngs::SmallRng; });
        imports.push(quote! {
            thread_local! {
                static DEPYLER_RNG: std::cell::RefCell<SmallRng> =
                    std::cell::RefCell::new(SmallRng::from_os_rng());
            }
        });
    }

    if ctx.needs_slice_random {
        imports.push(quote! { use rand::seq::SliceRandom; });
    }

    if ctx.needs_indexed_random {
        imports.push(quote! { use rand::seq::IndexedRandom; });
    }

    imports
}

/// Generate import token streams from Python imports
///
/// Maps Python imports to Rust use statements.
/// Complexity: ~7-8 (within ≤10 target)
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

    // DEPYLER-0335 FIX #1: Deduplicate imports using HashSet
    // Multiple Python imports can map to same Rust type (e.g., defaultdict + Counter -> HashMap)
    let mut seen_paths = std::collections::HashSet::new();

    // Add external imports (deduplicated)
    for import in external_imports {
        // Create unique key from path + alias
        let key = format!("{}:{:?}", import.path, import.alias);
        if !seen_paths.insert(key) {
            continue; // Skip duplicate
        }

        let path: syn::Path =
            syn::parse_str(&import.path).unwrap_or_else(|_| parse_quote! { unknown });
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

/// Generate interned string constant tokens
///
/// Generates constant definitions for interned strings.
/// Complexity: 2 (well within ≤10 target)
fn generate_interned_string_tokens(_optimizer: &StringOptimizer) -> Vec<proc_macro2::TokenStream> {
    // TODO: Implement generate_interned_constants method in StringOptimizer
    // let interned_constants = optimizer.generate_interned_constants();
    // interned_constants
    //     .into_iter()
    //     .filter_map(|constant| constant.parse().ok())
    //     .collect()
    vec![]
}

/// Infer the Rust type for a single HIR expression.
fn infer_single_expr_type(expr: &HirExpr) -> proc_macro2::TokenStream {
    match expr {
        HirExpr::Literal(Literal::Int(_)) => quote! { i32 },
        HirExpr::Literal(Literal::Float(_)) => quote! { f64 },
        HirExpr::Literal(Literal::String(_)) => quote! { String },
        HirExpr::Literal(Literal::Bool(_)) => quote! { bool },
        HirExpr::Unary { op, operand } => infer_unary_type(op, operand),
        HirExpr::List(inner) => {
            let inner_type = infer_list_element_type(inner);
            quote! { Vec<#inner_type> }
        }
        HirExpr::Tuple(elems) => infer_tuple_type(elems),
        _ => quote! { serde_json::Value },
    }
}

/// Infer the Rust element type for a list constant from its elements.
///
/// Checks all elements for type consistency. Falls back to `serde_json::Value`
/// for empty or heterogeneous lists.
fn infer_list_element_type(elts: &[HirExpr]) -> proc_macro2::TokenStream {
    let first = match elts.first() {
        Some(expr) => infer_single_expr_type(expr),
        None => return quote! { serde_json::Value },
    };
    let first_str = first.to_string();
    for expr in &elts[1..] {
        if infer_single_expr_type(expr).to_string() != first_str {
            return quote! { serde_json::Value };
        }
    }
    first
}

/// Infer the Rust key and value types for a dict constant from its entries.
///
/// Returns `(key_type, value_type)`. Falls back to `serde_json::Value` for
/// empty or heterogeneous dicts.
fn infer_dict_kv_types(
    pairs: &[(HirExpr, HirExpr)],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let fallback = || (quote! { serde_json::Value }, quote! { serde_json::Value });
    let (first_k, first_v) = match pairs.first() {
        Some((k, v)) => (infer_single_expr_type(k), infer_single_expr_type(v)),
        None => return fallback(),
    };
    let k_str = first_k.to_string();
    let v_str = first_v.to_string();
    for (k, v) in &pairs[1..] {
        if infer_single_expr_type(k).to_string() != k_str
            || infer_single_expr_type(v).to_string() != v_str
        {
            return fallback();
        }
    }
    (first_k, first_v)
}

/// Infer the Rust element type for a set constant from its elements.
fn infer_set_element_type(elts: &[HirExpr]) -> proc_macro2::TokenStream {
    infer_list_element_type(elts)
}

/// Infer the Rust tuple type from its elements, e.g. `(i32, i32)` or `(String, f64)`.
fn infer_tuple_type(elems: &[HirExpr]) -> proc_macro2::TokenStream {
    let types: Vec<proc_macro2::TokenStream> = elems.iter().map(infer_single_expr_type).collect();
    quote! { (#(#types),*) }
}

/// Infer the Rust type for a unary expression based on the operator and operand.
fn infer_unary_type(op: &UnaryOp, operand: &HirExpr) -> proc_macro2::TokenStream {
    match (op, operand) {
        (UnaryOp::Neg | UnaryOp::Pos, HirExpr::Literal(Literal::Int(_))) => quote! { i32 },
        (UnaryOp::Neg | UnaryOp::Pos, HirExpr::Literal(Literal::Float(_))) => quote! { f64 },
        (UnaryOp::Not, HirExpr::Literal(Literal::Bool(_))) => quote! { bool },
        _ => quote! { serde_json::Value },
    }
}

/// Check if a tuple element expression requires heap allocation.
fn tuple_element_needs_heap(expr: &HirExpr) -> bool {
    matches!(
        expr,
        HirExpr::Literal(Literal::String(_)) | HirExpr::List(_) | HirExpr::Dict(_)
    )
}

/// Check if a list element is const-safe (can live in a static array without heap allocation).
fn is_const_safe_list_element(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Literal(Literal::Int(_) | Literal::Float(_) | Literal::Bool(_)) => true,
        HirExpr::Unary { operand, .. } => is_const_safe_list_element(operand),
        HirExpr::Tuple(elems) => elems.iter().all(is_const_safe_list_element),
        _ => false,
    }
}

/// Check if a RustType requires heap allocation (cannot be `const`).
fn is_heap_allocated_rust_type(ty: &crate::type_mapper::RustType) -> bool {
    use crate::type_mapper::RustType;
    matches!(
        ty,
        RustType::String
            | RustType::Vec(_)
            | RustType::HashMap(_, _)
            | RustType::HashSet(_)
            | RustType::Custom(_)
    )
}

/// Infer the HIR Type for a module-level constant from its value expression.
fn infer_constant_hir_type(expr: &HirExpr) -> Type {
    match expr {
        HirExpr::Literal(Literal::Int(_)) => Type::Int,
        HirExpr::Literal(Literal::Float(_)) => Type::Float,
        HirExpr::Literal(Literal::String(_)) => Type::String,
        HirExpr::Literal(Literal::Bool(_)) => Type::Bool,
        HirExpr::Unary { operand, .. } => infer_constant_hir_type(operand),
        HirExpr::Tuple(elems) => {
            let elem_types: Vec<Type> = elems.iter().map(infer_constant_hir_type).collect();
            if elem_types.iter().any(|t| matches!(t, Type::Unknown)) {
                Type::Unknown
            } else {
                Type::Tuple(elem_types)
            }
        }
        HirExpr::List(elems) => {
            if elems.is_empty() {
                return Type::Unknown;
            }
            let elem_type = infer_constant_hir_type(&elems[0]);
            if matches!(elem_type, Type::Unknown) {
                Type::Unknown
            } else {
                Type::List(Box::new(elem_type))
            }
        }
        _ => Type::Unknown,
    }
}

/// Generate module-level constant tokens
///
/// Generates `pub const` for primitive types (i32, f64, bool, &str).
/// Uses `lazy_static!` for heap-allocated types (Vec, String, HashMap, etc.).
fn generate_constant_tokens(
    constants: &[HirConstant],
    ctx: &mut CodeGenContext,
) -> Result<Vec<proc_macro2::TokenStream>> {
    use crate::rust_gen::context::ToRustExpr;

    let mut items = Vec::new();

    for constant in constants {
        let name_ident = syn::Ident::new(&constant.name, proc_macro2::Span::call_site());

        // Generate the value expression
        let mut value_expr = constant.value.to_rust_expr(ctx)?;

        // Determine type annotation and whether it needs lazy_static
        let (type_annotation, needs_lazy) = if let Some(ref ty) = constant.type_annotation {
            let rust_type = ctx.type_mapper.map_type(ty);
            let needs_lazy = is_heap_allocated_rust_type(&rust_type);
            let syn_type = type_gen::rust_type_to_syn(&rust_type)?;
            (quote! { : #syn_type }, needs_lazy)
        } else {
            // DEPYLER-0448: Infer type from expression (not just literals)
            match &constant.value {
                // Literal types - const-safe
                HirExpr::Literal(Literal::Int(_)) => (quote! { : i32 }, false),
                HirExpr::Literal(Literal::Float(_)) => (quote! { : f64 }, false),
                HirExpr::Literal(Literal::String(_)) => (quote! { : &str }, false),
                HirExpr::Literal(Literal::Bool(_)) => (quote! { : bool }, false),

                // String method calls return String - needs lazy_static
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "upper"
                                | "lower"
                                | "strip"
                                | "lstrip"
                                | "rstrip"
                                | "replace"
                                | "title"
                                | "capitalize"
                                | "swapcase"
                                | "center"
                                | "ljust"
                                | "rjust"
                                | "zfill"
                                | "expandtabs"
                                | "join"
                        ) =>
                {
                    (quote! { : String }, true)
                }

                // String method calls that return i32 - const-safe
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "find" | "rfind" | "index" | "rindex" | "count"
                        ) =>
                {
                    (quote! { : i32 }, false)
                }

                // String method calls that return bool - const-safe
                HirExpr::MethodCall { object, method, .. }
                    if matches!(object.as_ref(), HirExpr::Literal(Literal::String(_)))
                        && matches!(
                            method.as_str(),
                            "startswith"
                                | "endswith"
                                | "isdigit"
                                | "isalpha"
                                | "isalnum"
                                | "isspace"
                                | "islower"
                                | "isupper"
                                | "istitle"
                                | "isascii"
                                | "isprintable"
                        ) =>
                {
                    (quote! { : bool }, false)
                }

                // Unary expressions (e.g., -3, +5, not True) - infer from operand
                HirExpr::Unary { op, operand } => {
                    let ty = infer_unary_type(op, operand);
                    (quote! { : #ty }, false)
                }

                // Dict constants: infer HashMap<K, V> from entries
                HirExpr::Dict(pairs) => {
                    let (kt, vt) = infer_dict_kv_types(pairs);
                    let kt_s = kt.to_string();
                    let vt_s = vt.to_string();
                    if kt_s.contains("serde_json") || vt_s.contains("serde_json") {
                        ctx.needs_serde_json = true;
                        (quote! { : serde_json::Value }, true)
                    } else {
                        ctx.needs_hashmap = true;
                        (quote! { : HashMap<#kt, #vt> }, true)
                    }
                }

                // Set constants: infer HashSet<T> from elements
                HirExpr::Set(elts) | HirExpr::FrozenSet(elts) => {
                    let elem_type = infer_set_element_type(elts);
                    let elem_s = elem_type.to_string();
                    if elem_s.contains("serde_json") {
                        ctx.needs_serde_json = true;
                        (quote! { : serde_json::Value }, true)
                    } else {
                        ctx.needs_hashset = true;
                        (quote! { : HashSet<#elem_type> }, true)
                    }
                }

                // Const-safe lists → pub static array; otherwise lazy_static Vec
                HirExpr::List(elts) => {
                    let elem_type = infer_list_element_type(elts);
                    let elem_type_str = elem_type.to_string();
                    if !elts.is_empty()
                        && !elem_type_str.contains("serde_json")
                        && elts.iter().all(is_const_safe_list_element)
                    {
                        let len_lit = syn::LitInt::new(
                            &elts.len().to_string(),
                            proc_macro2::Span::call_site(),
                        );
                        let elem_exprs: Vec<syn::Expr> = elts
                            .iter()
                            .map(|e| e.to_rust_expr(ctx))
                            .collect::<Result<Vec<_>>>()?;
                        value_expr = syn::parse_quote! { [#(#elem_exprs),*] };
                        (quote! { : [#elem_type; #len_lit] }, false)
                    } else {
                        (quote! { : Vec<#elem_type> }, true)
                    }
                }

                // Tuple constants - const-safe if all elements are primitive
                HirExpr::Tuple(elems) => {
                    let tuple_type = infer_tuple_type(elems);
                    let needs_lazy = elems.iter().any(tuple_element_needs_heap);
                    (quote! { : #tuple_type }, needs_lazy)
                }

                // DEPYLER-0448: Default fallback → serde_json::Value - needs lazy_static
                _ => {
                    ctx.needs_serde_json = true;
                    (quote! { : serde_json::Value }, true)
                }
            }
        };

        let use_static = matches!(&constant.value, HirExpr::List(_))
            && !needs_lazy;

        if needs_lazy {
            ctx.needs_lazy_static = true;
            ctx.lazy_static_constants.insert(constant.name.clone());
            items.push(quote! {
                lazy_static::lazy_static! {
                    pub static ref #name_ident #type_annotation = #value_expr;
                }
            });
        } else if use_static {
            ctx.static_array_constants.insert(constant.name.clone());
            items.push(quote! {
                pub static #name_ident #type_annotation = #value_expr;
            });
        } else {
            items.push(quote! {
                pub const #name_ident #type_annotation = #value_expr;
            });
        }
    }

    Ok(items)
}

/// Generate a complete Rust file from HIR module
pub fn generate_rust_file(
    module: &HirModule,
    type_mapper: &crate::type_mapper::TypeMapper,
) -> Result<(String, Vec<cargo_toml_gen::Dependency>)> {
    let module_mapper = crate::module_mapper::ModuleMapper::new();

    // INTERPROCEDURAL ANALYSIS: Analyze cross-function mutations
    let mut interprocedural_analyzer = crate::interprocedural::InterproceduralAnalyzer::new(module);
    let interprocedural_analysis = interprocedural_analyzer.analyze();

    // Process imports to populate the context
    let (imported_modules, imported_items) =
        process_module_imports(&module.imports, &module_mapper);

    // Extract class names from module (DEPYLER-0230: distinguish user classes from builtins)
    let class_names: HashSet<String> = module
        .classes
        .iter()
        .map(|class| class.name.clone())
        .collect();

    // DEPYLER-0231: Build map of mutating methods (class_name -> set of method names)
    let mut mutating_methods: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
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
        current_function_name: None,
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
        function_return_types: std::collections::HashMap::new(), // DEPYLER-0269: Track function return types
        function_param_borrows: std::collections::HashMap::new(), // DEPYLER-0270: Track parameter borrowing
        function_param_strategies: std::collections::HashMap::new(),
        current_function_param_ownership: std::collections::HashMap::new(),
        param_clone_requirements: HashSet::new(),
        tuple_iter_vars: HashSet::new(), // DEPYLER-0307 Fix #9: Track tuple iteration variables
        is_final_statement: false, // DEPYLER-0271: Track final statement for expression-based returns
        result_bool_functions: HashSet::new(), // DEPYLER-0308: Track functions returning Result<bool>
        result_returning_functions: HashSet::new(), // DEPYLER-0270: Track ALL Result-returning functions
        current_error_type: None, // DEPYLER-0310: Track error type for raise statement wrapping
        exception_scopes: Vec::new(), // DEPYLER-0333: Exception scope tracking stack
        argparser_tracker: argparse_transform::ArgParserTracker::new(), // DEPYLER-0363: Track ArgumentParser patterns
        generated_args_struct: None, // DEPYLER-0424: Args struct (hoisted to module level)
        generated_commands_enum: None, // DEPYLER-0424: Commands enum (hoisted to module level)
        current_subcommand_fields: None, // DEPYLER-0425: Subcommand field extraction
        validator_functions: HashSet::new(), // DEPYLER-0447: Track argparse validator functions
        stdlib_mappings: crate::stdlib_mappings::StdlibMappings::new(), // DEPYLER-0452: Stdlib API mappings
        interprocedural_analysis: Some(&interprocedural_analysis), // Interprocedural mutation analysis
        classes_needing_dynamic_access: HashSet::new(),
        current_func_mut_ref_params: HashSet::new(),
        current_func_ref_params: HashSet::new(),
        shadowed_ref_params: HashSet::new(),
        function_param_names: HashMap::new(),
        function_param_types: HashMap::new(),
        var_usage_counts: HashMap::new(),
        var_usage_current: HashMap::new(),
        optional_vars: HashSet::new(),
        lazy_static_constants: HashSet::new(),
        static_array_constants: HashSet::new(),
        is_assignment_target: false,
        prevent_clone: false,
        returns_reference: false,
        returns_mutable_reference: false,
        borrowable_vars: HashSet::new(),
        mut_borrowable_vars: HashSet::new(),
        generate_borrow: false,
        generate_mut_borrow: false,
        clone_already_applied: false,
        in_primitive_cast: false,
        vars_needing_clone_at_assign: HashSet::new(),
        needs_smallvec: false,
        needs_small_rng: false,
        needs_slice_random: false,
        needs_indexed_random: false,
        needs_unicode_normalization: false,
        needs_lazy_static: false,
        needs_complex: false,
        enum_names: HashSet::new(),
        copy_structs: HashSet::new(),
        class_field_types: HashMap::new(),
        function_param_muts: HashMap::new(),
        functions_with_mutated_return: HashSet::new(),
        functions_returning_refs: HashSet::new(),
    };

    // Analyze all functions first for string optimization
    analyze_string_optimization(&mut ctx, &module.functions);

    // TODO: Finalize interned string names (resolve collisions)
    // ctx.string_optimizer.finalize_interned_names();

    // DEPYLER-0447: Scan all function bodies and constants for argparse validators
    // Must run BEFORE function conversion so validator parameter types are correct
    analyze_validators(&mut ctx, &module.functions, &module.constants);

    // Populate enum_names so expression generation uses :: instead of . for enum access
    for class in &module.classes {
        if class.is_enum || class.is_intflag {
            ctx.enum_names.insert(class.name.clone());
        }
    }

    // Populate class_field_types so field_needs_clone() can determine when .clone() is needed
    // Also populate copy_structs for structs where all fields are Copy types
    for class in &module.classes {
        if !class.is_enum && !class.is_intflag {
            let mut field_map = HashMap::new();
            for field in &class.fields {
                field_map.insert(field.name.clone(), field.field_type.clone());
            }
            // Track structs that derive Copy (all instance fields are Copy types)
            let has_drop_impl = class
                .methods
                .iter()
                .any(|m| m.name == "__del__" || m.name == "close");
            let all_fields_copyable = class
                .fields
                .iter()
                .filter(|f| !f.is_class_var)
                .all(|f| is_field_copy_type(&f.field_type));
            if all_fields_copyable && !has_drop_impl {
                ctx.copy_structs.insert(class.name.clone());
            }
            ctx.class_field_types.insert(class.name.clone(), field_map);
        }
    }

    // Register module-level constant types so is_expr_float_type/is_expr_int_type
    // can recognize constants used inside function bodies for mixed-type arithmetic casts
    for constant in &module.constants {
        let const_type = if let Some(ref ty) = constant.type_annotation {
            ty.clone()
        } else {
            infer_constant_hir_type(&constant.value)
        };
        if !matches!(const_type, Type::Unknown) {
            ctx.var_types.insert(constant.name.clone(), const_type);
        }
    }

    // Pre-populate lazy_static_constants and static_array_constants so function
    // code generation knows which uppercase names are constants.
    for constant in &module.constants {
        if constant
            .name
            .chars()
            .next()
            .map_or(false, |c| c.is_uppercase())
        {
            if let HirExpr::List(elts) = &constant.value {
                let elem_type = infer_list_element_type(elts);
                let elem_type_str = elem_type.to_string();
                if !elts.is_empty()
                    && !elem_type_str.contains("serde_json")
                    && elts.iter().all(is_const_safe_list_element)
                {
                    ctx.static_array_constants.insert(constant.name.clone());
                    continue;
                }
            }
            ctx.lazy_static_constants.insert(constant.name.clone());
        }
    }

    // PRE-POPULATE function_param_borrows for ALL functions BEFORE code generation
    // This ensures that when function A calls function B, it knows B's parameter signature
    // even if B hasn't been generated yet. Fixes issue where call sites default to
    // immutable borrow when they should use mutable borrow or pass by value.
    populate_function_param_borrows(&module.functions, &mut ctx)?;

    // Pre-populate function signatures so forward-declared functions' types
    // are available when earlier functions are generated.
    for func in &module.functions {
        ctx.function_return_types
            .insert(func.name.clone(), func.ret_type.clone());
        ctx.function_param_types.insert(
            func.name.clone(),
            func.params.iter().map(|p| p.ty.clone()).collect(),
        );
    }

    // Convert classes first (they might be used by functions)
    let classes = convert_classes_to_rust(&module.classes, ctx.type_mapper)?;

    // Convert all functions to detect what imports we need
    let functions = convert_functions_to_rust(&module.functions, &mut ctx)?;

    // Build items list with all generated code
    let mut items = Vec::new();

    // Add module imports (create new mapper for token generation)
    let import_mapper = crate::module_mapper::ModuleMapper::new();
    items.extend(generate_import_tokens(&module.imports, &import_mapper));

    // Add interned string constants
    items.extend(generate_interned_string_tokens(&ctx.string_optimizer));

    // Generate module-level constants first (populates ctx.needs_hashmap, ctx.needs_hashset, etc.)
    let constant_tokens = generate_constant_tokens(&module.constants, &mut ctx)?;

    // Add collection imports if needed (must come before constants in output)
    items.extend(generate_conditional_imports(&ctx));

    // Now add the constants after their imports
    items.extend(constant_tokens);

    // DEPYLER-0335 FIX #1: Deduplicate imports across all sources
    // Both generate_import_tokens and generate_conditional_imports can add HashMap
    items = deduplicate_use_statements(items);

    // Add error type definitions if needed
    items.extend(generate_error_type_definitions(&ctx));

    // Add generated union enums
    items.extend(ctx.generated_enums.clone());

    // Add classes
    items.extend(classes);

    // DEPYLER-0424: Add ArgumentParser-generated structs at module level
    // (before functions so handler functions can reference Args type)
    if let Some(ref commands_enum) = ctx.generated_commands_enum {
        items.push(commands_enum.clone());
    }
    if let Some(ref args_struct) = ctx.generated_args_struct {
        items.push(args_struct.clone());
    }

    // Add all functions
    items.extend(functions);

    let file = quote! {
        #(#items)*
    };

    // DEPYLER-0384: Extract dependencies from context (BEFORE post-processing)
    let mut dependencies = cargo_toml_gen::extract_dependencies(&ctx);

    // Format the code first (this is when tokens become readable strings)
    let mut formatted_code = format_rust_code(file.to_string());

    // DEPYLER-0393: Post-process FORMATTED code to detect missed dependencies
    // TokenStreams don't have literal strings - must scan AFTER formatting
    if formatted_code.contains("serde_json::") && !ctx.needs_serde_json {
        // Add missing import at the beginning
        formatted_code = format!("use serde_json;\n{}", formatted_code);
        // Add missing Cargo.toml dependencies
        dependencies.push(cargo_toml_gen::Dependency::new("serde_json", "1.0"));
        dependencies.push(
            cargo_toml_gen::Dependency::new("serde", "1.0")
                .with_features(vec!["derive".to_string()]),
        );
        // Re-format to ensure imports are properly ordered
        formatted_code = format_rust_code(formatted_code);
    }

    Ok((formatted_code, dependencies))
}

/// Generate a main() function from module-level statements
///
/// In Python, module-level statements are executed when the module is imported or run.
/// In Rust, we need to wrap these in a main() function.
fn generate_main_from_statements(
    statements: &[HirStmt],
    ctx: &mut CodeGenContext,
) -> Result<proc_macro2::TokenStream> {
    use crate::rust_gen::context::RustCodeGen;

    // Generate Rust code for each statement
    let mut body_tokens = Vec::new();
    for stmt in statements {
        let stmt_tokens = stmt.to_rust_tokens(ctx)?;
        body_tokens.push(stmt_tokens);
    }

    // Wrap in a main() function
    Ok(quote! {
        fn main() {
            #(#body_tokens)*
        }
    })
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
            annotation_aware_mapper: AnnotationAwareTypeMapper::with_base_mapper(
                type_mapper.clone(),
            ),
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
            current_function_name: None,
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
            function_return_types: std::collections::HashMap::new(), // DEPYLER-0269: Track function return types
            function_param_borrows: std::collections::HashMap::new(), // DEPYLER-0270: Track parameter borrowing
            function_param_strategies: std::collections::HashMap::new(),
            current_function_param_ownership: std::collections::HashMap::new(),
            param_clone_requirements: HashSet::new(),
            tuple_iter_vars: HashSet::new(), // DEPYLER-0307 Fix #9: Track tuple iteration variables
            is_final_statement: false, // DEPYLER-0271: Track final statement for expression-based returns
            result_bool_functions: HashSet::new(), // DEPYLER-0308: Track functions returning Result<bool>
            result_returning_functions: HashSet::new(), // DEPYLER-0270: Track ALL Result-returning functions
            current_error_type: None, // DEPYLER-0310: Track error type for raise statement wrapping
            exception_scopes: Vec::new(), // DEPYLER-0333: Exception scope tracking stack
            argparser_tracker: argparse_transform::ArgParserTracker::new(), // DEPYLER-0363: Track ArgumentParser patterns
            generated_args_struct: None, // DEPYLER-0424: Args struct (hoisted to module level)
            generated_commands_enum: None, // DEPYLER-0424: Commands enum (hoisted to module level)
            current_subcommand_fields: None, // DEPYLER-0425: Subcommand field extraction
            validator_functions: HashSet::new(), // DEPYLER-0447: Track argparse validator functions
            stdlib_mappings: crate::stdlib_mappings::StdlibMappings::new(), // DEPYLER-0452
            interprocedural_analysis: None,
            classes_needing_dynamic_access: HashSet::new(),
            current_func_mut_ref_params: HashSet::new(),
            current_func_ref_params: HashSet::new(),
            shadowed_ref_params: HashSet::new(),
            function_param_names: HashMap::new(),
            function_param_types: HashMap::new(),
            var_usage_counts: HashMap::new(),
            var_usage_current: HashMap::new(),
            optional_vars: HashSet::new(),
            lazy_static_constants: HashSet::new(),
            static_array_constants: HashSet::new(),
            is_assignment_target: false,
            prevent_clone: false,
            returns_reference: false,
            returns_mutable_reference: false,
            borrowable_vars: HashSet::new(),
            mut_borrowable_vars: HashSet::new(),
            generate_borrow: false,
            generate_mut_borrow: false,
            clone_already_applied: false,
            in_primitive_cast: false,
            vars_needing_clone_at_assign: HashSet::new(),
            needs_smallvec: false,
            needs_small_rng: false,
            needs_slice_random: false,
            needs_indexed_random: false,
            needs_unicode_normalization: false,
            needs_lazy_static: false,
            needs_complex: false,
            enum_names: HashSet::new(),
            copy_structs: HashSet::new(),
            class_field_types: HashMap::new(),
            function_param_muts: HashMap::new(),
            functions_with_mutated_return: HashSet::new(),
            functions_returning_refs: HashSet::new(),
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
        // DEPYLER-0271: Final return statements use expression-based returns (no `return` keyword)
        // The function body should contain the expression result without explicit `return`
        assert!(
            code.contains("a + b"),
            "Function should contain expression 'a + b'"
        );
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
            else_body: Some(vec![HirStmt::Return(Some(HirExpr::Literal(
                Literal::String("negative".to_string()),
            )))]),
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
        let var_list = HirExpr::List(vec![
            HirExpr::Var("x".to_string()),
            HirExpr::Var("y".to_string()),
        ]);

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
    // DEPYLER-0140 Phase 1: Tests for extracted statement handlers
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
    // DEPYLER-0140 Phase 2: Tests for medium-complexity statement handlers
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
        let exc = Some(HirExpr::Literal(Literal::String("Error".to_string())));

        let result = codegen_raise_stmt(&exc, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "panic ! (\"{}\" , \"Error\") ;");
    }

    #[test]
    fn test_codegen_raise_stmt_bare() {
        let mut ctx = create_test_context();

        let result = codegen_raise_stmt(&None, &mut ctx).unwrap();
        assert_eq!(result.to_string(), "panic ! (\"Exception raised\") ;");
    }

    // NOTE: With statement with target incomplete - requires full implementation (tracked in DEPYLER-0424)
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
        // DEPYLER-0257 REFACTOR v3: Simplified try/except (no Result wrapper)
        // Just executes try block statements directly
        assert!(!result_str.is_empty(), "Should generate code");
        // Code should be simple block execution (no complex patterns for now)
    }

    #[test]
    fn test_codegen_try_stmt_with_finally() {
        use crate::hir::Literal;

        let mut ctx = create_test_context();
        // Use an actual statement in try block (not just pass)
        let body = vec![HirStmt::Expr(HirExpr::Literal(Literal::Int(1)))];
        let handlers = vec![];
        // Use an actual statement in finally block
        let finally = Some(vec![HirStmt::Expr(HirExpr::Literal(Literal::Int(2)))]);

        let result = codegen_try_stmt(&body, &handlers, &finally, &mut ctx).unwrap();
        let result_str = result.to_string();
        // Should contain both the try expression and finally expression
        assert!(result_str.contains("1"), "Should contain try block code");
        assert!(
            result_str.contains("2"),
            "Should contain finally block code"
        );
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
        // DEPYLER-0257 REFACTOR v3: Simplified try/except with finally
        // Executes try block then finally block
        assert!(!result_str.is_empty(), "Should generate code");
        // Code should execute try block and finally block
    }

    // Phase 1b/1c tests - Type conversion functions (DEPYLER-0149, DEPYLER-0216)
    #[test]
    fn test_int_cast_conversion() {
        // DEPYLER-0216 FIX: Python: int(x) → Rust: (x) as i32 (always cast variables)
        // Previous behavior (no cast) caused "cannot add bool to bool" errors
        // when x is a bool variable: int(flag1) + int(flag2) → flag1 + flag2 (ERROR!)
        let call_expr = HirExpr::Call {
            func: "int".to_string(),
            args: vec![HirExpr::Var("x".to_string())],
            kwargs: vec![],
            type_params: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        // Should generate cast for variables to prevent bool arithmetic errors
        assert!(code.contains("x"), "Expected 'x', got: {}", code);
        assert!(
            code.contains("as i32"),
            "Should contain 'as i32' cast, got: {}",
            code
        );
    }

    #[test]
    fn test_float_cast_conversion() {
        // Python: float(x) → Rust: (x) as f64
        let call_expr = HirExpr::Call {
            func: "float".to_string(),
            args: vec![HirExpr::Var("y".to_string())],
            kwargs: vec![],
            type_params: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        assert!(
            code.contains("as f64"),
            "Expected '(y) as f64', got: {}",
            code
        );
    }

    #[test]
    fn test_str_conversion() {
        // Python: str(x) → Rust: x.to_string()
        let call_expr = HirExpr::Call {
            func: "str".to_string(),
            args: vec![HirExpr::Var("value".to_string())],
            kwargs: vec![],
            type_params: vec![],
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

    // NOTE: Boolean casting incomplete - requires type cast implementation (tracked in DEPYLER-0424)
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
            type_params: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        assert!(
            code.contains("as bool"),
            "Expected '(flag) as bool', got: {}",
            code
        );
    }

    #[test]
    fn test_int_cast_with_expression() {
        // DEPYLER-0216 FIX: Python: int((low + high) / 2) → Rust: ((low + high) / 2) as i32
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
            type_params: vec![],
        };

        let mut ctx = create_test_context();
        let result = call_expr.to_rust_expr(&mut ctx).unwrap();
        let code = quote! { #result }.to_string();

        // Should generate cast for expressions to prevent bool arithmetic errors
        assert!(
            code.contains("low"),
            "Expected 'low' variable, got: {}",
            code
        );
        assert!(
            code.contains("high"),
            "Expected 'high' variable, got: {}",
            code
        );
        assert!(
            code.contains("as i32"),
            "Should contain 'as i32' cast, got: {}",
            code
        );
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
                type_params: vec![],
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
                type_params: vec![],
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
                type_params: vec![],
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
