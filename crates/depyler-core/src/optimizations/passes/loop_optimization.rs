//! Loop and miscellaneous machine-level optimisation passes for [`PerformanceOptimizer`].
//!
//! Covers loop unrolling, vectorisation, function inlining, bounds-check
//! removal, and latency / throughput tuning stubs.

use super::super::optimizer::PerformanceOptimizer;
use crate::hir::{AssignTarget, HirExpr, HirFunction, HirParam, HirStmt};
use std::collections::HashMap;

impl PerformanceOptimizer {
    pub(crate) fn loop_unrolling(&mut self, stmts: &mut Vec<HirStmt>, factor: usize) {
        for stmt in stmts.iter_mut() {
            if let HirStmt::For { body, .. } = stmt {
                let original_body = body.clone();
                for _ in 1..factor {
                    body.extend(original_body.clone());
                }
            }
        }
        self.optimizations_applied
            .push(format!("loop_unrolling_{factor}"));
    }

    pub(crate) fn vectorize_loops(&mut self, _stmts: &mut [HirStmt]) {
        // P4-A: Record intent.  Full SIMD / std::simd lowering requires
        // target-feature detection that is out of scope for the HIR layer.
        // The code generator can inspect `annotations.performance_hints` for
        // `PerformanceHint::Vectorize` and emit `#[target_feature(enable = "avx2")]`.
        self.optimizations_applied
            .push("vectorize_loops".to_string());
    }

