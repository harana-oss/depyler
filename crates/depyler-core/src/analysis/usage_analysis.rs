//! Static analysis passes for variable usage, field borrowing, and move tracking.
//!
//! These analysis passes run over a function body *before* code generation begins
//! to populate borrow-eligibility and clone-necessity information on
//! [`CodeGenContext`].  They have no business living in the context module itself
//! because they are independent analysis algorithms that happen to mutate context
//! state; keeping them here makes them independently readable and testable.
//!
//! The three public entry-points are methods on `CodeGenContext` so that existing
//! call sites (`ctx.analyze_field_borrowing(&body)`) require no changes.

use crate::hir::{AssignTarget, HirExpr, HirStmt};
use crate::rust_generator::context::CodeGenContext;
use std::collections::HashMap;

impl<'a> CodeGenContext<'a> {
    // ========================================================================
    // Public analysis entry-points
    // ========================================================================

    /// Analyze variable usage in function body before code generation.
    ///
    /// Resets all usage counters, counts every variable reference, and
    /// determines which field-source variables can be borrowed rather than cloned.
    pub fn analyze_var_usage(&mut self, stmts: &[HirStmt]) {
        self.reset_var_usage();
        self.borrowable_vars.clear();
        self.mut_borrowable_vars.clear();

        // First pass: identify field-source variables and count all uses
        let field_source_vars = collect_field_source_vars(self, stmts);
        for stmt in stmts {
            count_var_uses_in_stmt(self, stmt);
        }

        // Second pass: analyze if field-source variables can be borrowed
        for var_name in &field_source_vars {
            // Check for mutable borrowing first (requires &mut source AND mutation)
            if can_var_mut_borrow(self, stmts, var_name) {
                self.mut_borrowable_vars.insert(var_name.clone());
            } else if can_var_borrow(self, stmts, var_name) {
                self.borrowable_vars.insert(var_name.clone());
            }
        }
    }

    /// Analyze only borrow eligibility for field-source variables without
    /// resetting `var_usage_counts` (avoids side effects on clone decisions).
    pub fn analyze_field_borrowing(&mut self, stmts: &[HirStmt]) {
        self.borrowable_vars.clear();
        self.mut_borrowable_vars.clear();

        let field_source_vars = collect_field_source_vars(self, stmts);
        for var_name in &field_source_vars {
            if can_var_mut_borrow(self, stmts, var_name) {
                self.mut_borrowable_vars.insert(var_name.clone());
            } else if can_var_borrow(self, stmts, var_name) {
                self.borrowable_vars.insert(var_name.clone());
            }
        }
    }

    /// Detect variables consumed in multiple move positions and mark them for cloning.
    ///
    /// Consuming positions: attribute-assignment RHS, push/append/extend args,
    /// return values, and function call args where the callee takes ownership.
    pub fn analyze_move_consuming_uses(&mut self, stmts: &[HirStmt]) {
        self.vars_needing_clone_for_move.clear();
        self.move_consume_current.clear();
        self.move_consume_totals.clear();
        let mut consuming_counts: HashMap<String, usize> = HashMap::new();
        for stmt in stmts {
            count_move_consuming_uses_in_stmt(self, stmt, &mut consuming_counts);
        }
        for (name, count) in &consuming_counts {
            if *count > 1 {
                // Don't clone borrowed function parameters (&T is Copy)
                let is_borrowed_param = self
                    .current_function_param_ownership
                    .get(name)
                    .is_some_and(|takes_ownership| !takes_ownership);
                if !is_borrowed_param {
                    self.vars_needing_clone_for_move.insert(name.clone());
                }
            }
        }
        self.move_consume_totals = consuming_counts;
    }
}

// ============================================================================
// Private helpers – variable counting
// ============================================================================

