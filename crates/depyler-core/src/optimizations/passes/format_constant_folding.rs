//! Format constant-folding pass — `Optimizer`.
//!
//! Folds compile-time-known `format`-equivalent HIR patterns into plain
//! `Literal::String` nodes, removing runtime formatting overhead.
//!
//! ## Patterns folded
//!
//! | Input HIR                                          | Output              |
//! |----------------------------------------------------|---------------------|
//! | `Call("format", ["{}", Literal::String(s)])`       | `Literal::String(s)`|
//! | `Call("format", ["{}{}", Lit::Str(a), Lit::Str(b)])`| `Literal::String(a+b)`|
//! | `Binary(Add, Literal::String(a), Literal::String(b))`| `Literal::String(a+b)`|
//! | `MethodCall(Literal::String(fmt), "format", [Literal::String(a)])` | folded |

use super::super::optimizer::Optimizer;
use crate::hir::{BinOp, HirExpr, HirModule, HirStmt, Literal};

impl Optimizer {
    pub(crate) fn fold_format_constants_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            fold_in_body(&mut func.body);
        }
        program
    }
}

fn fold_in_body(stmts: &mut Vec<HirStmt>) {
    for stmt in stmts.iter_mut() {
        fold_in_stmt(stmt);
    }
}

fn fold_in_stmt(stmt: &mut HirStmt) {
    match stmt {
        HirStmt::Assign { value, .. } => fold_expr(value),
        HirStmt::Return(Some(e)) => fold_expr(e),
        HirStmt::Expr(e) => fold_expr(e),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            fold_expr(condition);
            fold_in_body(then_body);
            if let Some(eb) = else_body {
                fold_in_body(eb);
            }
        }
        HirStmt::While { condition, body } => {
            fold_expr(condition);
            fold_in_body(body);
        }
        HirStmt::For { body, .. } => fold_in_body(body),
        _ => {}
    }
}

fn fold_expr(expr: &mut HirExpr) {
    // Recurse first (bottom-up).
    match expr {
        HirExpr::Call { args, .. } => {
            for a in args.iter_mut() {
                fold_expr(a);
            }
        }
        HirExpr::MethodCall { object, args, .. } => {
            fold_expr(object);
            for a in args.iter_mut() {
                fold_expr(a);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            fold_expr(left);
            fold_expr(right);
        }
        _ => {}
    }

    // Now attempt to fold at this level.
    if let Some(folded) = try_fold(expr) {
        *expr = HirExpr::Literal(Literal::String(folded));
    }
}

fn try_fold(expr: &HirExpr) -> Option<String> {
    match expr {
        // format("{}", s) → s
        HirExpr::Call { func, args, .. } if func == "format" || func == "str" => {
            match args.as_slice() {
                [HirExpr::Literal(Literal::String(fmt)), HirExpr::Literal(Literal::String(s))]
                    if fmt == "{}" =>
                {
                    Some(s.clone())
                }
                // format("{}{}", a, b) → a + b
                [HirExpr::Literal(Literal::String(fmt)), HirExpr::Literal(Literal::String(a)), HirExpr::Literal(Literal::String(b))]
                    if fmt == "{}{}" =>
                {
                    Some(format!("{a}{b}"))
                }
                // str(s) with a single string literal
                [HirExpr::Literal(Literal::String(s))] if func == "str" => Some(s.clone()),
                _ => None,
            }
        }

        // "{}".format(s) → s
        HirExpr::MethodCall {
            object,
            method,
            args,
            ..
        } if method == "format" => {
            if let HirExpr::Literal(Literal::String(fmt)) = object.as_ref() {
                match (fmt.as_str(), args.as_slice()) {
                    ("{}", [HirExpr::Literal(Literal::String(s))]) => Some(s.clone()),
                    ("{}{}", [HirExpr::Literal(Literal::String(a)), HirExpr::Literal(Literal::String(b))]) => {
                        Some(format!("{a}{b}"))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }

        // "a" + "b" → "ab"
        HirExpr::Binary {
            op: BinOp::Add,
            left,
            right,
        } => {
            if let (
                HirExpr::Literal(Literal::String(a)),
                HirExpr::Literal(Literal::String(b)),
            ) = (left.as_ref(), right.as_ref())
            {
                Some(format!("{a}{b}"))
            } else {
                None
            }
        }

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{AssignTarget, HirFunction, FunctionProperties, Type};
    use crate::annotations::TranspilationAnnotations;
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

    #[test]
    fn test_format_single_string_folded() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("x".to_string()),
            value: HirExpr::Call {
                func: "format".to_string(),
                args: vec![
                    HirExpr::Literal(Literal::String("{}".to_string())),
                    HirExpr::Literal(Literal::String("hello".to_string())),
                ],
                kwargs: vec![],
                type_params: vec![],
            },
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().fold_format_constants_program(module);
        assert_eq!(
            result.functions[0].body[0],
            HirStmt::Assign {
                target: AssignTarget::Symbol("x".to_string()),
                value: HirExpr::Literal(Literal::String("hello".to_string())),
                type_annotation: None,
            }
        );
    }

    #[test]
    fn test_string_concat_folded() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("x".to_string()),
            value: HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Literal(Literal::String("foo".to_string()))),
                right: Box::new(HirExpr::Literal(Literal::String("bar".to_string()))),
            },
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().fold_format_constants_program(module);
        assert_eq!(
            result.functions[0].body[0],
            HirStmt::Assign {
                target: AssignTarget::Symbol("x".to_string()),
                value: HirExpr::Literal(Literal::String("foobar".to_string())),
                type_annotation: None,
            }
        );
    }

    #[test]
    fn test_non_constant_format_not_folded() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("x".to_string()),
            value: HirExpr::Call {
                func: "format".to_string(),
                args: vec![
                    HirExpr::Literal(Literal::String("{}".to_string())),
                    HirExpr::Var("y".to_string()),
                ],
                kwargs: vec![],
                type_params: vec![],
            },
            type_annotation: None,
        };
        let module = make_module(vec![stmt.clone()]);
        let result = optimizer().fold_format_constants_program(module);
        // Should be unchanged.
        assert_eq!(result.functions[0].body[0], stmt);
    }
}
