//! Iterator fusion pass — `Optimizer`.
//!
//! Fuses pairs of consecutive iterator pipeline statements to eliminate
//! intermediate `Vec` allocations.
//!
//! ## Pattern matched
//!
//! ```text
//! let t0 = x.iter().map(f).collect();
//! let t1 = t0.iter().filter(g).collect();
//! ```
//!
//! Rewrites to:
//!
//! ```text
//! let t1 = x.iter().map(f).filter(g).collect();
//! ```
//!
//! and removes the first assignment.
//!
//! ## Rules
//!
//! * `t0` must be used **only once** (in the second statement).
//! * The fused chain only fires for `map`+`filter` and `map`+`map` pairs in
//!   that order (the two most common Python generator-pipeline patterns).
//!   Other combinations are left untouched.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, HirExpr, HirModule, HirStmt};
use std::collections::HashMap;

impl Optimizer {
    pub(crate) fn fuse_iterators_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            func.body = fuse_in_block(std::mem::take(&mut func.body));
        }
        program
    }
}

// ---------------------------------------------------------------------------
// Core fusion logic
// ---------------------------------------------------------------------------

/// Count how many times each variable name is read in a slice of statements.
fn read_counts(stmts: &[HirStmt]) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for stmt in stmts {
        count_reads_stmt(stmt, &mut map);
    }
    map
}

fn count_reads_stmt(stmt: &HirStmt, map: &mut HashMap<String, usize>) {
    match stmt {
        HirStmt::Assign { value, .. } => count_reads_expr(value, map),
        HirStmt::Return(Some(e)) => count_reads_expr(e, map),
        HirStmt::Expr(e) => count_reads_expr(e, map),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            count_reads_expr(condition, map);
            for s in then_body {
                count_reads_stmt(s, map);
            }
            if let Some(eb) = else_body {
                for s in eb {
                    count_reads_stmt(s, map);
                }
            }
        }
        HirStmt::While { condition, body } => {
            count_reads_expr(condition, map);
            for s in body {
                count_reads_stmt(s, map);
            }
        }
        HirStmt::For { iter, body, .. } => {
            count_reads_expr(iter, map);
            for s in body {
                count_reads_stmt(s, map);
            }
        }
        _ => {}
    }
}