fn count_var_uses_in_stmt(ctx: &mut CodeGenContext, stmt: &HirStmt) {
    match stmt {
        HirStmt::Assign { value, .. } => {
            count_var_uses_in_expr(ctx, value);
        }
        HirStmt::Expr(expr) => {
            count_var_uses_in_expr(ctx, expr);
        }
        HirStmt::Return(Some(expr)) => {
            count_var_uses_in_expr(ctx, expr);
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            count_var_uses_in_expr(ctx, condition);
            for s in then_body {
                count_var_uses_in_stmt(ctx, s);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    count_var_uses_in_stmt(ctx, s);
                }
            }
        }
        HirStmt::While { condition, body } => {
            count_var_uses_in_expr(ctx, condition);
            for s in body {
                count_var_uses_in_stmt(ctx, s);
            }
        }
        HirStmt::For { iter, body, .. } => {
            count_var_uses_in_expr(ctx, iter);
            for s in body {
                count_var_uses_in_stmt(ctx, s);
            }
        }
        HirStmt::Assert { test, msg, .. } => {
            count_var_uses_in_expr(ctx, test);
            if let Some(m) = msg {
                count_var_uses_in_expr(ctx, m);
            }
        }
        HirStmt::Raise { exception, .. } => {
            if let Some(e) = exception {
                count_var_uses_in_expr(ctx, e);
            }
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            for s in body {
                count_var_uses_in_stmt(ctx, s);
            }
            for handler in handlers {
                for s in &handler.body {
                    count_var_uses_in_stmt(ctx, s);
                }
            }
            if let Some(els) = orelse {
                for s in els {
                    count_var_uses_in_stmt(ctx, s);
                }
            }
            if let Some(fin) = finalbody {
                for s in fin {
                    count_var_uses_in_stmt(ctx, s);
                }
            }
        }
        HirStmt::With { context, body, .. } => {
            count_var_uses_in_expr(ctx, context);
            for s in body {
                count_var_uses_in_stmt(ctx, s);
            }
        }
        _ => {}
    }
}

fn count_var_uses_in_expr(ctx: &mut CodeGenContext, expr: &HirExpr) {
    match expr {
        HirExpr::Var(name) => {
            ctx.count_var_use(name);
        }
        HirExpr::Binary { left, right, .. } => {
            count_var_uses_in_expr(ctx, left);
            count_var_uses_in_expr(ctx, right);
        }
        HirExpr::Unary { operand, .. } => {
            count_var_uses_in_expr(ctx, operand);
        }
        HirExpr::Call { args, kwargs, .. } => {
            for arg in args {
                count_var_uses_in_expr(ctx, arg);
            }
            for (_, v) in kwargs {
                count_var_uses_in_expr(ctx, v);
            }
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            // Method calls borrow the receiver, so if the object is just a variable,
            // don't count it as a consuming use. Only count nested expressions.
            if !matches!(**object, HirExpr::Var(_)) {
                count_var_uses_in_expr(ctx, object);
            }
            for arg in args {
                count_var_uses_in_expr(ctx, arg);
            }
            for (_, v) in kwargs {
                count_var_uses_in_expr(ctx, v);
            }
        }
        HirExpr::Attribute { value, .. } => {
            // Attribute access borrows the object (e.g., person.name gives &String),
            // so accessing person.first_name and person.last_name doesn't consume person.
            // Only count nested expressions, not direct variable access.
            if !matches!(**value, HirExpr::Var(_)) {
                count_var_uses_in_expr(ctx, value);
            }
        }
        HirExpr::Index { base, index } => {
            count_var_uses_in_expr(ctx, base);
            count_var_uses_in_expr(ctx, index);
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            count_var_uses_in_expr(ctx, base);
            if let Some(l) = start {
                count_var_uses_in_expr(ctx, l);
            }
            if let Some(u) = stop {
                count_var_uses_in_expr(ctx, u);
            }
            if let Some(s) = step {
                count_var_uses_in_expr(ctx, s);
            }
        }
        HirExpr::List(elts) | HirExpr::Tuple(elts) | HirExpr::Set(elts) => {
            for e in elts {
                count_var_uses_in_expr(ctx, e);
            }
        }
        HirExpr::Dict(pairs) => {
            for (k, v) in pairs {
                count_var_uses_in_expr(ctx, k);
                count_var_uses_in_expr(ctx, v);
            }
        }
        HirExpr::IfExpr { test, body, orelse } => {
            count_var_uses_in_expr(ctx, test);
            count_var_uses_in_expr(ctx, body);
            count_var_uses_in_expr(ctx, orelse);
        }
        HirExpr::ListComp {
            element,
            target: _,
            iter,
            condition,
        } => {
            count_var_uses_in_expr(ctx, element);
            count_var_uses_in_expr(ctx, iter);
            if let Some(c) = condition {
                count_var_uses_in_expr(ctx, c);
            }
        }
        HirExpr::Lambda { body, .. } => {
            count_var_uses_in_expr(ctx, body);
        }
        _ => {}
    }
}

