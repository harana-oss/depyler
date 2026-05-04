//! Strength reduction pass for [`PerformanceOptimizer`].
//!
//! Replaces expensive arithmetic operations with cheaper shift / mask
//! equivalents when the operand is a power-of-two literal.
//!
//! ## Transformations (P4-C — wrapping semantics, unsigned intent)
//!
//! | Pattern         | Replacement               | Condition                    |
//! |-----------------|---------------------------|------------------------------|
//! | `x * 2^n`       | `x << n`                  | right operand is power of 2  |
//! | `x / 2^n`       | `x >> n`                  | right operand is power of 2  |
//! | `x % 2^n`       | `x & (2^n - 1)`           | right operand is power of 2  |
//!
//! Only fires for positive power-of-two **integer** literals.  Floating-point
//! and negative values are left untouched.

use super::super::optimizer::PerformanceOptimizer;
use crate::hir::{BinOp, HirExpr, HirStmt, Literal};

impl PerformanceOptimizer {
    pub(crate) fn strength_reduction(&mut self, stmts: &mut [HirStmt]) {
        for stmt in stmts.iter_mut() {
            match stmt {
                HirStmt::Assign { value, .. } => {
                    self.reduce_strength_expr(value);
                }
                HirStmt::Return(Some(expr)) => {
                    self.reduce_strength_expr(expr);
                }
                HirStmt::If { condition, then_body, else_body } => {
                    self.reduce_strength_expr(condition);
                    self.strength_reduction(then_body);
                    if let Some(eb) = else_body {
                        self.strength_reduction(eb);
                    }
                }
                HirStmt::While { condition, body } => {
                    self.reduce_strength_expr(condition);
                    self.strength_reduction(body);
                }
                HirStmt::For { body, .. } => {
                    self.strength_reduction(body);
                }
                _ => {}
            }
        }
        self.optimizations_applied
            .push("strength_reduction".to_string());
    }

    pub(crate) fn reduce_strength_expr(&self, expr: &mut HirExpr) {
        // Recurse first (bottom-up).
        match expr {
            HirExpr::Binary { left, right, .. } => {
                self.reduce_strength_expr(left);
                self.reduce_strength_expr(right);
            }
            HirExpr::Unary { operand, .. } => self.reduce_strength_expr(operand),
            _ => {}
        }

        // Apply reduction at this level.
        if let Some(new_expr) = try_reduce(expr) {
            *expr = new_expr;
        }
    }
}

/// Returns `Some(reduced_expr)` when `expr` can be strength-reduced.
fn try_reduce(expr: &HirExpr) -> Option<HirExpr> {
    let HirExpr::Binary { op, left, right } = expr else {
        return None;
    };
    let n = power_of_two_exp(right)?;

    match op {
        BinOp::Mul => {
            // x * 2^n  →  x << n
            Some(HirExpr::Binary {
                op: BinOp::LShift,
                left: left.clone(),
                right: Box::new(HirExpr::Literal(Literal::Int(n))),
            })
        }
        BinOp::Div | BinOp::FloorDiv => {
            // x / 2^n  →  x >> n  (unsigned intent)
            Some(HirExpr::Binary {
                op: BinOp::RShift,
                left: left.clone(),
                right: Box::new(HirExpr::Literal(Literal::Int(n))),
            })
        }
        BinOp::Mod => {
            // x % 2^n  →  x & (2^n - 1)
            let mask = (1i64 << n) - 1;
            Some(HirExpr::Binary {
                op: BinOp::BitAnd,
                left: left.clone(),
                right: Box::new(HirExpr::Literal(Literal::Int(mask))),
            })
        }
        _ => None,
    }
}

/// If `expr` is a positive integer literal that is a power of two, return the
/// exponent `n` such that `2^n == value`.  Otherwise return `None`.
fn power_of_two_exp(expr: &HirExpr) -> Option<i64> {
    if let HirExpr::Literal(Literal::Int(v)) = expr {
        let v = *v;
        if v > 0 && v.count_ones() == 1 {
            return Some(v.trailing_zeros() as i64);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{AssignTarget};
    use crate::optimizations::optimizer::PerformanceOptimizer;

    fn make_assign(value: HirExpr) -> HirStmt {
        HirStmt::Assign {
            target: AssignTarget::Symbol("r".to_string()),
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
    fn test_mul_power_of_two_becomes_shift() {
        let expr = HirExpr::Binary {
            op: BinOp::Mul,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(8))), // 2^3
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.strength_reduction(&mut stmts);
        assert!(
            matches!(
                extract_value(&stmts[0]),
                HirExpr::Binary { op: BinOp::LShift, right, .. }
                if matches!(right.as_ref(), HirExpr::Literal(Literal::Int(3)))
            ),
            "expected x << 3"
        );
    }

    #[test]
    fn test_div_power_of_two_becomes_rshift() {
        let expr = HirExpr::Binary {
            op: BinOp::Div,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(4))), // 2^2
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.strength_reduction(&mut stmts);
        assert!(
            matches!(
                extract_value(&stmts[0]),
                HirExpr::Binary { op: BinOp::RShift, right, .. }
                if matches!(right.as_ref(), HirExpr::Literal(Literal::Int(2)))
            ),
            "expected x >> 2"
        );
    }

    #[test]
    fn test_mod_power_of_two_becomes_bitand() {
        let expr = HirExpr::Binary {
            op: BinOp::Mod,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(16))), // 2^4
        };
        let mut stmts = vec![make_assign(expr)];
        let mut opt = PerformanceOptimizer::new();
        opt.strength_reduction(&mut stmts);
        assert!(
            matches!(
                extract_value(&stmts[0]),
                HirExpr::Binary { op: BinOp::BitAnd, right, .. }
                if matches!(right.as_ref(), HirExpr::Literal(Literal::Int(15))) // 2^4 - 1
            ),
            "expected x & 15"
        );
    }

    #[test]
    fn test_non_power_of_two_not_reduced() {
        let expr = HirExpr::Binary {
            op: BinOp::Mul,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(6))), // not a power of 2
        };
        let mut stmts = vec![make_assign(expr.clone())];
        let mut opt = PerformanceOptimizer::new();
        opt.strength_reduction(&mut stmts);
        assert_eq!(extract_value(&stmts[0]), &expr);
    }
}

