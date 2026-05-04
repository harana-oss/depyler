//! Clone-elimination pass.
//!
//! Implements [`Optimizer`] methods for the `eliminate_clones` family.
//!
//! ## Strategy
//!
//! A `.clone()` call whose source variable is **never read after** the current
//! statement is unnecessary: the value can simply be *moved* instead of copied.
//! The pass:
//!
//! 1. Performs a single linear scan of the function body to build a
//!    `last_read` map: `variable → index of last statement that reads it`.
//! 2. Walks the statements a second time.  Whenever it finds a
//!    `HirExpr::MethodCall { method: "clone", args: [], object: Var(name) }`
//!    inside statement `i`, and `last_read[name] == i`, the clone is
//!    replaced by `Var(name)` — a move.
//!
//! The pass is conservative: it only eliminates clones on simple variable
//! expressions (`HirExpr::Var`). Clones on field accesses, index expressions,
//! or other compound sources are left untouched.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, HirExpr, HirModule, HirStmt};
use std::collections::HashMap;

impl Optimizer {
    /// Entry point: run clone elimination over every function in `program`.
    pub(crate) fn eliminate_clones_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            let last_read = last_read_indices(&func.body);
            eliminate_clones_in_body(&mut func.body, &last_read);
        }
        program
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a map from variable name to the index of the *last* statement
/// (at the top-level of `body`) that reads it.
fn last_read_indices(body: &[HirStmt]) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for (idx, stmt) in body.iter().enumerate() {
        collect_reads_stmt(stmt, idx, &mut map);
    }
    map
}

fn collect_reads_stmt(stmt: &HirStmt, idx: usize, map: &mut HashMap<String, usize>) {
    match stmt {
        HirStmt::Assign { target, value, .. } => {
            collect_reads_assign_target(target, idx, map);
            collect_reads_expr(value, idx, map);
        }
        HirStmt::Return(Some(expr)) => collect_reads_expr(expr, idx, map),
        HirStmt::Return(None) => {}
        HirStmt::Expr(expr) => collect_reads_expr(expr, idx, map),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_reads_expr(condition, idx, map);
            // Treat nested statements as occurring at the same top-level index
            // so we stay conservative (never remove a clone that might be needed
            // in a branch taken at runtime).
            for s in then_body {
                collect_reads_stmt(s, idx, map);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    collect_reads_stmt(s, idx, map);
                }
            }
        }
        HirStmt::While { condition, body } => {
            collect_reads_expr(condition, idx, map);
            for s in body {
                collect_reads_stmt(s, idx, map);
            }
        }
        HirStmt::For { iter, body, .. } => {
            collect_reads_expr(iter, idx, map);
            for s in body {
                collect_reads_stmt(s, idx, map);
            }
        }
        HirStmt::Raise {
            exception,
            cause,
        } => {
            if let Some(e) = exception {
                collect_reads_expr(e, idx, map);
            }
            if let Some(c) = cause {
                collect_reads_expr(c, idx, map);
            }
        }
        HirStmt::With {
            context, body, ..
        } => {
            collect_reads_expr(context, idx, map);
            for s in body {
                collect_reads_stmt(s, idx, map);
            }
        }
        HirStmt::Try {
            body,
            handlers,
            orelse,
            finalbody,
        } => {
            for s in body {
                collect_reads_stmt(s, idx, map);
            }
            for h in handlers {
                for s in &h.body {
                    collect_reads_stmt(s, idx, map);
                }
            }
            if let Some(or_stmts) = orelse {
                for s in or_stmts {
                    collect_reads_stmt(s, idx, map);
                }
            }
            if let Some(fin_stmts) = finalbody {
                for s in fin_stmts {
                    collect_reads_stmt(s, idx, map);
                }
            }
        }
        HirStmt::Assert { test, msg } => {
            collect_reads_expr(test, idx, map);
            if let Some(m) = msg {
                collect_reads_expr(m, idx, map);
            }
        }
        // Break / Continue / Pass / Global / Nonlocal / FunctionDef —
        // none of these read runtime variable values.
        _ => {}
    }
}