// ============================================================================
// Private helpers – move-consuming use counting
// ============================================================================

fn count_move_consuming_uses_in_stmt(
    ctx: &CodeGenContext,
    stmt: &HirStmt,
    counts: &mut HashMap<String, usize>,
) {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            match target {
                // obj.field = var → consumes var (moves into field)
                AssignTarget::Attribute { .. } => {
                    collect_consuming_vars(value, counts);
                }
                // let x = var → consumes var (moves into binding)
                AssignTarget::Symbol(_) => {
                    collect_consuming_vars(value, counts);
                }
                _ => {}
            }
        }
        // Bare expression: e.g., list.append(var) or func(var)
        HirStmt::Expr(expr) => {
            if let HirExpr::MethodCall { args, .. } = expr {
                for arg in args {
                    collect_consuming_vars(arg, counts);
                }
            } else if let HirExpr::Call { args, .. } = expr {
                for arg in args {
                    collect_consuming_vars(arg, counts);
                }
            }
        }
        HirStmt::Return(Some(expr)) => {
            collect_consuming_vars(expr, counts);
        }
        // Recurse into nested blocks
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                count_move_consuming_uses_in_stmt(ctx, s, counts);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    count_move_consuming_uses_in_stmt(ctx, s, counts);
                }
            }
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            for s in body {
                count_move_consuming_uses_in_stmt(ctx, s, counts);
            }
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            for s in body {
                count_move_consuming_uses_in_stmt(ctx, s, counts);
            }
            for handler in handlers {
                for s in &handler.body {
                    count_move_consuming_uses_in_stmt(ctx, s, counts);
                }
            }
            if let Some(els) = orelse {
                for s in els {
                    count_move_consuming_uses_in_stmt(ctx, s, counts);
                }
            }
            if let Some(fin) = finalbody {
                for s in fin {
                    count_move_consuming_uses_in_stmt(ctx, s, counts);
                }
            }
        }
        HirStmt::With { body, .. } => {
            for s in body {
                count_move_consuming_uses_in_stmt(ctx, s, counts);
            }
        }
        _ => {}
    }
}

/// Collect top-level variable references that represent a consuming (move) use.
fn collect_consuming_vars(expr: &HirExpr, counts: &mut HashMap<String, usize>) {
    match expr {
        HirExpr::Var(name) => {
            *counts.entry(name.clone()).or_insert(0) += 1;
        }
        // Call args are consuming (the function receives ownership or the codegen
        // will handle borrowing separately)
        HirExpr::Call { args, .. } => {
            for arg in args {
                if let HirExpr::Var(name) = arg {
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        _ => {}
    }
}

// ============================================================================
// Private helpers – field-source variable collection
// ============================================================================

fn collect_field_source_vars(ctx: &CodeGenContext, stmts: &[HirStmt]) -> Vec<String> {
    let mut result = Vec::new();
    for stmt in stmts {
        collect_field_source_vars_in_stmt(ctx, stmt, &mut result);
    }
    result
}

fn collect_field_source_vars_in_stmt(
    ctx: &CodeGenContext,
    stmt: &HirStmt,
    result: &mut Vec<String>,
) {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            if let AssignTarget::Symbol(var_name) = target {
                if is_attribute_sourced(value) {
                    result.push(var_name.clone());
                }
            }
        }
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                collect_field_source_vars_in_stmt(ctx, s, result);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    collect_field_source_vars_in_stmt(ctx, s, result);
                }
            }
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            for s in body {
                collect_field_source_vars_in_stmt(ctx, s, result);
            }
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            for s in body {
                collect_field_source_vars_in_stmt(ctx, s, result);
            }
            for handler in handlers {
                for s in &handler.body {
                    collect_field_source_vars_in_stmt(ctx, s, result);
                }
            }
            if let Some(els) = orelse {
                for s in els {
                    collect_field_source_vars_in_stmt(ctx, s, result);
                }
            }
            if let Some(fin) = finalbody {
                for s in fin {
                    collect_field_source_vars_in_stmt(ctx, s, result);
                }
            }
        }
        HirStmt::With { body, .. } => {
            for s in body {
                collect_field_source_vars_in_stmt(ctx, s, result);
            }
        }
        HirStmt::FunctionDef { body, .. } => {
            for s in body {
                collect_field_source_vars_in_stmt(ctx, s, result);
            }
        }
        _ => {}
    }
}

