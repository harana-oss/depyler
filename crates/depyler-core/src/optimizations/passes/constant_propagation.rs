//! Constant propagation pass.
//!
//! Implements [`Optimizer`] methods for the `propagate_constants` family.

use super::super::optimizer::Optimizer;
use crate::hir::{AssignTarget, BinOp, HirExpr, HirFunction, HirModule, HirStmt, Literal, UnaryOp};
use std::collections::{HashMap, HashSet};

impl Optimizer {
    pub(crate) fn propagate_constants_program(&self, mut program: HirModule) -> HirModule {
        let mut constants = HashMap::new();

        let mut mutated_vars = HashSet::new();
        for func in &program.functions {
            self.collect_mutated_vars_function(func, &mut mutated_vars);
        }

        let mut read_vars = HashSet::new();
        for func in &program.functions {
            self.collect_read_vars_function(func, &mut read_vars);
        }

        for func in &program.functions {
            self.collect_constants_function(func, &mut constants, &mutated_vars, &read_vars);
        }

        for func in &mut program.functions {
            self.propagate_constants_function(func, &constants);
        }

        program
    }

    fn collect_mutated_vars_function(
        &self,
        func: &HirFunction,
        mutated_vars: &mut HashSet<String>,
    ) {
        let mut assignments = HashMap::new();
        self.count_assignments_stmt(&func.body, &mut assignments);

        for (var, count) in assignments {
            if count > 1 {
                mutated_vars.insert(var);
            }
        }
    }

    fn collect_read_vars_function(&self, func: &HirFunction, read_vars: &mut HashSet<String>) {
        for stmt in &func.body {
            Self::collect_read_vars_stmt(stmt, read_vars);
        }
    }

    fn collect_read_vars_stmt(stmt: &HirStmt, read_vars: &mut HashSet<String>) {
        match stmt {
            HirStmt::Assign { value, .. } => {
                Self::collect_read_vars_expr(value, read_vars);
            }
            HirStmt::Expr(expr) => {
                Self::collect_read_vars_expr(expr, read_vars);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                Self::collect_read_vars_expr(condition, read_vars);
                for s in then_body {
                    Self::collect_read_vars_stmt(s, read_vars);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        Self::collect_read_vars_stmt(s, read_vars);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                Self::collect_read_vars_expr(condition, read_vars);
                for s in body {
                    Self::collect_read_vars_stmt(s, read_vars);
                }
            }
            HirStmt::For { iter, body, .. } => {
                Self::collect_read_vars_expr(iter, read_vars);
                for s in body {
                    Self::collect_read_vars_stmt(s, read_vars);
                }
            }
            HirStmt::Return(Some(expr)) => {
                Self::collect_read_vars_expr(expr, read_vars);
            }
            _ => {}
        }
    }

    fn collect_read_vars_expr(expr: &HirExpr, read_vars: &mut HashSet<String>) {
        match expr {
            HirExpr::Var(name) => {
                read_vars.insert(name.clone());
            }
            HirExpr::Binary { left, right, .. } => {
                Self::collect_read_vars_expr(left, read_vars);
                Self::collect_read_vars_expr(right, read_vars);
            }
            HirExpr::Unary { operand, .. } => {
                Self::collect_read_vars_expr(operand, read_vars);
            }
            HirExpr::List(items) => {
                for item in items {
                    Self::collect_read_vars_expr(item, read_vars);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    Self::collect_read_vars_expr(k, read_vars);
                    Self::collect_read_vars_expr(v, read_vars);
                }
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    Self::collect_read_vars_expr(arg, read_vars);
                }
            }
            HirExpr::MethodCall { object, args, .. } => {
                Self::collect_read_vars_expr(object, read_vars);
                for arg in args {
                    Self::collect_read_vars_expr(arg, read_vars);
                }
            }
            HirExpr::Lambda { body, .. } => {
                Self::collect_read_vars_expr(body, read_vars);
            }
            _ => {}
        }
    }