fn collect_reads_assign_target(
    target: &AssignTarget,
    idx: usize,
    map: &mut HashMap<String, usize>,
) {
    match target {
        AssignTarget::Symbol(_) => {}
        AssignTarget::Index { base, index } => {
            collect_reads_expr(base, idx, map);
            collect_reads_expr(index, idx, map);
        }
        AssignTarget::Slice {
            base,
            start,
            stop,
            step,
        } => {
            collect_reads_expr(base, idx, map);
            for e in [start, stop, step].iter().filter_map(|o| o.as_ref()) {
                collect_reads_expr(e, idx, map);
            }
        }
        AssignTarget::Attribute { value, .. } => collect_reads_expr(value, idx, map),
        AssignTarget::Tuple(targets) => {
            for t in targets {
                collect_reads_assign_target(t, idx, map);
            }
        }
        AssignTarget::Starred(_) => {}
    }
}

fn collect_reads_expr(expr: &HirExpr, idx: usize, map: &mut HashMap<String, usize>) {
    match expr {
        HirExpr::Var(name) => {
            map.insert(name.clone(), idx);
        }
        HirExpr::Binary { left, right, .. } => {
            collect_reads_expr(left, idx, map);
            collect_reads_expr(right, idx, map);
        }
        HirExpr::Unary { operand, .. } => collect_reads_expr(operand, idx, map),
        HirExpr::Call { args, kwargs, .. } => {
            for a in args {
                collect_reads_expr(a, idx, map);
            }
            for (_, v) in kwargs {
                collect_reads_expr(v, idx, map);
            }
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            collect_reads_expr(object, idx, map);
            for a in args {
                collect_reads_expr(a, idx, map);
            }
            for (_, v) in kwargs {
                collect_reads_expr(v, idx, map);
            }
        }
        HirExpr::Index { base, index } => {
            collect_reads_expr(base, idx, map);
            collect_reads_expr(index, idx, map);
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            collect_reads_expr(base, idx, map);
            for e in [start, stop, step].iter().filter_map(|o| o.as_ref()) {
                collect_reads_expr(e, idx, map);
            }
        }
        HirExpr::Attribute { value, .. } => collect_reads_expr(value, idx, map),
        HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) => {
            for item in items {
                collect_reads_expr(item, idx, map);
            }
        }
        HirExpr::FrozenSet(items) => {
            for item in items {
                collect_reads_expr(item, idx, map);
            }
        }
        HirExpr::Dict(pairs) => {
            for (k, v) in pairs {
                collect_reads_expr(k, idx, map);
                collect_reads_expr(v, idx, map);
            }
        }
        HirExpr::Borrow { expr, .. } => collect_reads_expr(expr, idx, map),
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        } => {
            collect_reads_expr(element, idx, map);
            collect_reads_expr(iter, idx, map);
            if let Some(c) = condition {
                collect_reads_expr(c, idx, map);
            }
        }
        HirExpr::Await { value: inner } => collect_reads_expr(inner, idx, map),
        // Literals, None, Bool, etc. — no variable reads
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Elimination
// ---------------------------------------------------------------------------

/// Walk `body` and replace `var.clone()` with `var` wherever the clone is
/// provably the last read of that variable.
fn eliminate_clones_in_body(body: &mut Vec<HirStmt>, last_read: &HashMap<String, usize>) {
    for (idx, stmt) in body.iter_mut().enumerate() {
        eliminate_clones_in_stmt(stmt, idx, last_read);
    }
}