/// Check if an expression is "attribute-sourced" — either a direct attribute access
/// or a conditional expression where both branches are attribute-sourced.
fn is_attribute_sourced(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { .. } => true,
        HirExpr::Index { base, .. } => is_attribute_sourced(base),
        HirExpr::IfExpr { body, orelse, .. } => {
            is_attribute_sourced(body) && is_attribute_sourced(orelse)
        }
        _ => false,
    }
}

/// Check if an expression is an empty collection initialization.
/// These should not block borrowability since they're just placeholder values.
fn is_empty_collection_init(expr: &HirExpr) -> bool {
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
        HirExpr::Literal(lit) => {
            use crate::hir::Literal;
            match lit {
                Literal::Int(0) => true,
                Literal::Float(f) => *f == 0.0,
                Literal::String(s) => s.is_empty(),
                Literal::Bool(false) | Literal::None => true,
                _ => false,
            }
        }
        _ => false,
    }
}

// ============================================================================
// Private helpers – borrow eligibility
// ============================================================================

fn is_mut_ref_attribute_sourced(ctx: &CodeGenContext, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } => is_mut_ref_base(ctx, value),
        HirExpr::Index { base, .. } => is_mut_ref_attribute_sourced(ctx, base),
        HirExpr::IfExpr { body, orelse, .. } => {
            is_mut_ref_attribute_sourced(ctx, body) && is_mut_ref_attribute_sourced(ctx, orelse)
        }
        _ => false,
    }
}

fn is_mut_ref_base(ctx: &CodeGenContext, expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Var(name) => ctx.current_func_mut_ref_params.contains(name),
        HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
            is_mut_ref_base(ctx, value)
        }
        _ => false,
    }
}

fn can_var_mut_borrow(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    if !var_has_mut_use(ctx, stmts, var_name) {
        return false;
    }
    if !var_has_mut_ref_source(ctx, stmts, var_name) {
        return false;
    }
    !var_has_move_use(ctx, stmts, var_name)
        && !var_captured_in_closure(ctx, stmts, var_name)
        && !var_has_non_attribute_assignment(ctx, stmts, var_name)
}

fn var_has_mut_ref_source(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts
        .iter()
        .any(|s| stmt_has_mut_ref_source(ctx, s, var_name))
}

fn stmt_has_mut_ref_source(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            if let AssignTarget::Symbol(name) = target {
                if name == var_name && is_mut_ref_attribute_sourced(ctx, value) {
                    return true;
                }
            }
            false
        }
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_has_mut_ref_source(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .is_some_and(|e| var_has_mut_ref_source(ctx, e, var_name))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_has_mut_ref_source(ctx, body, var_name)
        }
        _ => false,
    }
}

fn can_var_borrow(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    // Only check for actual moves (returns), not function calls (passed by reference)
    !var_has_return_move(ctx, stmts, var_name)
        && !var_field_access_returned(ctx, stmts, var_name)
        && !var_has_mut_use(ctx, stmts, var_name)
        && !var_captured_in_closure(ctx, stmts, var_name)
        && !var_has_non_attribute_assignment(ctx, stmts, var_name)
}