    /// P4-B — real implementation.
    ///
    /// Inlines calls to functions whose body is ≤ `threshold` statements.
    /// Substitutes argument expressions for parameter variables in the inlined
    /// body, then splices the body statements in place of the call-site
    /// `Assign` or `Expr` statement.
    pub(crate) fn inline_small_functions_with_map(
        &mut self,
        stmts: &mut Vec<HirStmt>,
        fn_map: &HashMap<String, &HirFunction>,
        threshold: usize,
    ) {
        let mut i = 0;
        while i < stmts.len() {
            // Try to inline a call inside an Assign or stand-alone Expr.
            let replacement = match &stmts[i] {
                HirStmt::Assign {
                    target,
                    value: HirExpr::Call { func, args, .. },
                    type_annotation,
                } => {
                    if let Some(callee) = fn_map.get(func.as_str()) {
                        if callee.body.len() <= threshold {
                            Some(build_inline_stmts(
                                callee,
                                args,
                                Some(target.clone()),
                                type_annotation.clone(),
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                HirStmt::Expr(HirExpr::Call { func, args, .. }) => {
                    if let Some(callee) = fn_map.get(func.as_str()) {
                        if callee.body.len() <= threshold {
                            Some(build_inline_stmts(callee, args, None, None))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(mut inlined) = replacement {
                // Recursively inline within the inlined body.
                self.inline_small_functions_with_map(&mut inlined, fn_map, threshold);
                let count = inlined.len();
                stmts.splice(i..=i, inlined);
                i += count;
            } else {
                i += 1;
            }
        }

        self.optimizations_applied
            .push("inline_small_functions".to_string());
    }

    pub(crate) fn inline_small_functions(&mut self, _stmts: &mut [HirStmt]) {
        // Stub: called from optimize_function which does not have access to the
        // full module function map.  Real inlining is performed by
        // `inline_small_functions_with_map` which is called from
        // `Optimizer::inline_small_functions_program` with a function-map
        // built from the full module.
        self.optimizations_applied
            .push("inline_small_functions".to_string());
    }

    pub(crate) fn remove_bounds_checks(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("remove_bounds_checks".to_string());
    }

    pub(crate) fn optimize_for_latency(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("optimize_for_latency".to_string());
    }

    pub(crate) fn optimize_for_throughput(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("optimize_for_throughput".to_string());
    }

    pub(crate) fn common_subexpression_elimination(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("common_subexpression_elimination".to_string());
    }
}

// ---------------------------------------------------------------------------
// Inline helpers
// ---------------------------------------------------------------------------

pub const INLINE_THRESHOLD: usize = 5;

/// Build the sequence of statements that replaces a call site when inlining
/// `callee` with the given `args`.
///
/// Strategy:
/// 1. Assign each argument to a fresh `__inline_<param>_<uid>` temporary.
/// 2. Copy the callee body, replacing every `Var(param)` with the temporary.
/// 3. If a `Return` statement is present and there is an assignment target,
///    replace the last `Return(Some(expr))` with `Assign { target, value: expr }`.
fn build_inline_stmts(
    callee: &HirFunction,
    args: &[HirExpr],
    target: Option<AssignTarget>,
    type_annotation: Option<crate::hir::Type>,
) -> Vec<HirStmt> {
    // Map param names → fresh temp names.
    let uid: u64 = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
    };

    let param_to_temp: HashMap<String, String> = callee
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let temp = format!("__inline_{}_{}", p.name, uid.wrapping_add(i as u64));
            (p.name.clone(), temp)
        })
        .collect();

    let mut result: Vec<HirStmt> = Vec::new();

    // Assign arguments to temporaries.
    for (param, arg) in callee.params.iter().zip(args.iter()) {
        if let Some(temp) = param_to_temp.get(&param.name) {
            result.push(HirStmt::Assign {
                target: AssignTarget::Symbol(temp.clone()),
                value: arg.clone(),
                type_annotation: None,
            });
        }
    }

    // Copy and rename the callee body.
    let mut body: Vec<HirStmt> = callee
        .body
        .iter()
        .map(|s| rename_stmt(s, &param_to_temp))
        .collect();

    // Redirect the last `Return` to the assignment target (if any).
    if let Some(tgt) = target {
        if let Some(last) = body.last_mut() {
            if let HirStmt::Return(Some(ret_expr)) = last {
                *last = HirStmt::Assign {
                    target: tgt,
                    value: ret_expr.clone(),
                    type_annotation,
                };
            }
        }
    }

    result.extend(body);
    result
}

fn rename_stmt(stmt: &HirStmt, map: &HashMap<String, String>) -> HirStmt {
    match stmt {
        HirStmt::Assign { target, value, type_annotation } => HirStmt::Assign {
            target: target.clone(),
            value: rename_expr(value, map),
            type_annotation: type_annotation.clone(),
        },
        HirStmt::Return(Some(e)) => HirStmt::Return(Some(rename_expr(e, map))),
        HirStmt::Return(None) => HirStmt::Return(None),
        HirStmt::If { condition, then_body, else_body } => HirStmt::If {
            condition: rename_expr(condition, map),
            then_body: then_body.iter().map(|s| rename_stmt(s, map)).collect(),
            else_body: else_body.as_ref().map(|eb| eb.iter().map(|s| rename_stmt(s, map)).collect()),
        },
        HirStmt::While { condition, body } => HirStmt::While {
            condition: rename_expr(condition, map),
            body: body.iter().map(|s| rename_stmt(s, map)).collect(),
        },
        HirStmt::For { target, iter, body } => HirStmt::For {
            target: target.clone(),
            iter: rename_expr(iter, map),
            body: body.iter().map(|s| rename_stmt(s, map)).collect(),
        },
        HirStmt::Expr(e) => HirStmt::Expr(rename_expr(e, map)),
        other => other.clone(),
    }
}

fn rename_expr(expr: &HirExpr, map: &HashMap<String, String>) -> HirExpr {
    match expr {
        HirExpr::Var(name) => {
            if let Some(renamed) = map.get(name) {
                HirExpr::Var(renamed.clone())
            } else {
                HirExpr::Var(name.clone())
            }
        }
        HirExpr::Binary { op, left, right } => HirExpr::Binary {
            op: *op,
            left: Box::new(rename_expr(left, map)),
            right: Box::new(rename_expr(right, map)),
        },
        HirExpr::Unary { op, operand } => HirExpr::Unary {
            op: *op,
            operand: Box::new(rename_expr(operand, map)),
        },
        HirExpr::Call { func, args, kwargs, type_params } => HirExpr::Call {
            func: func.clone(),
            args: args.iter().map(|a| rename_expr(a, map)).collect(),
            kwargs: kwargs
                .iter()
                .map(|(k, v)| (k.clone(), rename_expr(v, map)))
                .collect(),
            type_params: type_params.clone(),
        },
        HirExpr::MethodCall { object, method, args, kwargs, type_params } => HirExpr::MethodCall {
            object: Box::new(rename_expr(object, map)),
            method: method.clone(),
            args: args.iter().map(|a| rename_expr(a, map)).collect(),
            kwargs: kwargs
                .iter()
                .map(|(k, v)| (k.clone(), rename_expr(v, map)))
                .collect(),
            type_params: type_params.clone(),
        },
        HirExpr::Index { base, index } => HirExpr::Index {
            base: Box::new(rename_expr(base, map)),
            index: Box::new(rename_expr(index, map)),
        },
        HirExpr::List(items) => HirExpr::List(items.iter().map(|i| rename_expr(i, map)).collect()),
        HirExpr::Tuple(items) => HirExpr::Tuple(items.iter().map(|i| rename_expr(i, map)).collect()),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{FunctionProperties, HirParam, Literal, Type};
    use crate::annotations::TranspilationAnnotations;
    use crate::optimizations::optimizer::PerformanceOptimizer;

    fn make_callee(name: &str, param: &str, body: Vec<HirStmt>) -> HirFunction {
        HirFunction {
            name: name.to_string(),
            params: vec![HirParam::new(param.to_string(), Type::Int)],
            ret_type: Type::Int,
            body,
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_small_function_inlined() {
        // callee: def double(x): return x * 2
        let callee = make_callee(
            "double",
            "x",
            vec![HirStmt::Return(Some(HirExpr::Binary {
                op: crate::hir::BinOp::Mul,
                left: Box::new(HirExpr::Var("x".to_string())),
                right: Box::new(HirExpr::Literal(Literal::Int(2))),
            }))],
        );

        let mut fn_map: HashMap<String, &HirFunction> = HashMap::new();
        fn_map.insert("double".to_string(), &callee);

        // caller: r = double(5)
        let call_stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("r".to_string()),
            value: HirExpr::Call {
                func: "double".to_string(),
                args: vec![HirExpr::Literal(Literal::Int(5))],
                kwargs: vec![],
                type_params: vec![],
            },
            type_annotation: None,
        };

        let mut stmts = vec![call_stmt];
        let mut opt = PerformanceOptimizer::new();
        opt.inline_small_functions_with_map(&mut stmts, &fn_map, INLINE_THRESHOLD);

        // The call site should have been replaced with inlined stmts (≥ 2: arg assign + body).
        assert!(stmts.len() >= 2, "expected inlined statements, got {:?}", stmts);
    }

    #[test]
    fn test_large_function_not_inlined() {
        // callee with 6 statements (> threshold of 5)
        let body: Vec<HirStmt> = (0..6)
            .map(|i| HirStmt::Assign {
                target: AssignTarget::Symbol(format!("v{i}")),
                value: HirExpr::Literal(Literal::Int(i)),
                type_annotation: None,
            })
            .collect();
        let callee = make_callee("big", "x", body);

        let mut fn_map: HashMap<String, &HirFunction> = HashMap::new();
        fn_map.insert("big".to_string(), &callee);

        let call_stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("r".to_string()),
            value: HirExpr::Call {
                func: "big".to_string(),
                args: vec![HirExpr::Literal(Literal::Int(1))],
                kwargs: vec![],
                type_params: vec![],
            },
            type_annotation: None,
        };

        let mut stmts = vec![call_stmt];
        let mut opt = PerformanceOptimizer::new();
        opt.inline_small_functions_with_map(&mut stmts, &fn_map, INLINE_THRESHOLD);

        // Should still be a single call statement.
        assert_eq!(stmts.len(), 1);
    }
}