fn eliminate_clones_in_stmt(
    stmt: &mut HirStmt,
    idx: usize,
    last_read: &HashMap<String, usize>,
) {
    match stmt {
        HirStmt::Assign { value, .. } => eliminate_clones_in_expr(value, idx, last_read),
        HirStmt::Return(Some(expr)) => eliminate_clones_in_expr(expr, idx, last_read),
        HirStmt::Expr(expr) => eliminate_clones_in_expr(expr, idx, last_read),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            eliminate_clones_in_expr(condition, idx, last_read);
            // Use the same top-level index for nested bodies (conservative).
            for s in then_body.iter_mut() {
                eliminate_clones_in_stmt(s, idx, last_read);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts.iter_mut() {
                    eliminate_clones_in_stmt(s, idx, last_read);
                }
            }
        }
        HirStmt::While { condition, body } => {
            eliminate_clones_in_expr(condition, idx, last_read);
            for s in body.iter_mut() {
                eliminate_clones_in_stmt(s, idx, last_read);
            }
        }
        HirStmt::For { iter, body, .. } => {
            eliminate_clones_in_expr(iter, idx, last_read);
            for s in body.iter_mut() {
                eliminate_clones_in_stmt(s, idx, last_read);
            }
        }
        HirStmt::With { context, body, .. } => {
            eliminate_clones_in_expr(context, idx, last_read);
            for s in body.iter_mut() {
                eliminate_clones_in_stmt(s, idx, last_read);
            }
        }
        _ => {}
    }
}

/// Recursively walk `expr` and replace `Var(name).clone()` with `Var(name)`
/// when statement `idx` is the last read of `name`.
fn eliminate_clones_in_expr(
    expr: &mut HirExpr,
    idx: usize,
    last_read: &HashMap<String, usize>,
) {
    // Check if *this* node is an eliminable clone before recursing, so we
    // don't accidentally recurse into the object we're about to replace.
    if is_eliminable_clone(expr, idx, last_read) {
        // Safety: we just matched MethodCall { object: Var(_), .. }
        if let HirExpr::MethodCall { object, .. } = expr {
            let inner = std::mem::replace(object.as_mut(), HirExpr::Literal(crate::hir::Literal::None));
            *expr = inner;
        }
        return;
    }

    // Otherwise recurse.
    match expr {
        HirExpr::Binary { left, right, .. } => {
            eliminate_clones_in_expr(left, idx, last_read);
            eliminate_clones_in_expr(right, idx, last_read);
        }
        HirExpr::Unary { operand, .. } => eliminate_clones_in_expr(operand, idx, last_read),
        HirExpr::Call { args, kwargs, .. } => {
            for a in args.iter_mut() {
                eliminate_clones_in_expr(a, idx, last_read);
            }
            for (_, v) in kwargs.iter_mut() {
                eliminate_clones_in_expr(v, idx, last_read);
            }
        }
        HirExpr::MethodCall {
            object,
            args,
            kwargs,
            ..
        } => {
            eliminate_clones_in_expr(object, idx, last_read);
            for a in args.iter_mut() {
                eliminate_clones_in_expr(a, idx, last_read);
            }
            for (_, v) in kwargs.iter_mut() {
                eliminate_clones_in_expr(v, idx, last_read);
            }
        }
        HirExpr::Index { base, index } => {
            eliminate_clones_in_expr(base, idx, last_read);
            eliminate_clones_in_expr(index, idx, last_read);
        }
        HirExpr::Slice {
            base,
            start,
            stop,
            step,
        } => {
            eliminate_clones_in_expr(base, idx, last_read);
            for e in [start, stop, step].iter_mut().filter_map(|o| o.as_mut()) {
                eliminate_clones_in_expr(e, idx, last_read);
            }
        }
        HirExpr::Attribute { value, .. } => eliminate_clones_in_expr(value, idx, last_read),
        HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) => {
            for item in items.iter_mut() {
                eliminate_clones_in_expr(item, idx, last_read);
            }
        }
        HirExpr::FrozenSet(items) => {
            for item in items.iter_mut() {
                eliminate_clones_in_expr(item, idx, last_read);
            }
        }
        HirExpr::Dict(pairs) => {
            for (k, v) in pairs.iter_mut() {
                eliminate_clones_in_expr(k, idx, last_read);
                eliminate_clones_in_expr(v, idx, last_read);
            }
        }
        HirExpr::Borrow { expr: inner, .. } => eliminate_clones_in_expr(inner, idx, last_read),
        HirExpr::ListComp {
            element,
            iter,
            condition,
            ..
        } => {
            eliminate_clones_in_expr(element, idx, last_read);
            eliminate_clones_in_expr(iter, idx, last_read);
            if let Some(c) = condition {
                eliminate_clones_in_expr(c, idx, last_read);
            }
        }
        HirExpr::Await { value: inner } => eliminate_clones_in_expr(inner, idx, last_read),
        _ => {}
    }
}