fn var_field_access_returned(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts
        .iter()
        .any(|s| stmt_has_field_return(ctx, s, var_name))
}

fn stmt_has_field_return(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Return(Some(expr)) => expr_accesses_var_field(ctx, expr, var_name),
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_field_access_returned(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .is_some_and(|e| var_field_access_returned(ctx, e, var_name))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_field_access_returned(ctx, body, var_name)
        }
        _ => false,
    }
}

fn expr_accesses_var_field(ctx: &CodeGenContext, expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Attribute { value, .. } | HirExpr::Index { base: value, .. } => {
            expr_is_or_contains_var(value, var_name)
        }
        HirExpr::IfExpr { body, orelse, .. } => {
            expr_accesses_var_field(ctx, body, var_name)
                || expr_accesses_var_field(ctx, orelse, var_name)
        }
        _ => false,
    }
}

fn var_has_return_move(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts.iter().any(|s| stmt_has_return_move(ctx, s, var_name))
}

fn stmt_has_return_move(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Return(Some(expr)) => expr_is_var_move(expr, var_name),
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_has_return_move(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .is_some_and(|e| var_has_return_move(ctx, e, var_name))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_has_return_move(ctx, body, var_name)
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            var_has_return_move(ctx, body, var_name)
                || handlers
                    .iter()
                    .any(|h| var_has_return_move(ctx, &h.body, var_name))
                || orelse
                    .as_ref()
                    .is_some_and(|e| var_has_return_move(ctx, e, var_name))
                || finalbody
                    .as_ref()
                    .is_some_and(|e| var_has_return_move(ctx, e, var_name))
        }
        HirStmt::With { body, .. } => var_has_return_move(ctx, body, var_name),
        _ => false,
    }
}

fn var_has_non_attribute_assignment(
    ctx: &CodeGenContext,
    stmts: &[HirStmt],
    var_name: &str,
) -> bool {
    stmts
        .iter()
        .any(|s| stmt_has_non_attribute_assignment(ctx, s, var_name))
}

fn stmt_has_non_attribute_assignment(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Assign {
            target,
            value,
            type_annotation: _,
        } => {
            if let AssignTarget::Symbol(name) = target {
                let is_empty_init = is_empty_collection_init(value);
                if name == var_name && !is_attribute_sourced(value) && !is_empty_init {
                    return true;
                }
            }
            false
        }
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_has_non_attribute_assignment(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .is_some_and(|e| var_has_non_attribute_assignment(ctx, e, var_name))
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_has_non_attribute_assignment(ctx, body, var_name)
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            var_has_non_attribute_assignment(ctx, body, var_name)
                || handlers
                    .iter()
                    .any(|h| var_has_non_attribute_assignment(ctx, &h.body, var_name))
                || orelse
                    .as_ref()
                    .is_some_and(|e| var_has_non_attribute_assignment(ctx, e, var_name))
                || finalbody
                    .as_ref()
                    .is_some_and(|e| var_has_non_attribute_assignment(ctx, e, var_name))
        }
        HirStmt::With { body, .. } => var_has_non_attribute_assignment(ctx, body, var_name),
        _ => false,
    }
}

fn var_has_move_use(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts.iter().any(|s| stmt_has_move_use(ctx, s, var_name))
}

