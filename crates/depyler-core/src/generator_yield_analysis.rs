//! Generator Yield Point Analysis
//!
//! This module analyzes generator functions to identify yield points
//! and plan the state machine transformation needed for proper coroutine execution.
//!
//! ## Problem
//! Current implementation executes the entire function body in state 0 and returns
//! at the first yield, never resuming. This causes:
//! - Unreachable code after yields
//! - Only one value yielded per generator
//! - Loops with yields don't iterate
//!
//! ## Solution
//! Transform the function into a resumable state machine where each yield point
//! becomes a state transition. Similar to async/await lowering.

use crate::hir::{HirExpr, HirFunction, HirStmt};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct YieldPoint {
    pub state_id: usize,
    pub yield_expr: HirExpr,
    pub live_vars: Vec<String>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct YieldAnalysis {
    pub yield_points: Vec<YieldPoint>,
    pub state_variables: Vec<String>,
    pub resume_points: HashMap<usize, usize>,
}

impl YieldAnalysis {
    pub fn new() -> Self {
        Self {
            yield_points: Vec::new(),
            state_variables: Vec::new(),
            resume_points: HashMap::new(),
        }
    }

    /// Analyze a generator function to find all yield points
    ///
    /// This is the entry point for yield point analysis. It walks the function
    /// body and identifies every yield statement, assigning state numbers.
    ///
    pub fn analyze(func: &HirFunction) -> Self {
        let mut analysis = Self::new();
        let mut state_counter = 1;

        for (idx, stmt) in func.body.iter().enumerate() {
            Self::analyze_stmt(stmt, &mut analysis, &mut state_counter, 0, idx);
        }

        analysis.finalize();
        analysis
    }

    /// Recursively analyze a statement for yield points
    ///
    fn analyze_stmt(
        stmt: &HirStmt,
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        depth: usize,
        stmt_idx: usize,
    ) {
        match stmt {
            HirStmt::Expr(expr) => {
                Self::analyze_expr_stmt(expr, analysis, state_counter, depth, stmt_idx);
            }
            HirStmt::If {
                then_body, else_body, ..
            } => {
                Self::analyze_if_stmt(then_body, else_body, analysis, state_counter, depth, stmt_idx);
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                Self::analyze_loop_stmt(body, analysis, state_counter, depth, stmt_idx);
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                Self::analyze_try_stmt(
                    body,
                    handlers,
                    orelse,
                    finalbody,
                    analysis,
                    state_counter,
                    (depth, stmt_idx),
                );
            }
            HirStmt::With { body, .. } => {
                Self::analyze_with_stmt(body, analysis, state_counter, depth, stmt_idx);
            }
            _ => {}
        }
    }

    /// Analyze expression statement for yield
    ///
    #[inline]
    fn analyze_expr_stmt(
        expr: &HirExpr,
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        depth: usize,
        stmt_idx: usize,
    ) {
        if let Some(yield_expr) = Self::extract_yield_expr(expr) {
            let yield_point = YieldPoint {
                state_id: *state_counter,
                yield_expr,
                live_vars: Vec::new(),
                depth,
            };
            analysis.yield_points.push(yield_point);
            analysis.resume_points.insert(*state_counter, stmt_idx + 1);
            *state_counter += 1;
        }
    }

    /// Analyze if statement branches
    ///
    #[inline]
    fn analyze_if_stmt(
        then_body: &[HirStmt],
        else_body: &Option<Vec<HirStmt>>,
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        depth: usize,
        stmt_idx: usize,
    ) {
        for s in then_body {
            Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
        }
        if let Some(else_stmts) = else_body {
            for s in else_stmts {
                Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
            }
        }
    }

    /// Analyze loop body with increased depth
    ///
    #[inline]
    fn analyze_loop_stmt(
        body: &[HirStmt],
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        depth: usize,
        stmt_idx: usize,
    ) {
        for s in body {
            Self::analyze_stmt(s, analysis, state_counter, depth + 1, stmt_idx);
        }
    }

    /// Analyze try/except/else/finally blocks
    ///
    #[inline]
    fn analyze_try_stmt(
        body: &[HirStmt],
        handlers: &[crate::hir::ExceptHandler],
        orelse: &Option<Vec<HirStmt>>,
        finalbody: &Option<Vec<HirStmt>>,
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        context: (usize, usize), // (depth, stmt_idx)
    ) {
        let (depth, stmt_idx) = context;
        for s in body {
            Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
        }
        for handler in handlers {
            for s in &handler.body {
                Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
            }
        }
        if let Some(else_stmts) = orelse {
            for s in else_stmts {
                Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
            }
        }
        if let Some(finally_stmts) = finalbody {
            for s in finally_stmts {
                Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
            }
        }
    }

    /// Analyze with statement body
    ///
    #[inline]
    fn analyze_with_stmt(
        body: &[HirStmt],
        analysis: &mut YieldAnalysis,
        state_counter: &mut usize,
        depth: usize,
        stmt_idx: usize,
    ) {
        for s in body {
            Self::analyze_stmt(s, analysis, state_counter, depth, stmt_idx);
        }
    }

    /// Extract yield expression if present
    ///
    fn extract_yield_expr(expr: &HirExpr) -> Option<HirExpr> {
        match expr {
            HirExpr::Yield { value } => value.as_ref().map(|v| *v.clone()),
            _ => None,
        }
    }

    /// Finalize analysis by computing live variables and state variables
    ///
    fn finalize(&mut self) {
        // NOTE: Implement liveness analysis to determine which variables need capturing ()
        // need to be preserved in the state struct.
        // For now, we'll rely on the existing GeneratorStateInfo analysis.
    }

    /// Check if this function contains any yields
    #[inline]
    pub fn has_yields(&self) -> bool {
        !self.yield_points.is_empty()
    }

    /// Get the number of states needed (including state 0 for initialization)
    #[inline]
    pub fn num_states(&self) -> usize {
        self.yield_points.len() + 1
    }
}

impl Default for YieldAnalysis {
    fn default() -> Self {
        Self::new()
    }
}
