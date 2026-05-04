//! Branch-to-match pass — `Optimizer`.
//!
//! Rewrites if-else chains of the form
//!
//! ```python
//! if x == A:   …
//! elif x == B: …
//! elif x == C: …
//! else:        …
//! ```
//!
//! into a `HirStmt::Match` node, which the code generator lowers to a Rust
//! `match` expression (potentially compiled to a jump table by LLVM).
//!
//! ## Criterion
//!
//! A chain of ≥ 3 consecutive `If` blocks must compare the **same variable**
//! against **literals** using `BinOp::Eq`.  The chain is detected by following
//! the `else_body` of each `If` statement.

use super::super::optimizer::Optimizer;
use crate::hir::{
    AssignTarget, BinOp, HirExpr, HirModule, HirPattern, HirStmt, Literal, MatchCase,
};

/// Minimum chain length (number of literal arms) to trigger the rewrite.
const MIN_CHAIN_LEN: usize = 3;

impl Optimizer {
    pub(crate) fn branch_to_match_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            func.body = rewrite_block(std::mem::take(&mut func.body));
        }
        program
    }
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

fn rewrite_block(stmts: Vec<HirStmt>) -> Vec<HirStmt> {
    stmts
        .into_iter()
        .map(|stmt| match stmt {
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                // Try to interpret this if as the start of a chain.
                if let Some(match_stmt) = try_build_match(&condition, then_body.clone(), else_body.clone()) {
                    match_stmt
                } else {
                    HirStmt::If {
                        condition,
                        then_body: rewrite_block(then_body),
                        else_body: else_body.map(rewrite_block),
                    }
                }
            }
            other => other,
        })
        .collect()
}

/// Try to build a `Match` node from an if-else chain.
/// Returns `Some(HirStmt::Match { … })` when the chain qualifies.
fn try_build_match(
    condition: &HirExpr,
    then_body: Vec<HirStmt>,
    else_body: Option<Vec<HirStmt>>,
) -> Option<HirStmt> {
    // The condition must be `var == literal`.
    let (var_name, first_lit) = extract_eq_var_lit(condition)?;

    // Collect all arms by following the else_body chain.
    let mut cases: Vec<MatchCase> = Vec::new();

    // First arm.
    cases.push(MatchCase {
        pattern: literal_to_pattern(&first_lit),
        guard: None,
        body: rewrite_block(then_body),
    });

    // Follow the chain.
    let mut current_else = else_body;
    let mut wildcard_body: Option<Vec<HirStmt>> = None;

    loop {
        match current_else {
            None => break,
            Some(stmts) => {
                // The else block must be a single If to continue the chain.
                if stmts.len() == 1 {
                    let single = stmts.into_iter().next().unwrap();
                    if let HirStmt::If {
                        condition: cond,
                        then_body: tb,
                        else_body: eb,
                    } = single
                    {
                        if let Some((v, lit)) = extract_eq_var_lit(&cond) {
                            if v == var_name {
                                cases.push(MatchCase {
                                    pattern: literal_to_pattern(&lit),
                                    guard: None,
                                    body: rewrite_block(tb),
                                });
                                current_else = eb;
                                continue;
                            }
                        }
                        // Else branch is an If but doesn't match the pattern — treat as wildcard.
                        wildcard_body = Some(vec![HirStmt::If {
                            condition: cond,
                            then_body: tb,
                            else_body: eb,
                        }]);
                        break;
                    } else {
                        // single wasn't an If; treat as wildcard block
                        wildcard_body = Some(rewrite_block(vec![single]));
                        break;
                    }
                }
                // Else block is a plain else — wildcard arm.
                wildcard_body = Some(rewrite_block(stmts));
                break;
            }
        }
    }

    // Only rewrite if we have enough literal arms.
    if cases.len() < MIN_CHAIN_LEN {
        return None;
    }

    // Add wildcard arm for the else branch (if present).
    if let Some(wb) = wildcard_body {
        cases.push(MatchCase {
            pattern: HirPattern::Wildcard,
            guard: None,
            body: wb,
        });
    }

    Some(HirStmt::Match {
        subject: HirExpr::Var(var_name),
        cases,
    })
}

/// Returns `Some((variable_name, literal))` if `expr` is `Var == Literal` or
/// `Literal == Var`.
fn extract_eq_var_lit(expr: &HirExpr) -> Option<(String, Literal)> {
    if let HirExpr::Binary {
        op: BinOp::Eq,
        left,
        right,
    } = expr
    {
        match (left.as_ref(), right.as_ref()) {
            (HirExpr::Var(name), HirExpr::Literal(lit)) => {
                Some((name.clone(), lit.clone()))
            }
            (HirExpr::Literal(lit), HirExpr::Var(name)) => {
                Some((name.clone(), lit.clone()))
            }
            _ => None,
        }
    } else {
        None
    }
}

fn literal_to_pattern(lit: &Literal) -> HirPattern {
    match lit {
        Literal::None => HirPattern::Singleton(lit.clone()),
        Literal::Bool(_) => HirPattern::Singleton(lit.clone()),
        _ => HirPattern::Value(HirExpr::Literal(lit.clone())),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{HirFunction, FunctionProperties, Type};
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

    fn ret(n: i64) -> Vec<HirStmt> {
        vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(n))))]
    }

    fn eq_x(n: i64) -> HirExpr {
        HirExpr::Binary {
            op: BinOp::Eq,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Literal(Literal::Int(n))),
        }
    }

    /// Build a nested if-elif-elif-else chain.
    fn chain(arms: &[(i64, i64)], else_val: i64) -> HirStmt {
        let mut result: Option<HirStmt> = Some(HirStmt::If {
            condition: eq_x(0),  // placeholder, will be overwritten
            then_body: vec![],
            else_body: Some(ret(else_val)),
        });
        // Build from last to first.
        for &(lit, val) in arms.iter().rev() {
            result = Some(HirStmt::If {
                condition: eq_x(lit),
                then_body: ret(val),
                else_body: if let Some(r) = result {
                    Some(vec![r])
                } else {
                    Some(ret(else_val))
                },
            });
        }
        result.unwrap()
    }

    #[test]
    fn test_three_arm_chain_becomes_match() {
        let if_chain = HirStmt::If {
            condition: eq_x(1),
            then_body: ret(10),
            else_body: Some(vec![HirStmt::If {
                condition: eq_x(2),
                then_body: ret(20),
                else_body: Some(vec![HirStmt::If {
                    condition: eq_x(3),
                    then_body: ret(30),
                    else_body: Some(ret(0)),
                }]),
            }]),
        };
        let module = make_module(vec![if_chain]);
        let result = optimizer().branch_to_match_program(module);
        assert!(
            matches!(&result.functions[0].body[0], HirStmt::Match { .. }),
            "expected Match node"
        );
    }

    #[test]
    fn test_two_arm_chain_not_converted() {
        let if_chain = HirStmt::If {
            condition: eq_x(1),
            then_body: ret(10),
            else_body: Some(vec![HirStmt::If {
                condition: eq_x(2),
                then_body: ret(20),
                else_body: None,
            }]),
        };
        let module = make_module(vec![if_chain]);
        let result = optimizer().branch_to_match_program(module);
        assert!(
            matches!(&result.functions[0].body[0], HirStmt::If { .. }),
            "two-arm chain should not be converted"
        );
    }
}