    fn count_assignments_stmt(&self, stmts: &[HirStmt], assignments: &mut HashMap<String, usize>) {
        for stmt in stmts {
            self.count_assignments_in_single_stmt(stmt, assignments);
        }
    }

    fn count_assignments_in_single_stmt(
        &self,
        stmt: &HirStmt,
        assignments: &mut HashMap<String, usize>,
    ) {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                ..
            } => {
                *assignments.entry(name.clone()).or_insert(0) += 1;
            }
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                self.count_assignments_stmt(then_body, assignments);
                if let Some(else_stmts) = else_body {
                    self.count_assignments_stmt(else_stmts, assignments);
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                self.count_assignments_stmt(body, assignments);
            }
            _ => {}
        }
    }

    fn collect_constants_function(
        &self,
        func: &HirFunction,
        constants: &mut HashMap<String, HirExpr>,
        mutated_vars: &HashSet<String>,
        used_vars: &HashSet<String>,
    ) {
        for stmt in &func.body {
            self.collect_constants_stmt(stmt, constants, mutated_vars, used_vars);
        }
    }

    fn collect_constants_stmt(
        &self,
        stmt: &HirStmt,
        constants: &mut HashMap<String, HirExpr>,
        mutated_vars: &HashSet<String>,
        used_vars: &HashSet<String>,
    ) {
        match stmt {
            HirStmt::Assign {
                target: AssignTarget::Symbol(name),
                value,
                ..
            } => {
                if !mutated_vars.contains(name)
                    && !used_vars.contains(name)
                    && self.is_constant_expr(value)
                {
                    constants.insert(name.clone(), value.clone());
                }
            }
            HirStmt::Assign { .. } => {}
            HirStmt::If {
                then_body,
                else_body,
                ..
            } => {
                for s in then_body {
                    self.collect_constants_stmt(s, constants, mutated_vars, used_vars);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.collect_constants_stmt(s, constants, mutated_vars, used_vars);
                    }
                }
            }
            HirStmt::While { body, .. } | HirStmt::For { body, .. } => {
                for s in body {
                    self.collect_constants_stmt(s, constants, mutated_vars, used_vars);
                }
            }
            _ => {}
        }
    }

    fn is_constant_expr(&self, expr: &HirExpr) -> bool {
        super::cse::is_constant_expr_inner(expr)
    }

    fn propagate_constants_function(
        &self,
        func: &mut HirFunction,
        constants: &HashMap<String, HirExpr>,
    ) {
        for stmt in &mut func.body {
            self.propagate_constants_stmt(stmt, constants);
        }
    }

    fn propagate_constants_stmt(&self, stmt: &mut HirStmt, constants: &HashMap<String, HirExpr>) {
        match stmt {
            HirStmt::Assign { value, .. } => {
                self.propagate_constants_expr(value, constants);
            }
            HirStmt::Return(Some(expr)) => {
                self.propagate_constants_expr(expr, constants);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.propagate_constants_expr(condition, constants);
                for s in then_body {
                    self.propagate_constants_stmt(s, constants);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.propagate_constants_stmt(s, constants);
                    }
                }
            }
            HirStmt::While { condition, body } => {
                self.propagate_constants_expr(condition, constants);
                for s in body {
                    self.propagate_constants_stmt(s, constants);
                }
            }
            HirStmt::For { iter, body, .. } => {
                self.propagate_constants_expr(iter, constants);
                for s in body {
                    self.propagate_constants_stmt(s, constants);
                }
            }
            HirStmt::Expr(expr) => {
                self.propagate_constants_expr(expr, constants);
            }
            _ => {}
        }
    }

    fn propagate_constants_expr(&self, expr: &mut HirExpr, constants: &HashMap<String, HirExpr>) {
        match expr {
            HirExpr::Var(name) => {
                if let Some(const_expr) = constants.get(name) {
                    *expr = const_expr.clone();
                }
            }
            HirExpr::Binary { left, right, .. } => {
                self.propagate_constants_expr(left, constants);
                self.propagate_constants_expr(right, constants);

                if let Some(result) = self.evaluate_constant_binop(expr) {
                    *expr = result;
                }
            }
            HirExpr::Unary { operand, .. } => {
                self.propagate_constants_expr(operand, constants);

                if let Some(result) = self.evaluate_constant_unaryop(expr) {
                    *expr = result;
                }
            }
            HirExpr::List(items) => {
                for item in items {
                    self.propagate_constants_expr(item, constants);
                }
            }
            HirExpr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.propagate_constants_expr(k, constants);
                    self.propagate_constants_expr(v, constants);
                }
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    self.propagate_constants_expr(arg, constants);
                }
            }
            HirExpr::MethodCall { object, args, .. } => {
                self.propagate_constants_expr(object, constants);
                for arg in args {
                    self.propagate_constants_expr(arg, constants);
                }
            }
            HirExpr::Lambda { body, .. } => {
                self.propagate_constants_expr(body, constants);
            }
            _ => {}
        }
    }

    fn evaluate_constant_binop(&self, expr: &HirExpr) -> Option<HirExpr> {
        if let HirExpr::Binary { left, right, op } = expr {
            match (left.as_ref(), right.as_ref(), op) {
                (
                    HirExpr::Literal(Literal::Int(a)),
                    HirExpr::Literal(Literal::Int(b)),
                    BinOp::Add,
                ) => Some(HirExpr::Literal(Literal::Int(a + b))),
                (
                    HirExpr::Literal(Literal::Int(a)),
                    HirExpr::Literal(Literal::Int(b)),
                    BinOp::Sub,
                ) => Some(HirExpr::Literal(Literal::Int(a - b))),
                (
                    HirExpr::Literal(Literal::Int(a)),
                    HirExpr::Literal(Literal::Int(b)),
                    BinOp::Mul,
                ) => Some(HirExpr::Literal(Literal::Int(a * b))),
                (
                    HirExpr::Literal(Literal::Int(a)),
                    HirExpr::Literal(Literal::Int(b)),
                    BinOp::Div,
                ) if *b != 0 => Some(HirExpr::Literal(Literal::Int(a / b))),
                (
                    HirExpr::Literal(Literal::Float(a)),
                    HirExpr::Literal(Literal::Float(b)),
                    BinOp::Add,
                ) => Some(HirExpr::Literal(Literal::Float(a + b))),
                (
                    HirExpr::Literal(Literal::Float(a)),
                    HirExpr::Literal(Literal::Float(b)),
                    BinOp::Sub,
                ) => Some(HirExpr::Literal(Literal::Float(a - b))),
                (
                    HirExpr::Literal(Literal::Float(a)),
                    HirExpr::Literal(Literal::Float(b)),
                    BinOp::Mul,
                ) => Some(HirExpr::Literal(Literal::Float(a * b))),
                (
                    HirExpr::Literal(Literal::Float(a)),
                    HirExpr::Literal(Literal::Float(b)),
                    BinOp::Div,
                ) if *b != 0.0 => Some(HirExpr::Literal(Literal::Float(a / b))),
                _ => None,
            }
        } else {
            None
        }
    }

    fn evaluate_constant_unaryop(&self, expr: &HirExpr) -> Option<HirExpr> {
        if let HirExpr::Unary { op, operand } = expr {
            match (operand.as_ref(), op) {
                (HirExpr::Literal(Literal::Int(n)), UnaryOp::Neg) => {
                    Some(HirExpr::Literal(Literal::Int(-n)))
                }
                (HirExpr::Literal(Literal::Float(f)), UnaryOp::Neg) => {
                    Some(HirExpr::Literal(Literal::Float(-f)))
                }
                (HirExpr::Literal(Literal::Bool(b)), UnaryOp::Not) => {
                    Some(HirExpr::Literal(Literal::Bool(!b)))
                }
                _ => None,
            }
        } else {
            None
        }
    }

}
