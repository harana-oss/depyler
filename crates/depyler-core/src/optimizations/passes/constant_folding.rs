//! Constant folding pass for [`PerformanceOptimizer`].
//!
//! Folds compile-time-known binary expressions into their result literals,
//! walking the full statement tree recursively.

use super::super::optimizer::PerformanceOptimizer;
use crate::hir::{BinOp, HirExpr, HirStmt, Literal};

impl PerformanceOptimizer {
    pub(crate) fn constant_folding(&mut self, stmts: &mut Vec<HirStmt>) {
        for stmt in stmts.iter_mut() {
            self.fold_constants_in_stmt(stmt);
        }
        self.optimizations_applied
            .push("constant_folding".to_string());
    }

    pub(crate) fn fold_constants_in_stmt(&mut self, stmt: &mut HirStmt) {
        match stmt {
            HirStmt::Assign { value, .. } => {
                self.fold_constants_expr(value);
            }
            HirStmt::Return(Some(expr)) => {
                self.fold_constants_expr(expr);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.fold_constants_in_if(condition, then_body, else_body);
            }
            HirStmt::While { condition, body } => {
                self.fold_constants_in_while(condition, body);
            }
            HirStmt::For { body, .. } => {
                self.constant_folding(body);
            }
            _ => {}
        }
    }

    pub(crate) fn fold_constants_in_if(
        &mut self,
        condition: &mut HirExpr,
        then_body: &mut Vec<HirStmt>,
        else_body: &mut Option<Vec<HirStmt>>,
    ) {
        self.fold_constants_expr(condition);
        self.constant_folding(then_body);
        if let Some(else_stmts) = else_body {
            self.constant_folding(else_stmts);
        }
    }

    pub(crate) fn fold_constants_in_while(
        &mut self,
        condition: &mut HirExpr,
        body: &mut Vec<HirStmt>,
    ) {
        self.fold_constants_expr(condition);
        self.constant_folding(body);
    }

    pub(crate) fn fold_constants_expr(&self, expr: &mut HirExpr) {
        if let HirExpr::Binary { op, left, right } = expr {
            self.fold_constants_expr(left);
            self.fold_constants_expr(right);

            if let (HirExpr::Literal(left_lit), HirExpr::Literal(right_lit)) =
                (left.as_ref(), right.as_ref())
            {
                if let Some(folded) = self.evaluate_binary_op(*op, left_lit, right_lit) {
                    *expr = HirExpr::Literal(folded);
                }
            }
        }
    }

    pub(crate) fn evaluate_binary_op(
        &self,
        op: BinOp,
        left: &Literal,
        right: &Literal,
    ) -> Option<Literal> {
        match (op, left, right) {
            (BinOp::Add, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a + b)),
            (BinOp::Sub, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a - b)),
            (BinOp::Mul, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a * b)),
            (BinOp::Add, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a + b)),
            (BinOp::Sub, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a - b)),
            (BinOp::Mul, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a * b)),
            _ => None,
        }
    }
}