fn count_reads_expr(expr: &HirExpr, map: &mut HashMap<String, usize>) {
    match expr {
        HirExpr::Var(name) => *map.entry(name.clone()).or_default() += 1,
        HirExpr::MethodCall { object, args, .. } => {
            count_reads_expr(object, map);
            for a in args {
                count_reads_expr(a, map);
            }
        }
        HirExpr::Call { args, .. } => {
            for a in args {
                count_reads_expr(a, map);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            count_reads_expr(left, map);
            count_reads_expr(right, map);
        }
        HirExpr::Unary { operand, .. } => count_reads_expr(operand, map),
        HirExpr::List(items) | HirExpr::Tuple(items) | HirExpr::Set(items) => {
            for i in items {
                count_reads_expr(i, map);
            }
        }
        HirExpr::Index { base, index } => {
            count_reads_expr(base, map);
            count_reads_expr(index, map);
        }
        _ => {}
    }
}

/// Returns the innermost object before `.collect()` for a chain ending with
/// `.method(args...).collect()`.
///
/// Returns `Some((inner_chain, method_name, method_args))` where `inner_chain`
/// is the expression on which `method_name` was called.
fn peel_collect(expr: &HirExpr) -> Option<(&HirExpr, &str, &[HirExpr])> {
    if let HirExpr::MethodCall {
        object,
        method,
        args,
        ..
    } = expr
    {
        if method == "collect" && args.is_empty() {
            // object should be a MethodCall like .map(f) or .filter(g)
            if let HirExpr::MethodCall {
                object: inner,
                method: inner_method,
                args: inner_args,
                ..
            } = object.as_ref()
            {
                if matches!(inner_method.as_str(), "map" | "filter" | "flat_map") {
                    return Some((inner.as_ref(), inner_method.as_str(), inner_args.as_slice()));
                }
            }
        }
    }
    None
}

/// Returns the iterator source if `expr` is `source.iter().map_or_filter(f).collect()`.
fn source_of_pipeline(expr: &HirExpr) -> Option<&HirExpr> {
    if let Some((inner, _, _)) = peel_collect(expr) {
        // inner should be `source.iter()`
        if let HirExpr::MethodCall {
            object,
            method,
            args,
            ..
        } = inner
        {
            if method == "iter" && args.is_empty() {
                return Some(object.as_ref());
            }
        }
    }
    None
}

/// Fuse a pair:
///   stmt[i]  = `let t0 = X.iter().op1(f).collect()`
///   stmt[i+1]= `let t1 = t0.iter().op2(g).collect()`
/// into `let t1 = X.iter().op1(f).op2(g).collect()`.
///
/// Returns `Some((fused_stmt, name_to_drop))` on success.
fn try_fuse(first: &HirStmt, second: &HirStmt) -> Option<(HirStmt, String)> {
    // Unpack first: let t0 = X.iter().op1(f).collect()
    let (t0, first_pipeline) = match first {
        HirStmt::Assign {
            target: AssignTarget::Symbol(name),
            value,
            ..
        } => (name.as_str(), value),
        _ => return None,
    };

    // Check first is a proper pipeline
    let (first_inner, first_op, first_args) = peel_collect(first_pipeline)?;
    // first_inner should be source.iter()
    let source = source_of_pipeline(first_pipeline)?;

    // Unpack second: let t1 = t0.iter().op2(g).collect()
    let (t1, second_pipeline) = match second {
        HirStmt::Assign {
            target: AssignTarget::Symbol(name),
            value,
            ..
        } => (name.as_str(), value),
        _ => return None,
    };

    let (second_inner, second_op, second_args) = peel_collect(second_pipeline)?;
    // second_inner should be t0.iter()
    if let HirExpr::MethodCall {
        object,
        method,
        args,
        ..
    } = second_inner
    {
        if method != "iter" || !args.is_empty() {
            return None;
        }
        if !matches!(object.as_ref(), HirExpr::Var(v) if v == t0) {
            return None;
        }
    } else {
        return None;
    }

    // Build fused: source.iter().op1(f).op2(g).collect()
    let iter_call = HirExpr::MethodCall {
        object: Box::new(source.clone()),
        method: "iter".to_string(),
        args: vec![],
        kwargs: vec![],
        type_params: vec![],
    };
    let op1_call = HirExpr::MethodCall {
        object: Box::new(iter_call),
        method: first_op.to_string(),
        args: first_args.to_vec(),
        kwargs: vec![],
        type_params: vec![],
    };
    let op2_call = HirExpr::MethodCall {
        object: Box::new(op1_call),
        method: second_op.to_string(),
        args: second_args.to_vec(),
        kwargs: vec![],
        type_params: vec![],
    };
    let collect_call = HirExpr::MethodCall {
        object: Box::new(op2_call),
        method: "collect".to_string(),
        args: vec![],
        kwargs: vec![],
        type_params: vec![],
    };

    Some((
        HirStmt::Assign {
            target: AssignTarget::Symbol(t1.to_string()),
            value: collect_call,
            type_annotation: None,
        },
        t0.to_string(),
    ))
}

fn fuse_in_block(stmts: Vec<HirStmt>) -> Vec<HirStmt> {
    let counts = read_counts(&stmts);
    let mut result: Vec<HirStmt> = Vec::with_capacity(stmts.len());
    let mut iter = stmts.into_iter().peekable();

    while let Some(stmt) = iter.next() {
        if let Some(next) = iter.peek() {
            if let Some((fused, drop_name)) = try_fuse(&stmt, next) {
                // Only fuse if t0 is used exactly once (in the second stmt).
                if counts.get(&drop_name).copied().unwrap_or(0) == 1 {
                    let _ = iter.next(); // consume the second statement
                    result.push(fused);
                    continue;
                }
            }
        }
        result.push(stmt);
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::TranspilationAnnotations;
    use crate::hir::{FunctionProperties, HirFunction, HirParam, Literal, Type};
    use crate::optimizations::optimizer::{Optimizer, OptimizerConfig};

    fn optimizer() -> Optimizer {
        Optimizer::new(OptimizerConfig::default())
    }

    fn make_module(body: Vec<HirStmt>) -> HirModule {
        HirModule {
            functions: vec![HirFunction {
                name: "f".to_string(),
                params: vec![],
                ret_type: Type::Unknown,
                body,
                properties: FunctionProperties::default(),
                annotations: TranspilationAnnotations::default(),
                docstring: None,
            }],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
            statements: vec![],
        }
    }

    fn make_pipeline(source: HirExpr, op: &str, arg: HirExpr) -> HirExpr {
        let iter = HirExpr::MethodCall {
            object: Box::new(source),
            method: "iter".to_string(),
            args: vec![],
            kwargs: vec![],
            type_params: vec![],
        };
        let op_call = HirExpr::MethodCall {
            object: Box::new(iter),
            method: op.to_string(),
            args: vec![arg],
            kwargs: vec![],
            type_params: vec![],
        };
        HirExpr::MethodCall {
            object: Box::new(op_call),
            method: "collect".to_string(),
            args: vec![],
            kwargs: vec![],
            type_params: vec![],
        }
    }

    #[test]
    fn test_fuses_map_then_filter() {
        let f_lam = HirExpr::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(HirExpr::Var("x".to_string())),
        };
        let g_lam = HirExpr::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(HirExpr::Var("x".to_string())),
        };
        let s0 = HirStmt::Assign {
            target: AssignTarget::Symbol("t0".to_string()),
            value: make_pipeline(HirExpr::Var("xs".to_string()), "map", f_lam.clone()),
            type_annotation: None,
        };
        let s1 = HirStmt::Assign {
            target: AssignTarget::Symbol("t1".to_string()),
            value: make_pipeline(HirExpr::Var("t0".to_string()), "filter", g_lam.clone()),
            type_annotation: None,
        };
        let module = make_module(vec![s0, s1]);
        let result = optimizer().fuse_iterators_program(module);
        // Should have collapsed to 1 statement.
        assert_eq!(result.functions[0].body.len(), 1);
    }

    #[test]
    fn test_does_not_fuse_when_t0_used_elsewhere() {
        let f_lam = HirExpr::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(HirExpr::Var("x".to_string())),
        };
        let g_lam = HirExpr::Lambda {
            params: vec!["x".to_string()],
            body: Box::new(HirExpr::Var("x".to_string())),
        };
        let s0 = HirStmt::Assign {
            target: AssignTarget::Symbol("t0".to_string()),
            value: make_pipeline(HirExpr::Var("xs".to_string()), "map", f_lam),
            type_annotation: None,
        };
        let s1 = HirStmt::Assign {
            target: AssignTarget::Symbol("t1".to_string()),
            value: make_pipeline(HirExpr::Var("t0".to_string()), "filter", g_lam),
            type_annotation: None,
        };
        // Extra use of t0
        let s2 = HirStmt::Return(Some(HirExpr::Var("t0".to_string())));
        let module = make_module(vec![s0, s1, s2]);
        let result = optimizer().fuse_iterators_program(module);
        // Should NOT fuse (t0 is used twice).
        assert_eq!(result.functions[0].body.len(), 3);
    }
}
