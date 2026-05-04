//! Unused variable elimination pass.
//!
//! Implements [`Optimizer`] methods for the `eliminate_dead_code` family.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, HirExpr, HirFunction, HirModule, HirStmt};
use std::collections::{HashMap, HashSet};

impl Optimizer {
    pub(crate) fn eliminate_dead_code_program(&self, mut program: HirModule) -> HirModule {
        for func in &mut program.functions {
            self.eliminate_dead_code_function(func);
        }
        program
    }

    fn eliminate_dead_code_function(&self, func: &mut HirFunction) {
        // Collect truly used variables (referenced after assignment)
        let mut used_vars = HashMap::new();
        for stmt in &func.body {
            self.collect_truly_used_vars_stmt(stmt, &mut used_vars);
        }

        // Collect assignments with side effects (indexing) that need to be preserved
        let mut side_effect_vars = HashSet::new();
        for stmt in &func.body {
            if let HirStmt::Assign { target, value, .. } = stmt {
                if Self::expr_contains_index(value) {
                    if let AssignTarget::Symbol(name) = target {
                        side_effect_vars.insert(name.clone());
                    }
                }
            }
        }

        // Rename variables that have side effects but aren't actually used to `_varname`
        // This prevents Rust's unused_variables warning while preserving the side effect
        for stmt in &mut func.body {
            if let HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                ..
            } = stmt
            {
                if side_effect_vars.contains(name) && !used_vars.contains_key(name) {
                    *name = format!("_{}", name);
                }
            }
        }

        // Remove truly dead assignments (not used and no side effects)
        func.body.retain(|stmt| {
            if let HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                ..
            } = stmt
            {
                // Keep if: truly used OR has side effects (including renamed _varname)
                used_vars.contains_key(name)
                    || side_effect_vars.contains(name.trim_start_matches('_'))
            } else {
                true
            }
        });
    }

    /// This version does NOT mark side-effect assignments as used - that's handled separately
    /// in eliminate_dead_code_function to allow renaming them to `_varname`.
    fn collect_truly_used_vars_stmt(&self, stmt: &HirStmt, used: &mut HashMap<String, bool>) {
        match stmt {
            HirStmt::Assign { target, value, .. } => {
                // This fixes property writes like `b.size = 20` where `b` is used on LHS
                self.collect_used_vars_assign_target(target, used);
                self.collect_used_vars_expr(value, used);
                // NOTE: We do NOT mark side-effect assignments as used here
                // That's now handled in eliminate_dead_code_function
            }
            HirStmt::Return(Some(expr)) => {
                self.collect_used_vars_expr(expr, used);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.collect_used_vars_expr(condition, used);
                for s in then_body {
                    self.collect_truly_used_vars_stmt(s, used);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.collect_truly_used_vars_stmt(s, used);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                self.collect_used_vars_expr(condition, used);
                for s in body {
                    self.collect_truly_used_vars_stmt(s, used);
                }
            }
            HirStmt::For { iter, body, .. } => {
                self.collect_used_vars_expr(iter, used);
                for s in body {
                    self.collect_truly_used_vars_stmt(s, used);
                }
            }
            HirStmt::Expr(expr) => {
                self.collect_used_vars_expr(expr, used);
            }
            _ => {}
        }
    }

    /// Returns true if the expression tree contains any Index nodes, which indicate
    /// operations that can fail (e.g., list[0], dict["key"]) and have side effects.
    ///
    /// # Complexity
    /// 5 (recursive expression traversal with early return)
    fn expr_contains_index(expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Index { .. } => true,
            HirExpr::Binary { left, right, .. } => {
                Self::expr_contains_index(left) || Self::expr_contains_index(right)
            }
            HirExpr::Unary { operand, .. } => Self::expr_contains_index(operand),
            HirExpr::Call { args, .. } => args.iter().any(Self::expr_contains_index),
            HirExpr::List(items) | HirExpr::Tuple(items) => {
                items.iter().any(Self::expr_contains_index)
            }
            HirExpr::Dict(pairs) => pairs
                .iter()
                .any(|(k, v)| Self::expr_contains_index(k) || Self::expr_contains_index(v)),
            HirExpr::Set(items) => items.iter().any(Self::expr_contains_index),
            HirExpr::MethodCall { object, args, .. } => {
                Self::expr_contains_index(object) || args.iter().any(Self::expr_contains_index)
            }
            HirExpr::Attribute { value, .. } => Self::expr_contains_index(value),
            HirExpr::Slice {
                base,
                start,
                stop,
                step,
            } => {
                Self::expr_contains_index(base)
                    || start.as_ref().is_some_and(|e| Self::expr_contains_index(e))
                    || stop.as_ref().is_some_and(|e| Self::expr_contains_index(e))
                    || step.as_ref().is_some_and(|e| Self::expr_contains_index(e))
            }
            _ => false,
        }
    }

    fn collect_used_vars_expr(&self, expr: &HirExpr, used: &mut HashMap<String, bool>) {
        super::cse::collect_used_vars_expr_inner(expr, used);
    }

    fn collect_used_vars_assign_target(
        &self,
        target: &AssignTarget,
        used: &mut HashMap<String, bool>,
    ) {
        match target {
            AssignTarget::Symbol(_) => {
                // Simple variable assignment - no variables used on LHS
            }
            AssignTarget::Index { base, index } => {
                // Collect from both base and index expressions
                // e.g., `arr[i] = value` uses both `arr` and `i`
                self.collect_used_vars_expr(base, used);
                self.collect_used_vars_expr(index, used);
            }
            AssignTarget::Slice {
                base,
                start,
                stop,
                step,
            } => {
                // Collect from base and slice bounds
                self.collect_used_vars_expr(base, used);
                if let Some(s) = start {
                    self.collect_used_vars_expr(s, used);
                }
                if let Some(s) = stop {
                    self.collect_used_vars_expr(s, used);
                }
                if let Some(s) = step {
                    self.collect_used_vars_expr(s, used);
                }
            }
            AssignTarget::Attribute { value, .. } => {
                // Collect from the base object
                // e.g., `obj.attr = value` uses `obj`
                self.collect_used_vars_expr(value, used);
            }
            AssignTarget::Tuple(targets) => {
                // Recursively collect from tuple elements
                for t in targets {
                    self.collect_used_vars_assign_target(t, used);
                }
            }
            AssignTarget::Starred(_) => {
                // Starred target in unpacking - no variables used on LHS
            }
        }
    }
}
