//! Tail-call elimination pass — `Optimizer`.
//!
//! Converts tail-recursive Python functions into iterative `while true` loops
//! to avoid stack overflows and allow Rust to optimise the generated code
//! without relying on TCO (which Rust does not guarantee).
//!
//! ## Algorithm
//!
//! 1. Check every `Return` in the function body.  If *all* returns are either
//!    a) a non-recursive base-case return, or
//!    b) `Return(Some(Call { func: fn_name, args }))` (a tail call),
//!    and there is at least one tail call, the function is eligible.
//! 2. Rewrite:
//!    - Wrap the original body in `While { condition: Literal(Bool(true)), body }`.
//!    - Replace every tail call `Return(Call { func: fn_name, args })` with:
//!      - Temporary assignments `let __tce_N = argN;` (to avoid aliasing),
//!      - Reassignments of the function parameters from the temps,
//!      - `Continue { label: None }`.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, HirExpr, HirModule, HirStmt, Literal};

impl Optimizer {
    /// Entry point: run tail-call elimination over every function in `program`.
    pub(crate) fn eliminate_tail_calls_program(&self, mut program: HirModule) -> HirModule {
        // Collect function names first to avoid borrow issues.
        let names: Vec<String> = program.functions.iter().map(|f| f.name.clone()).collect();

        for (func, name) in program.functions.iter_mut().zip(names.iter()) {
            if !is_tail_recursive(&func.body, name) {
                continue;
            }

            let params: Vec<String> = func.params.iter().map(|p| p.name.clone()).collect();
            let new_body = rewrite_tail_calls(func.body.clone(), name, &params);
            // Wrap in `while true { … }`
            func.body = vec![HirStmt::While {
                condition: HirExpr::Literal(Literal::Bool(true)),
                body: new_body,
            }];
        }

        program
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the function body contains at least one tail call to
/// `fn_name` and every return is either a base-case return or a tail call.
fn is_tail_recursive(body: &[HirStmt], fn_name: &str) -> bool {
    let mut has_tail_call = false;
    for stmt in body {
        if !check_stmt(stmt, fn_name, &mut has_tail_call) {
            return false;
        }
    }
    has_tail_call
}

/// Returns `false` if a disqualifying pattern is found.
fn check_stmt(stmt: &HirStmt, fn_name: &str, has_tail_call: &mut bool) -> bool {
    match stmt {
        HirStmt::Return(Some(expr)) => {
            if is_call_to(expr, fn_name) {
                *has_tail_call = true;
            }
            // Any return is fine (base case or tail call).
            true
        }
        HirStmt::Return(None) => true,
        HirStmt::If {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                if !check_stmt(s, fn_name, has_tail_call) {
                    return false;
                }
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    if !check_stmt(s, fn_name, has_tail_call) {
                        return false;
                    }
                }
            }
            true
        }
        // Recursive calls inside non-return position → not a proper tail call.
        HirStmt::Assign { value, .. } => {
            if contains_call_to(value, fn_name) {
                return false;
            }
            true
        }
        HirStmt::Expr(expr) => {
            if contains_call_to(expr, fn_name) {
                return false;
            }
            true
        }
        _ => true,
    }
}

fn is_call_to(expr: &HirExpr, fn_name: &str) -> bool {
    matches!(expr, HirExpr::Call { func, .. } if func == fn_name)
}

fn contains_call_to(expr: &HirExpr, fn_name: &str) -> bool {
    match expr {
        HirExpr::Call { func, args, .. } => {
            func == fn_name || args.iter().any(|a| contains_call_to(a, fn_name))
        }
        HirExpr::Binary { left, right, .. } => {
            contains_call_to(left, fn_name) || contains_call_to(right, fn_name)
        }
        HirExpr::Unary { operand, .. } => contains_call_to(operand, fn_name),
        _ => false,
    }
}

/// Rewrite a function body by replacing tail calls with loop-continue blocks.
fn rewrite_tail_calls(stmts: Vec<HirStmt>, fn_name: &str, params: &[String]) -> Vec<HirStmt> {
    stmts
        .into_iter()
        .flat_map(|stmt| rewrite_stmt(stmt, fn_name, params))
        .collect()
}

