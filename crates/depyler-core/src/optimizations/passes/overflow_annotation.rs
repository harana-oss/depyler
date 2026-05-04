//! Overflow annotation pass — `Optimizer`.
//!
//! Analyses the integer arithmetic in each function and annotates the function
//! with an [`ArithmeticMode`] hint that the code generator can use to emit the
//! cheapest Rust integer operations instead of always using checked arithmetic.
//!
//! ## Strategy (conservative)
//!
//! * Walk every `BinOp::{Add, Sub, Mul}` expression in the function body.
//! * If *all* integer arithmetic operands are bounded by a known literal range
//!   (i.e. both operands are literals or one operand is a literal that is small
//!   enough that overflow within `i64` is impossible), mark the function as
//!   [`ArithmeticMode::Wrapping`].
//! * Otherwise leave the mode as [`ArithmeticMode::Default`].
//!
//! This is intentionally conservative: unknown variables default to "may
//! overflow", so most functions will keep [`ArithmeticMode::Default`].  The
//! value of the pass is for tight numeric kernels with fully constant operands.

use super::super::optimizer::Optimizer;
use crate::annotations::ArithmeticMode;
use crate::hir::{BinOp, HirExpr, HirModule, HirStmt, Literal};

impl Optimizer {
    pub(crate) fn annotate_overflow_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            let all_bounded = all_arithmetic_bounded(&func.body);
            if all_bounded {
                func.annotations.arithmetic_mode = ArithmeticMode::Wrapping;
            }
        }
        program
    }
}

// ---------------------------------------------------------------------------
// Analysis helpers
// ---------------------------------------------------------------------------

/// Returns `true` when every integer `Add`/`Sub`/`Mul` in `stmts` has both
/// operands as `Literal::Int` values that cannot overflow `i64`.
fn all_arithmetic_bounded(stmts: &[HirStmt]) -> bool {
    stmts.iter().all(|s| stmt_bounded(s))
}

fn stmt_bounded(stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Assign { value, .. } => expr_bounded(value),
        HirStmt::Return(Some(e)) => expr_bounded(e),
        HirStmt::Expr(e) => expr_bounded(e),
        HirStmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_bounded(condition)
                && all_arithmetic_bounded(then_body)
                && else_body
                    .as_ref()
                    .map_or(true, |eb| all_arithmetic_bounded(eb))
        }
        HirStmt::While { condition, body } => {
            expr_bounded(condition) && all_arithmetic_bounded(body)
        }
        HirStmt::For { body, .. } => all_arithmetic_bounded(body),
        _ => true,
    }
}

/// Returns `true` when the expression contains no unbounded integer arithmetic.
fn expr_bounded(expr: &HirExpr) -> bool {
    match expr {
        HirExpr::Binary { op, left, right } => {
            // Recurse on children first.
            if !expr_bounded(left) || !expr_bounded(right) {
                return false;
            }
            // Check this node.
            if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
                return arithmetic_op_bounded(left, right);
            }
            true
        }
        HirExpr::Unary { operand, .. } => expr_bounded(operand),
        HirExpr::Call { args, .. } => args.iter().all(expr_bounded),
        HirExpr::MethodCall { object, args, .. } => {
            expr_bounded(object) && args.iter().all(expr_bounded)
        }
        HirExpr::List(elems) | HirExpr::Tuple(elems) | HirExpr::Set(elems) => {
            elems.iter().all(expr_bounded)
        }
        HirExpr::Index { base, index } => expr_bounded(base) && expr_bounded(index),
        _ => true,
    }
}

/// Conservative check: both operands must be small `Literal::Int` values.
/// "Small" means the operation cannot overflow a signed 64-bit integer.
fn arithmetic_op_bounded(left: &HirExpr, right: &HirExpr) -> bool {
    const SAFE_BOUND: i64 = i32::MAX as i64; // conservative: fits in i32 ⇒ operations fit in i64

    match (left, right) {
        (HirExpr::Literal(Literal::Int(a)), HirExpr::Literal(Literal::Int(b))) => {
            a.abs() <= SAFE_BOUND && b.abs() <= SAFE_BOUND
        }
        _ => false, // Unknown variable — conservatively assume may overflow.
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
                ret_type: Type::Int,
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
    fn test_small_literal_arithmetic_gets_wrapping_mode() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("r".to_string()),
            value: HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Literal(Literal::Int(10))),
                right: Box::new(HirExpr::Literal(Literal::Int(20))),
            },
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().annotate_overflow_program(module);
        assert_eq!(
            result.functions[0].annotations.arithmetic_mode,
            ArithmeticMode::Wrapping
        );
    }

    #[test]
    fn test_variable_arithmetic_stays_default() {
        let stmt = HirStmt::Assign {
            target: AssignTarget::Symbol("r".to_string()),
            value: HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Var("x".to_string())),
                right: Box::new(HirExpr::Literal(Literal::Int(1))),
            },
            type_annotation: None,
        };
        let module = make_module(vec![stmt]);
        let result = optimizer().annotate_overflow_program(module);
        assert_eq!(
            result.functions[0].annotations.arithmetic_mode,
            ArithmeticMode::Default
        );
    }
}