fn stmt_has_move_use(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Return(Some(expr)) => expr_is_var_move(expr, var_name),
        HirStmt::Assign { value, .. } => expr_has_var_as_call_arg(value, var_name),
        HirStmt::Expr(expr) => expr_has_var_as_call_arg(expr, var_name),
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_has_move_use(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .map(|e| var_has_move_use(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_has_move_use(ctx, body, var_name)
        }
        HirStmt::Raise { exception, .. } => exception
            .as_ref()
            .map(|e| expr_is_var_move(e, var_name))
            .unwrap_or(false),
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            var_has_move_use(ctx, body, var_name)
                || handlers
                    .iter()
                    .any(|h| var_has_move_use(ctx, &h.body, var_name))
                || orelse
                    .as_ref()
                    .map(|e| var_has_move_use(ctx, e, var_name))
                    .unwrap_or(false)
                || finalbody
                    .as_ref()
                    .map(|e| var_has_move_use(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::With { body, .. } => var_has_move_use(ctx, body, var_name),
        _ => false,
    }
}

fn expr_is_var_move(expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Var(name) => name == var_name,
        HirExpr::IfExpr { body, orelse, .. } => {
            expr_is_var_move(body, var_name) || expr_is_var_move(orelse, var_name)
        }
        _ => false,
    }
}

fn expr_has_var_as_call_arg(expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Call { args, kwargs, .. } => {
            args.iter().any(|a| expr_is_var_move(a, var_name))
                || kwargs.iter().any(|(_, v)| expr_is_var_move(v, var_name))
        }
        HirExpr::MethodCall { args, kwargs, .. } => {
            args.iter().any(|a| expr_is_var_move(a, var_name))
                || kwargs.iter().any(|(_, v)| expr_is_var_move(v, var_name))
        }
        _ => false,
    }
}

fn var_has_mut_use(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts.iter().any(|s| stmt_has_mut_use(ctx, s, var_name))
}

fn stmt_has_mut_use(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Expr(expr) => expr_is_mutating_method_call(ctx, expr, var_name),
        HirStmt::Assign { target, .. } => assign_target_mutates_var(target, var_name),
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            var_has_mut_use(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .map(|e| var_has_mut_use(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
            var_has_mut_use(ctx, body, var_name)
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            var_has_mut_use(ctx, body, var_name)
                || handlers
                    .iter()
                    .any(|h| var_has_mut_use(ctx, &h.body, var_name))
                || orelse
                    .as_ref()
                    .map(|e| var_has_mut_use(ctx, e, var_name))
                    .unwrap_or(false)
                || finalbody
                    .as_ref()
                    .map(|e| var_has_mut_use(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::With { body, .. } => var_has_mut_use(ctx, body, var_name),
        _ => false,
    }
}

fn assign_target_mutates_var(target: &AssignTarget, var_name: &str) -> bool {
    match target {
        AssignTarget::Symbol(_) => false,
        AssignTarget::Attribute { value, .. } => expr_is_or_contains_var(value, var_name),
        AssignTarget::Index { base, .. } => expr_is_or_contains_var(base, var_name),
        AssignTarget::Slice { base, .. } => expr_is_or_contains_var(base, var_name),
        AssignTarget::Tuple(targets) => targets
            .iter()
            .any(|t| assign_target_mutates_var(t, var_name)),
        AssignTarget::Starred(_) => false,
    }
}

fn expr_is_or_contains_var(expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Var(name) => name == var_name,
        HirExpr::Attribute { value, .. } => expr_is_or_contains_var(value, var_name),
        HirExpr::Index { base, .. } => expr_is_or_contains_var(base, var_name),
        _ => false,
    }
}

fn expr_is_mutating_method_call(ctx: &CodeGenContext, expr: &HirExpr, var_name: &str) -> bool {
    if let HirExpr::MethodCall { object, method, .. } = expr {
        if let HirExpr::Var(name) = object.as_ref() {
            if name == var_name && is_mutating_method(method) {
                return true;
            }
        }
    }
    false
}

fn is_mutating_method(method: &str) -> bool {
    matches!(
        method,
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
            | "push"
            | "push_back"
            | "push_front"
            | "pop_back"
            | "pop_front"
    )
}

// ============================================================================
// Private helpers – closure capture detection
// ============================================================================

fn var_captured_in_closure(ctx: &CodeGenContext, stmts: &[HirStmt], var_name: &str) -> bool {
    stmts
        .iter()
        .any(|s| stmt_has_closure_capture(ctx, s, var_name))
}

fn stmt_has_closure_capture(ctx: &CodeGenContext, stmt: &HirStmt, var_name: &str) -> bool {
    match stmt {
        HirStmt::Assign { value, .. } => expr_has_closure_capture(value, var_name),
        HirStmt::Expr(expr) => expr_has_closure_capture(expr, var_name),
        HirStmt::Return(Some(expr)) => expr_has_closure_capture(expr, var_name),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_has_closure_capture(condition, var_name)
                || var_captured_in_closure(ctx, then_body, var_name)
                || else_body
                    .as_ref()
                    .map(|e| var_captured_in_closure(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::While { condition, body } => {
            expr_has_closure_capture(condition, var_name)
                || var_captured_in_closure(ctx, body, var_name)
        }
        HirStmt::For { iter, body, .. } => {
            expr_has_closure_capture(iter, var_name) || var_captured_in_closure(ctx, body, var_name)
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            var_captured_in_closure(ctx, body, var_name)
                || handlers
                    .iter()
                    .any(|h| var_captured_in_closure(ctx, &h.body, var_name))
                || orelse
                    .as_ref()
                    .map(|e| var_captured_in_closure(ctx, e, var_name))
                    .unwrap_or(false)
                || finalbody
                    .as_ref()
                    .map(|e| var_captured_in_closure(ctx, e, var_name))
                    .unwrap_or(false)
        }
        HirStmt::With { context, body, .. } => {
            expr_has_closure_capture(context, var_name)
                || var_captured_in_closure(ctx, body, var_name)
        }
        _ => false,
    }
}

fn expr_has_closure_capture(expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Lambda { body, params } => {
            !params.contains(&var_name.to_string()) && expr_references_var(body, var_name)
        }
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
            expr_has_closure_capture(element, var_name)
                || expr_has_closure_capture(iter, var_name)
                || condition
                    .as_ref()
                    .map(|c| expr_has_closure_capture(c, var_name))
                    .unwrap_or(false)
        }
        HirExpr::Binary { left, right, .. } => {
            expr_has_closure_capture(left, var_name) || expr_has_closure_capture(right, var_name)
        }
        HirExpr::Call { args, kwargs, .. } => {
            args.iter().any(|a| expr_has_closure_capture(a, var_name))
                || kwargs
                    .iter()
                    .any(|(_, v)| expr_has_closure_capture(v, var_name))
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            expr_has_closure_capture(object, var_name)
                || args.iter().any(|a| expr_has_closure_capture(a, var_name))
                || kwargs
                    .iter()
                    .any(|(_, v)| expr_has_closure_capture(v, var_name))
        }
        _ => false,
    }
}

fn expr_references_var(expr: &HirExpr, var_name: &str) -> bool {
    match expr {
        HirExpr::Var(name) => name == var_name,
        HirExpr::Binary { left, right, .. } => {
            expr_references_var(left, var_name) || expr_references_var(right, var_name)
        }
        HirExpr::Unary { operand, .. } => expr_references_var(operand, var_name),
        HirExpr::Call { args, kwargs, .. } => {
            args.iter().any(|a| expr_references_var(a, var_name))
                || kwargs.iter().any(|(_, v)| expr_references_var(v, var_name))
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            expr_references_var(object, var_name)
                || args.iter().any(|a| expr_references_var(a, var_name))
                || kwargs.iter().any(|(_, v)| expr_references_var(v, var_name))
        }
        HirExpr::Attribute { value, .. } => expr_references_var(value, var_name),
        HirExpr::Index { base, index } => {
            expr_references_var(base, var_name) || expr_references_var(index, var_name)
        }
        HirExpr::List(elts) | HirExpr::Tuple(elts) | HirExpr::Set(elts) => {
            elts.iter().any(|e| expr_references_var(e, var_name))
        }
        HirExpr::Dict(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_references_var(k, var_name) || expr_references_var(v, var_name)),
        HirExpr::IfExpr { test, body, orelse } => {
            expr_references_var(test, var_name)
                || expr_references_var(body, var_name)
                || expr_references_var(orelse, var_name)
        }
        _ => false,
    }
}