fn rewrite_stmt(stmt: HirStmt, fn_name: &str, params: &[String]) -> Vec<HirStmt> {
    match stmt {
        HirStmt::Return(Some(expr)) if is_call_to(&expr, fn_name) => {
            // Extract arguments from the tail call.
            let args = match expr {
                HirExpr::Call { args, .. } => args,
                _ => unreachable!(),
            };

            let mut result = Vec::new();
            // Assign each argument to a temporary to avoid aliasing.
            for (i, arg) in args.iter().enumerate() {
                result.push(HirStmt::Assign {
                    target: AssignTarget::Symbol(format!("__tce_{i}")),
                    value: arg.clone(),
                    type_annotation: None,
                });
            }
            // Assign temporaries to the original parameters.
            for (i, param) in params.iter().enumerate() {
                if i < args.len() {
                    result.push(HirStmt::Assign {
                        target: AssignTarget::Symbol(param.clone()),
                        value: HirExpr::Var(format!("__tce_{i}")),
                        type_annotation: None,
                    });
                }
            }
            result.push(HirStmt::Continue { label: None });
            result
        }
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            vec![HirStmt::If {
                condition,
                then_body: rewrite_tail_calls(then_body, fn_name, params),
                else_body: else_body.map(|e| rewrite_tail_calls(e, fn_name, params)),
            }]
        }
        other => vec![other],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::TranspilationAnnotations;
    use crate::hir::{BinOp, FunctionProperties, HirFunction, HirParam, Literal, Type};
    use crate::optimizations::optimizer::{Optimizer, OptimizerConfig};

    fn make_module(func: HirFunction) -> HirModule {
        HirModule {
            functions: vec![func],
            imports: vec![],
            type_aliases: vec![],
            protocols: vec![],
            classes: vec![],
            constants: vec![],
            statements: vec![],
        }
    }

    fn make_func(name: &str, params: Vec<HirParam>, body: Vec<HirStmt>) -> HirFunction {
        HirFunction {
            name: name.to_string(),
            params,
            ret_type: Type::Int,
            body,
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_tail_recursive_function_is_rewritten() {
        // fn factorial(n, acc) { if n <= 1 { return acc; } else { return factorial(n-1, n*acc); } }
        let body = vec![HirStmt::If {
            condition: HirExpr::Binary {
                op: BinOp::LtEq,
                left: Box::new(HirExpr::Var("n".to_string())),
                right: Box::new(HirExpr::Literal(Literal::Int(1))),
            },
            then_body: vec![HirStmt::Return(Some(HirExpr::Var("acc".to_string())))],
            else_body: Some(vec![HirStmt::Return(Some(HirExpr::Call {
                func: "factorial".to_string(),
                args: vec![
                    HirExpr::Binary {
                        op: BinOp::Sub,
                        left: Box::new(HirExpr::Var("n".to_string())),
                        right: Box::new(HirExpr::Literal(Literal::Int(1))),
                    },
                    HirExpr::Binary {
                        op: BinOp::Mul,
                        left: Box::new(HirExpr::Var("n".to_string())),
                        right: Box::new(HirExpr::Var("acc".to_string())),
                    },
                ],
                kwargs: vec![],
                type_params: vec![],
            }))]),
        }];

        let func = make_func(
            "factorial",
            vec![
                HirParam::new("n".to_string(), Type::Int),
                HirParam::new("acc".to_string(), Type::Int),
            ],
            body,
        );
        let module = make_module(func);
        let optimizer = Optimizer::new(OptimizerConfig::default());
        let result = optimizer.eliminate_tail_calls_program(module);

        // Body should now be a single `while true { … }` statement.
        assert_eq!(result.functions[0].body.len(), 1);
        assert!(matches!(
            &result.functions[0].body[0],
            HirStmt::While {
                condition: HirExpr::Literal(Literal::Bool(true)),
                ..
            }
        ));
    }

    #[test]
    fn test_non_recursive_function_is_unchanged() {
        let body = vec![HirStmt::Return(Some(HirExpr::Var("x".to_string())))];
        let func = make_func(
            "identity",
            vec![HirParam::new("x".to_string(), Type::Int)],
            body.clone(),
        );
        let module = make_module(func);
        let optimizer = Optimizer::new(OptimizerConfig::default());
        let result = optimizer.eliminate_tail_calls_program(module);
        assert_eq!(result.functions[0].body, body);
    }
}