/// Returns `true` when `expr` is of the form `Var(name).clone()` and
/// statement `idx` is the last statement that reads `name`.
fn is_eliminable_clone(
    expr: &HirExpr,
    idx: usize,
    last_read: &HashMap<String, usize>,
) -> bool {
    if let HirExpr::MethodCall {
        object,
        method,
        args,
        kwargs,
        ..
    } = expr
    {
        if method == "clone" && args.is_empty() && kwargs.is_empty() {
            if let HirExpr::Var(name) = object.as_ref() {
                return last_read.get(name).copied() == Some(idx);
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{AssignTarget, HirExpr, HirStmt, Literal};

    fn var(name: &str) -> HirExpr {
        HirExpr::Var(name.to_string())
    }

    fn clone_of(name: &str) -> HirExpr {
        HirExpr::MethodCall {
            object: Box::new(var(name)),
            method: "clone".to_string(),
            args: vec![],
            kwargs: vec![],
            type_params: vec![],
        }
    }

    fn assign(target: &str, value: HirExpr) -> HirStmt {
        HirStmt::Assign {
            target: AssignTarget::Symbol(target.to_string()),
            value,
            type_annotation: None,
        }
    }

    fn ret(expr: HirExpr) -> HirStmt {
        HirStmt::Return(Some(expr))
    }

    /// `a` is only read at statement 0 (the clone), so the clone is eliminated.
    #[test]
    fn eliminates_clone_when_last_read() {
        let mut body = vec![
            // let b = a.clone();   <- a's only read
            assign("b", clone_of("a")),
        ];
        let last_read = last_read_indices(&body);
        eliminate_clones_in_body(&mut body, &last_read);

        if let HirStmt::Assign { value, .. } = &body[0] {
            assert_eq!(value, &var("a"), "clone should have been eliminated");
        } else {
            panic!("expected Assign");
        }
    }

    /// `a` is read again at statement 1, so the clone at statement 0 must stay.
    #[test]
    fn preserves_clone_when_variable_used_later() {
        let mut body = vec![
            // let b = a.clone();
            assign("b", clone_of("a")),
            // return a;
            ret(var("a")),
        ];
        let last_read = last_read_indices(&body);
        eliminate_clones_in_body(&mut body, &last_read);

        if let HirStmt::Assign { value, .. } = &body[0] {
            assert!(
                matches!(value, HirExpr::MethodCall { method, .. } if method == "clone"),
                "clone should have been preserved"
            );
        } else {
            panic!("expected Assign");
        }
    }

    /// Two sequential clones: both `a` and `b` are only used once, so both
    /// clones are eliminated.
    #[test]
    fn eliminates_multiple_independent_clones() {
        let mut body = vec![
            assign("x", clone_of("a")),
            assign("y", clone_of("b")),
        ];
        let last_read = last_read_indices(&body);
        eliminate_clones_in_body(&mut body, &last_read);

        for (i, name) in [("x", "a"), ("y", "b")] {
            if let HirStmt::Assign { value, target: AssignTarget::Symbol(tgt), .. } =
                body.iter().find(|s| matches!(s, HirStmt::Assign { target: AssignTarget::Symbol(t), .. } if t == i)).unwrap()
            {
                assert_eq!(value, &var(name), "clone for {tgt} should be eliminated");
            }
        }
    }
}
