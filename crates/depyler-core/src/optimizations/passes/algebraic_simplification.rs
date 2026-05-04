//! Algebraic simplification pass — `PerformanceOptimizer`.
//!
//! Extends (and partially replaces) the `strength_reduction` stub with full
//! algebraic identity reduction.  Runs *after* `constant_folding` so operands
//! are already simplified where possible.
//!
//! ## Identities applied
//!
//! | Pattern     | Simplifies to        | Notes                          |
//! |-------------|----------------------|--------------------------------|
//! | `x + 0`     | `x`                  |                                |
//! | `0 + x`     | `x`                  |                                |
//! | `x - 0`     | `x`                  |                                |
//! | `x * 1`     | `x`                  |                                |
//! | `1 * x`     | `x`                  |                                |
//! | `x * 0`     | `0`                  |                                |
//! | `0 * x`     | `0`                  |                                |
//! | `x / 1`     | `x`                  |                                |
//! | `x ** 0`    | `1`                  |                                |
//! | `x ** 1`    | `x`                  |                                |
//! | `x - x`     | `0`                  | only for simple `Var` operands |
//! | `!!x`       | `x`                  | double-negation                |

use super::super::optimizer::PerformanceOptimizer;
use crate::hir::{BinOp, HirExpr, HirStmt, Literal, UnaryOp};

impl PerformanceOptimizer {
    pub(crate) fn algebraic_simplification(&mut self, stmts: &mut Vec<HirStmt>) {
        for stmt in stmts.iter_mut() {
            simplify_stmt(stmt);
        }
        self.optimizations_applied
            .push("algebraic_simplification".to_string());
    }
}

fn simplify_stmt(stmt: &mut HirStmt) {
    match stmt {
        HirStmt::Assign { value, .. } => simplify_expr(value),
        HirStmt::Return(Some(e)) => simplify_expr(e),
        HirStmt::Expr(e) => simplify_expr(e),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            simplify_expr(condition);
            for s in then_body.iter_mut() {
                simplify_stmt(s);
            }
            if let Some(eb) = else_body {
                for s in eb.iter_mut() {
                    simplify_stmt(s);
                }
            }
        }
        HirStmt::While { condition, body } => {
            simplify_expr(condition);
            for s in body.iter_mut() {
                simplify_stmt(s);
            }
        }
        HirStmt::For { body, .. } => {
            for s in body.iter_mut() {
                simplify_stmt(s);
            }
        }
        _ => {}
    }
}

fn simplify_expr(expr: &mut HirExpr) {
    // Recurse first (bottom-up).
    match expr {
        HirExpr::Binary { left, right, .. } => {
            simplify_expr(left);
            simplify_expr(right);
        }
        HirExpr::Unary { operand, .. } => simplify_expr(operand),
        HirExpr::Call { args, .. } => {
            for a in args.iter_mut() {
                simplify_expr(a);
            }
        }
        HirExpr::MethodCall { object, args, .. } => {
            simplify_expr(object);
            for a in args.iter_mut() {
                simplify_expr(a);
            }
        }
        _ => {}
    }

    // Apply algebraic identities.
    let simplified = apply_identity(expr);
    if let Some(new_expr) = simplified {
        *expr = new_expr;
    }
}

fn apply_identity(expr: &HirExpr) -> Option<HirExpr> {
    match expr {
        // ── Binary identities ──────────────────────────────────────────────
        HirExpr::Binary { op, left, right } => {
            let lit_zero_int = |e: &HirExpr| matches!(e, HirExpr::Literal(Literal::Int(0)));
            let lit_one_int = |e: &HirExpr| matches!(e, HirExpr::Literal(Literal::Int(1)));
            let lit_zero_f = |e: &HirExpr| {
                matches!(e, HirExpr::Literal(Literal::Float(f)) if *f == 0.0)
            };
            let lit_one_f = |e: &HirExpr| {
                matches!(e, HirExpr::Literal(Literal::Float(f)) if *f == 1.0)
            };

            match op {
                BinOp::Add => {
                    if lit_zero_int(right) || lit_zero_f(right) {
                        return Some(*left.clone());
                    }
                    if lit_zero_int(left) || lit_zero_f(left) {
                        return Some(*right.clone());
                    }
                }
                BinOp::Sub => {
                    if lit_zero_int(right) || lit_zero_f(right) {
                        return Some(*left.clone());
                    }
                    // x - x → 0  (only for simple Var)
                    if let (HirExpr::Var(a), HirExpr::Var(b)) = (left.as_ref(), right.as_ref()) {
                        if a == b {
                            return Some(HirExpr::Literal(Literal::Int(0)));
                        }
                    }
                }
                BinOp::Mul => {
                    if lit_zero_int(right) || lit_zero_f(right) {
                        return Some(HirExpr::Literal(Literal::Int(0)));
                    }
                    if lit_zero_int(left) || lit_zero_f(left) {
                        return Some(HirExpr::Literal(Literal::Int(0)));
                    }
                    if lit_one_int(right) || lit_one_f(right) {
                        return Some(*left.clone());
                    }
                    if lit_one_int(left) || lit_one_f(left) {
                        return Some(*right.clone());
                    }
                }
                BinOp::Div => {
                    if lit_one_int(right) || lit_one_f(right) {
                        return Some(*left.clone());
                    }
                }
                BinOp::Pow => {
                    if lit_zero_int(right) || lit_zero_f(right) {
                        return Some(HirExpr::Literal(Literal::Int(1)));
                    }
                    if lit_one_int(right) || lit_one_f(right) {
                        return Some(*left.clone());
                    }
                }
                _ => {}
            }
        }

        // ── Unary double-negation: !!x → x ────────────────────────────────
        HirExpr::Unary {
            op: UnaryOp::Not,
            operand,
        } => {
            if let HirExpr::Unary {
                op: UnaryOp::Not,
                operand: inner,
            } = operand.as_ref()
            {
                return Some(*inner.clone());
            }
        }

        _ => {}
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::AssignTarget;
    use crate::optimizations::optimizer::PerformanceOptimizer;

    fn make_assign(value: HirExpr) -> HirStmt {
        HirStmt::Assign {
            target: AssignTarget::Symbol("x".to_string()),
            value,
            type_annotation: None,
        }
    }

    fn extract_value(stmt: &HirStmt) -> &HirExpr {
        match stmt {
            HirStmt::Assign { value, .. } => value,
            _ => panic!("not an assign"),
        }
    }

    #[test]
    fn test_add_zero_simplified() {
        let expr = HirExpr::Binary {
            op: BinOp::Add,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(0))),
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.algebraic_simplification(&mut stmts);
        assert_eq!(extract_value(&stmts[0]), &HirExpr::Var("x".to_string()));
    }

    #[test]
    fn test_mul_zero_simplified() {
        let expr = HirExpr::Binary {
            op: BinOp::Mul,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(0))),
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.algebraic_simplification(&mut stmts);
        assert_eq!(
            extract_value(&stmts[0]),
            &HirExpr::Literal(Literal::Int(0))
        );
    }

    #[test]
    fn test_pow_zero_simplified() {
        let expr = HirExpr::Binary {
            op: BinOp::Pow,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(0))),
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.algebraic_simplification(&mut stmts);
        assert_eq!(
            extract_value(&stmts[0]),
            &HirExpr::Literal(Literal::Int(1))
        );
    }

    #[test]
    fn test_double_negation_removed() {
        let expr = HirExpr::Unary {
            op: UnaryOp::Not,
            operand: Box::new(HirExpr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(HirExpr::Var("b".to_string())),
            }),
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.algebraic_simplification(&mut stmts);
        assert_eq!(extract_value(&stmts[0]), &HirExpr::Var("b".to_string()));
    }

    #[test]
    fn test_x_minus_x_simplified() {
        let expr = HirExpr::Binary {
            op: BinOp::Sub,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Var("x".to_string())),
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.algebraic_simplification(&mut stmts);
        assert_eq!(
            extract_value(&stmts[0]),
            &HirExpr::Literal(Literal::Int(0))
        );
    }
}
